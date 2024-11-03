use std::{
    alloc::Layout,
    any::{
        type_name,
        TypeId,
    },
    mem::needs_drop,
};

use crate::{
    storage::StorageType,
    util::{
        drop_ptr,
        sparse_map::SparseMapKey,
        type_id_map::{
            self,
            TypeIdMap,
        },
        DropFn,
    },
};

pub trait Component: 'static {
    const STORAGE_TYPE: StorageType;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ComponentId(usize);

impl SparseMapKey for ComponentId {
    fn index(&self) -> usize {
        self.0
    }

    fn from_index(index: usize) -> Self {
        Self(index)
    }
}

#[derive(Clone, Debug)]
pub struct ComponentDescriptor {
    name: &'static str,
    type_id: TypeId,
    layout: Layout,
    drop_fn: Option<DropFn>,
}

impl ComponentDescriptor {
    pub fn new<C: Component>() -> Self {
        Self {
            name: type_name::<C>(),
            type_id: TypeId::of::<C>(),
            layout: Layout::new::<C>(),
            drop_fn: needs_drop::<C>().then_some(drop_ptr::<C>),
        }
    }

    pub fn layout(&self) -> Layout {
        self.layout
    }

    pub fn drop_fn(&self) -> Option<DropFn> {
        self.drop_fn
    }

    pub fn name(&self) -> &'static str {
        self.name
    }
}

#[derive(Clone, Debug)]
pub struct ComponentInfo {
    id: ComponentId,
    storage_type: StorageType,
    descriptor: ComponentDescriptor,
}

impl ComponentInfo {
    pub fn id(&self) -> ComponentId {
        self.id
    }

    pub fn descriptor(&self) -> &ComponentDescriptor {
        &self.descriptor
    }

    pub fn storage_type(&self) -> StorageType {
        self.storage_type
    }
}

#[derive(Clone, Debug, Default)]
pub struct Components {
    components: Vec<ComponentInfo>,
    by_type: TypeIdMap<ComponentId>,
}

impl Components {
    pub fn register<C: Component>(&mut self) -> &mut ComponentInfo {
        let index = match self.by_type.entry::<C>() {
            type_id_map::Entry::Occupied(occupied_entry) => occupied_entry.get().index(),
            type_id_map::Entry::Vacant(vacant_entry) => {
                let index = self.components.len();
                let id = ComponentId(index);
                self.components.push(ComponentInfo {
                    id,
                    storage_type: C::STORAGE_TYPE,
                    descriptor: ComponentDescriptor::new::<C>(),
                });
                vacant_entry.insert(id);
                index
            }
        };

        &mut self.components[index]
    }

    pub fn get_component_info(&self, component_id: ComponentId) -> &ComponentInfo {
        &self.components[component_id.index()]
    }

    pub fn get_component_id<C: Component>(&self) -> Option<ComponentId> {
        self.by_type.get::<C>().copied()
    }
}
