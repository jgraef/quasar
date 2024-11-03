use std::collections::HashMap;

use crate::{
    archetype,
    bundle::{
        BundleId,
        BundleInfo,
    },
    component::{
        ComponentId,
        ComponentInfo,
    },
    entity::{
        ChangedLocation,
        Entity,
    },
    storage::{
        table::{
            Table,
            TableBuilder,
            TableId,
            TableRow,
            Tables,
        },
        StorageType,
    },
    util::{
        slice_get_mut_pair,
        sparse_map::{
            self,
            ImmutableSparseMap,
            SparseMap,
        },
        sparse_set::{
            ImmutableSparseSet,
            SparseSet,
        },
    },
};

#[derive(Debug)]
pub struct Archetype {
    id: ArchetypeId,
    table_id: TableId,
    entities: Vec<ArchetypeEntity>,
    components: ImmutableSparseMap<ComponentId, ArchetypeComponentInfo>,
    edges: Edges,
}

impl Archetype {
    pub fn insert_entity(&mut self, archetype_entity: ArchetypeEntity) -> ArchetypeRow {
        let index = self.entities.len();
        self.entities.push(archetype_entity);
        ArchetypeRow::from_index(index)
    }

    pub fn remove_entity(
        &mut self,
        archetype_row: ArchetypeRow,
    ) -> Option<ChangedLocation<ArchetypeRow>> {
        if archetype_row.is_invalid() {
            None
        }
        else {
            let index = archetype_row.index();
            let swapped = index != self.entities.len() - 1;
            let _removed_entity = self.entities.swap_remove(index);
            swapped.then(|| {
                ChangedLocation {
                    entity: self.entities[index].entity,
                    changed_value: archetype_row,
                }
            })
        }
    }

    pub fn id(&self) -> ArchetypeId {
        self.id
    }

    pub fn table_id(&self) -> TableId {
        self.table_id
    }

    pub fn contains_component(&self, component_id: ComponentId) -> bool {
        self.components.contains_key(&component_id)
    }

    pub fn add_bundle(&self, bundle_id: BundleId) -> Option<&AddBundle> {
        self.edges.add_bundle.get(&bundle_id)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct ArchetypeId(u32);

impl ArchetypeId {
    pub const EMPTY: Self = Self(0);
    pub const INVALID: Self = Self(u32::MAX);

    fn index(&self) -> usize {
        self.0 as usize
    }

    fn from_index(index: usize) -> Self {
        Self(index.try_into().expect("ArchetypeId overflow"))
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct ArchetypeRow(u32);

impl ArchetypeRow {
    pub const INVALID: Self = Self(u32::MAX);

    fn index(&self) -> usize {
        self.0 as usize
    }

    fn from_index(index: usize) -> Self {
        Self(index.try_into().expect("ArchetypeRow overflow"))
    }

    pub fn is_invalid(&self) -> bool {
        *self == Self::INVALID
    }

    pub fn is_valid(&self) -> bool {
        !self.is_invalid()
    }
}

#[derive(Clone, Copy, Debug)]
pub struct ArchetypeEntity {
    pub entity: Entity,
    pub table_row: TableRow,
}

#[derive(Clone, Copy, Debug)]
pub struct ArchetypeComponentInfo {
    storage_type: StorageType,
    component_id: ComponentId,
}

impl<'a> From<&'a ComponentInfo> for ArchetypeComponentInfo {
    fn from(value: &'a ComponentInfo) -> Self {
        Self {
            storage_type: value.storage_type(),
            component_id: value.id(),
        }
    }
}

#[derive(Debug)]
pub struct Archetypes {
    archetypes: Vec<Archetype>,
    by_components: HashMap<Box<[ComponentId]>, ArchetypeId>,
    by_component: HashMap<ComponentId, Vec<ArchetypeId>>,
}

impl Default for Archetypes {
    fn default() -> Self {
        Self {
            archetypes: vec![Archetype {
                id: ArchetypeId::EMPTY,
                table_id: TableId::EMPTY,
                entities: vec![],
                components: ImmutableSparseMap::default(),
                edges: Edges::default(),
            }],
            by_components: {
                let mut hash_map = HashMap::with_capacity(1);
                hash_map.insert(std::iter::empty().collect(), ArchetypeId::EMPTY);
                hash_map
            },
            by_component: HashMap::new(),
        }
    }
}

impl Archetypes {
    pub fn get(&self, archetype_id: ArchetypeId) -> &Archetype {
        &self.archetypes[archetype_id.index()]
    }

    pub fn get_mut(&mut self, archetype_id: ArchetypeId) -> &mut Archetype {
        &mut self.archetypes[archetype_id.index()]
    }

    pub fn iter(&self) -> ArchetypesIter {
        ArchetypesIter {
            iter: self.archetypes.iter(),
        }
    }

    pub fn add_bundle<'i, 'b>(
        &mut self,
        archetype_id: ArchetypeId,
        bundle_info: &BundleInfo,
        get_component_info: impl Fn(ComponentId) -> &'i ComponentInfo,
        insert_table: impl FnOnce(Table) -> TableId,
    ) -> Option<(&mut Archetype, &mut Archetype)> {
        if bundle_info.is_empty() {
            return None;
        }

        let from_archetype_index = archetype_id.index();
        let reserved_archetype_id = ArchetypeId::from_index(self.archetypes.len());

        let from_archetype = &mut self.archetypes[from_archetype_index];

        let to_archetype_id =
            if let Some(add_bundle) = from_archetype.edges.add_bundle.get(&bundle_info.id()) {
                add_bundle.archetype_id
            }
            else {
                let existing = &from_archetype.components;
                let mut duplicate = SparseSet::with_capacity(existing.len());

                let mut component_ids =
                    Vec::with_capacity(existing.len() + bundle_info.component_ids().len());
                component_ids.extend(existing.keys());
                for component_id in bundle_info.component_ids() {
                    if existing.contains_key(component_id) {
                        duplicate.insert(component_id);
                    }
                    else {
                        component_ids.push(*component_id);
                    }
                }
                component_ids.sort_unstable();
                let component_ids: Box<[ComponentId]> = component_ids.into();

                let to_archetype_id = self
                    .by_components
                    .get(&component_ids)
                    .copied()
                    .unwrap_or_else(|| {
                        let mut table_builder = TableBuilder::new(1, component_ids.len());
                        let mut archetype_component_infos =
                            SparseMap::with_capacity(component_ids.len());

                        for component_id in &component_ids {
                            let component_info = get_component_info(*component_id);
                            table_builder.add_column(component_info);
                            archetype_component_infos
                                .insert(component_id, ArchetypeComponentInfo::from(component_info));
                        }

                        let table_id = insert_table(table_builder.build());

                        for component_id in &component_ids {
                            self.by_component
                                .entry(*component_id)
                                .or_default()
                                .push(reserved_archetype_id);
                        }
                        self.by_components
                            .insert(component_ids, reserved_archetype_id);
                        self.archetypes.push(Archetype {
                            id: reserved_archetype_id,
                            table_id,
                            entities: Vec::with_capacity(1),
                            components: archetype_component_infos.into(),
                            edges: Edges::default(),
                        });

                        reserved_archetype_id
                    });

                let source = &mut self.archetypes[from_archetype_index];
                source.edges.add_bundle.insert(
                    &bundle_info.id(),
                    AddBundle {
                        archetype_id: to_archetype_id,
                        duplicate: duplicate.into(),
                    },
                );

                to_archetype_id
            };

        slice_get_mut_pair(
            &mut self.archetypes,
            from_archetype_index,
            to_archetype_id.index(),
        )
        .ok()
    }
}

pub struct ArchetypesIter<'a> {
    iter: std::slice::Iter<'a, Archetype>,
}

impl<'a> Iterator for ArchetypesIter<'a> {
    type Item = &'a Archetype;

    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next()
    }
}

#[derive(Debug, Default)]
struct Edges {
    pub add_bundle: SparseMap<BundleId, AddBundle>,
    pub remove_bundle: SparseMap<BundleId, RemoveBundle>,
}

#[derive(Debug)]
pub struct AddBundle {
    pub archetype_id: ArchetypeId,
    pub duplicate: ImmutableSparseSet<ComponentId>,
}

#[derive(Debug)]
pub struct RemoveBundle {
    pub output_archetype_id: ArchetypeId,
}
