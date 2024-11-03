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
    storage::table::{
        InsertIntoTable,
        Table,
        TableRow,
    },
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
pub unsafe trait DynamicBundle: 'static {
    fn num_components(&self) -> usize;
    fn component_types<F: ComponentTypesCallback>(&self, callback: F);
    fn into_components<F: IntoComponentsCallback>(self, callback: F);
}

pub unsafe trait Bundle: 'static {
    const NUM_COMPONENTS: usize;

    fn component_types<F: ComponentTypesCallback>(callback: F);
    fn from_components<F: FromComponentsCallback>(callback: F) -> Self;
    fn into_components<F: IntoComponentsCallback>(self, callback: F);
}

unsafe impl<B: Bundle> DynamicBundle for B {
    fn num_components(&self) -> usize {
        <B as Bundle>::NUM_COMPONENTS
    }

    fn component_types<F: ComponentTypesCallback>(&self, callback: F) {
        <B as Bundle>::component_types(callback)
    }

    fn into_components<F: IntoComponentsCallback>(self, callback: F) {
        <B as Bundle>::into_components(self, callback)
    }
}

unsafe impl<T: Component> Bundle for T {
    const NUM_COMPONENTS: usize = 1;

    fn component_types<F: ComponentTypesCallback>(mut callback: F) {
        callback.call::<T>();
    }

    fn into_components<F: IntoComponentsCallback>(self, mut callback: F) {
        callback.call(self)
    }

    fn from_components<F: FromComponentsCallback>(mut callback: F) -> Self {
        callback.call()
    }
}

unsafe impl Bundle for () {
    const NUM_COMPONENTS: usize = 0;

    fn component_types<F: ComponentTypesCallback>(_callback: F) {}

    fn into_components<F: IntoComponentsCallback>(self, _callback: F) {}

    fn from_components<F: FromComponentsCallback>(_callback: F) -> Self {
        ()
    }
}

pub trait ComponentTypesCallback {
    fn call<C: Component>(&mut self);
}

pub trait IntoComponentsCallback {
    fn call<C: Component>(&mut self, component: C);
}

pub trait FromComponentsCallback {
    fn call<C: Component>(&mut self) -> C;
}

impl<'a, T: ComponentTypesCallback> ComponentTypesCallback for &'a mut T {
    fn call<C: Component>(&mut self) {
        <T as ComponentTypesCallback>::call::<C>(*self);
    }
}

impl<'a, T: IntoComponentsCallback> IntoComponentsCallback for &'a mut T {
    fn call<C: Component>(&mut self, component: C) {
        <T as IntoComponentsCallback>::call::<C>(*self, component);
    }
}

#[derive(Debug)]
pub struct RegisterComponents<'a, F> {
    components: &'a mut Components,
    callback: F,
}

impl<'a, F> RegisterComponents<'a, F> {
    pub fn new(components: &'a mut Components, callback: F) -> Self {
        Self {
            components,
            callback,
        }
    }
}

impl<'a, F> ComponentTypesCallback for RegisterComponents<'a, F>
where
    F: FnMut(&ComponentInfo),
{
    fn call<C: Component>(&mut self) {
        let component_info = self.components.register::<C>();
        (self.callback)(component_info);
    }
}

#[derive(Debug)]
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

impl<'a, 't, F> IntoComponentsCallback for InsertComponentsIntoTable<'a, 't, F>
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
pub struct TakeComponentsFromTable<'a, 't> {
    component_ids: std::slice::Iter<'a, ComponentId>,
    table: &'t mut Table,
    table_row: TableRow,
}

impl<'a, 't> TakeComponentsFromTable<'a, 't> {
    pub fn new(bundle_info: &'a BundleInfo, table: &'t mut Table, table_row: TableRow) -> Self {
        Self {
            component_ids: bundle_info.component_ids().iter(),
            table,
            table_row,
        }
    }
}

impl<'a, 't> FromComponentsCallback for TakeComponentsFromTable<'a, 't> {
    fn call<C: Component>(&mut self) -> C {
        let component_id = self
            .component_ids
            .next()
            .expect("not enough component ids from bundle info");

        unsafe {
            self.table
                .take_component_and_remove_later::<C>(*component_id, self.table_row)
                .unwrap()
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
    pub fn get_mut_or_insert_static<B: Bundle>(
        &mut self,
        components: &mut Components,
    ) -> &mut BundleInfo {
        self.get_mut_or_insert_inner::<B>(
            |buf, components| {
                buf.reserve(B::NUM_COMPONENTS);
                B::component_types(RegisterComponents::new(
                    components,
                    |component_info: &ComponentInfo| {
                        buf.push(component_info.id());
                    },
                ));
            },
            components,
        )
    }

    pub fn get_mut_or_insert_dynamic<B: DynamicBundle>(
        &mut self,
        bundle: &B,
        components: &mut Components,
    ) -> &mut BundleInfo {
        self.get_mut_or_insert_inner::<B>(
            |buf, components| {
                buf.reserve(bundle.num_components());
                bundle.component_types(RegisterComponents::new(
                    components,
                    |component_info: &ComponentInfo| {
                        buf.push(component_info.id());
                    },
                ));
            },
            components,
        )
    }

    fn get_mut_or_insert_inner<B: 'static>(
        &mut self,
        component_types: impl FnOnce(&mut Vec<ComponentId>, &mut Components),
        components: &mut Components,
    ) -> &mut BundleInfo {
        let occupied_entry = self.by_type_id.entry::<B>().or_insert_with(|| {
            let index = self.bundle_infos.len();
            let id = BundleId::from_index(index);
            let name = type_name::<B>();

            self.insert_component_ids_buf.clear(); // note: in case we panicked before draining this

            component_types(&mut self.insert_component_ids_buf, components);

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

    pub fn get<B: DynamicBundle>(&self) -> Option<&BundleInfo> {
        let index = self.by_type_id.get::<B>()?;
        Some(&self.bundle_infos[index.index()])
    }
}
