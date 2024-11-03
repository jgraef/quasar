use std::{
    any::type_name,
    collections::HashSet,
};

use crate::{
    component::{
        Component,
        ComponentId,
        ComponentInfo,
        Components,
    },
    storage::table::InsertIntoTable,
    util::{
        partition_dedup,
        sparse_map::SparseMapKey,
        type_id_map::TypeIdMap,
        Joined,
    },
};

/// # Safety
///
/// This trait is not safe to implement, since the following invariants must be
/// upheld:
///
/// - [`for_each_component`] and [`into_each_component`] always call the
///   callback with the same component types in the same order.
pub unsafe trait Bundle: 'static {
    fn num_components(&self) -> usize;
    fn for_each_component<C: ForEachComponent>(&self, callback: C);
    fn into_each_component<C: IntoEachComponent>(self, callback: C);
}

unsafe impl<T: Component> Bundle for T {
    fn num_components(&self) -> usize {
        1
    }

    fn for_each_component<C: ForEachComponent>(&self, mut callback: C) {
        callback.call(self);
    }

    fn into_each_component<C: IntoEachComponent>(self, mut callback: C) {
        callback.call(self)
    }
}

unsafe impl Bundle for () {
    fn num_components(&self) -> usize {
        0
    }

    fn for_each_component<C: ForEachComponent>(&self, _callback: C) {}

    fn into_each_component<C: IntoEachComponent>(self, _callback: C) {}
}

pub trait ForEachComponent {
    fn call<C: Component>(&mut self, component: &C);
}

pub trait IntoEachComponent {
    fn call<C: Component>(&mut self, component: C);
}

impl<'a, T: ForEachComponent> ForEachComponent for &'a mut T {
    fn call<C: Component>(&mut self, component: &C) {
        <T as ForEachComponent>::call::<C>(*self, component);
    }
}

impl<'a, T: IntoEachComponent> IntoEachComponent for &'a mut T {
    fn call<C: Component>(&mut self, component: C) {
        <T as IntoEachComponent>::call::<C>(*self, component);
    }
}

pub struct WithComponentInfo<'a, F> {
    components: &'a mut Components,
    callback: F,
}

impl<'a, F> WithComponentInfo<'a, F> {
    pub fn new(components: &'a mut Components, callback: F) -> Self {
        Self {
            components,
            callback,
        }
    }
}

impl<'a, F> ForEachComponent for WithComponentInfo<'a, F>
where
    F: FnMut(&ComponentInfo),
{
    fn call<C: Component>(&mut self, _component: &C) {
        let component_info = self.components.register::<C>();
        (self.callback)(component_info);
    }
}

pub struct InsertComponentsIntoTable<'a, 't, F> {
    component_ids: std::slice::Iter<'a, ComponentId>,
    filter: F,
    insert_into_table: &'a mut InsertIntoTable<'t>,
}

impl<'a, 't, F> InsertComponentsIntoTable<'a, 't, F> {
    pub fn new(
        bundle_info: &'a BundleInfo,
        filter: F,
        insert_into_table: &'a mut InsertIntoTable<'t>,
    ) -> Self {
        Self {
            component_ids: bundle_info.component_ids().iter(),
            filter,
            insert_into_table,
        }
    }
}

impl<'a, 't, F> IntoEachComponent for InsertComponentsIntoTable<'a, 't, F>
where
    F: Fn(ComponentId) -> bool,
{
    fn call<C: Component>(&mut self, component: C) {
        let component_id = self
            .component_ids
            .next()
            .expect("not enough component ids from bundle info");

        if (self.filter)(*component_id) {
            unsafe {
                // SAFETY:
                // The implementor of the Bundle trait must ensure that they only call this
                // callback with components of the correct type.
                self.insert_into_table
                    .write_column(*component_id, component);
            }
        }
    }
}

#[derive(Debug)]
pub struct BundleInfo {
    id: BundleId,
    name: &'static str,
    component_ids: Box<[ComponentId]>,
}

impl BundleInfo {
    pub fn id(&self) -> BundleId {
        self.id
    }

    pub fn name(&self) -> &'static str {
        self.name
    }

    pub fn component_ids(&self) -> &[ComponentId] {
        &self.component_ids
    }

    pub fn is_empty(&self) -> bool {
        self.component_ids.is_empty()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct BundleId(u32);

impl SparseMapKey for BundleId {
    fn index(&self) -> usize {
        self.0 as usize
    }

    fn from_index(index: usize) -> Self {
        Self(index.try_into().expect("BundleId overflow"))
    }
}

#[derive(Debug, Default)]
pub struct Bundles {
    bundle_infos: Vec<BundleInfo>,
    by_type_id: TypeIdMap<BundleId>,
    insert_component_ids_buf: Vec<ComponentId>,
}

impl Bundles {
    pub fn get_mut_or_insert<B: Bundle>(
        &mut self,
        bundle: &B,
        components: &mut Components,
    ) -> &mut BundleInfo {
        let occupied_entry = self.by_type_id.entry::<B>().or_insert_with(|| {
            let index = self.bundle_infos.len();
            let id = BundleId::from_index(index);
            let name = type_name::<B>();

            self.insert_component_ids_buf.clear(); // note: in case we panicked before draining this
            self.insert_component_ids_buf
                .reserve(bundle.num_components());
            bundle.for_each_component(WithComponentInfo::new(
                components,
                |component_info: &ComponentInfo| {
                    self.insert_component_ids_buf.push(component_info.id());
                },
            ));

            self.insert_component_ids_buf.sort_unstable();
            let (_, duplicates) = partition_dedup(&mut self.insert_component_ids_buf);
            if !duplicates.is_empty() {
                let duplicates = duplicates.iter().copied().collect::<HashSet<_>>();
                let names = duplicates
                    .into_iter()
                    .map(|component_id| {
                        components
                            .get_component_info(component_id)
                            .descriptor()
                            .name()
                    })
                    .collect::<Vec<_>>();
                panic!(
                    "Bundle {name} contains duplicate components: {}",
                    Joined::new(", ", &names)
                );
            }

            self.bundle_infos.push(BundleInfo {
                id,
                name: type_name::<B>(),
                component_ids: self.insert_component_ids_buf.drain(..).collect(),
            });

            id
        });

        &mut self.bundle_infos[occupied_entry.get().index()]
    }

    pub fn get<B: Bundle>(&self) -> Option<&BundleInfo> {
        let index = self.by_type_id.get::<B>()?;
        Some(&self.bundle_infos[index.index()])
    }
}
