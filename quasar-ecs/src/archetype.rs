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
        Components,
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

    fn get_or_insert_archetype_by_components(
        &mut self,
        component_ids: Box<[ComponentId]>,
        create_archetype: impl FnOnce(ArchetypeId, &[ComponentId]) -> Archetype,
    ) -> ArchetypeId {
        let reserved_archetype_id = ArchetypeId::from_index(self.archetypes.len());

        self.by_components
            .get(&component_ids)
            .copied()
            .unwrap_or_else(|| {
                // the resulting archetype doesn't exist, so we need to create it.
                let archetype = create_archetype(reserved_archetype_id, &component_ids);
                self.archetypes.push(archetype);

                for component_id in &component_ids {
                    // add new archetype to by_component map
                    self.by_component
                        .entry(*component_id)
                        .or_default()
                        .push(reserved_archetype_id);
                }

                // add new archetype to by_components map
                self.by_components
                    .insert(component_ids, reserved_archetype_id);

                reserved_archetype_id
            })
    }

    pub fn add_bundle<'i, 'b>(
        &mut self,
        archetype_id: ArchetypeId,
        bundle_info: &BundleInfo,
        create_archetype: impl FnOnce(ArchetypeId, &[ComponentId]) -> Archetype,
    ) -> Option<(&mut Archetype, &mut Archetype)> {
        if bundle_info.is_empty() {
            // inserting an empty bundle doesn't do anything
            return None;
        }

        // the archetype to which we're adding a bundle. this should exist.
        let from_archetype_index = archetype_id.index();
        let from_archetype = &mut self.archetypes[from_archetype_index];

        // get the archetype id for the resulting archetype. creating the edge in the
        // process
        let to_archetype_id =
            if let Some(add_bundle) = from_archetype.edges.add_bundle.get(&bundle_info.id()) {
                // an edge already exists
                add_bundle.archetype_id
            }
            else {
                // an edge didn't exist, so we need to create it.

                // the components that are already existing in `from_archetype`.
                let existing = &from_archetype.components;

                // stores any components that are added by the bundle, but already exist in
                // `from_archetype`.
                let mut duplicate = SparseSet::with_capacity(existing.len());

                // compute the component ids for the resulting archetype
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

                // even if the edge didn't exist, the resulting archetype might already exist.
                let to_archetype_id =
                    self.get_or_insert_archetype_by_components(component_ids, create_archetype);

                self.archetypes[from_archetype_index]
                    .edges
                    .add_bundle
                    .insert(
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

    pub fn remove_bundle<'i, 'b>(
        &mut self,
        archetype_id: ArchetypeId,
        bundle_info: &BundleInfo,
        create_archetype: impl FnOnce(ArchetypeId, &[ComponentId]) -> Archetype,
    ) -> Option<(&mut Archetype, &mut Archetype)> {
        if bundle_info.is_empty() {
            return None;
        }

        // the archetype to which we're adding a bundle. this should exist.
        let from_archetype_index = archetype_id.index();
        let from_archetype = &mut self.archetypes[from_archetype_index];

        // get the archetype id for the resulting archetype. creating the edge in the
        // process
        let to_archetype_id = if let Some(remove_bundle) =
            from_archetype.edges.remove_bundle.get(&bundle_info.id())
        {
            // an edge already exists
            remove_bundle.archetype_id()
        }
        else {
            // an edge didn't exist, so we need to create it.

            // the components that need to be removed
            let remove_components = bundle_info
                .component_ids()
                .iter()
                .copied()
                .collect::<ImmutableSparseSet<_>>();

            // the components that are kept
            let component_ids = from_archetype
                .components
                .keys()
                .filter(|component_id| !remove_components.contains(component_id))
                .collect::<Box<[ComponentId]>>();

            let (to_archetype_id, edge) = if remove_components.len() + component_ids.len()
                < from_archetype.components.len()
            {
                // some components from the bundle are not in the archetype
                //let missing = bundle_info.component_ids().iter().copied()
                //    .filter(|component_id| from_archetype.contains_component(*component_id))
                //    .collect::<Vec<_>>();
                (None, RemoveBundle::Mismatch)
            }
            else {
                // even if the edge didn't exist, the resulting archetype might already exist.
                let to_archetype_id =
                    self.get_or_insert_archetype_by_components(component_ids, create_archetype);

                (
                    Some(to_archetype_id),
                    RemoveBundle::Match {
                        archetype_id: to_archetype_id,
                    },
                )
            };

            self.archetypes[from_archetype_index]
                .edges
                .remove_bundle
                .insert(&bundle_info.id(), edge);

            to_archetype_id
        };

        to_archetype_id.and_then(|to_archetype_id| {
            slice_get_mut_pair(
                &mut self.archetypes,
                from_archetype_index,
                to_archetype_id.index(),
            )
            .ok()
        })
    }
}

pub fn create_archetype(
    archetype_id: ArchetypeId,
    component_ids: &[ComponentId],
    components: &Components,
    tables: &mut Tables,
) -> Archetype {
    enum Table {
        Existing(TableId),
        New(TableBuilder),
    }

    impl Table {
        fn new(tables: &mut Tables, component_ids: &[ComponentId]) -> Self {
            if let Some(table_id) = tables.get_table_id_by_component_ids(component_ids) {
                Table::Existing(table_id)
            }
            else {
                Table::New(TableBuilder::new(1, component_ids.len()))
            }
        }

        fn add_component(&mut self, component_info: &ComponentInfo) {
            match self {
                Table::Existing(_table_id) => {}
                Table::New(table_builder) => table_builder.add_column(component_info),
            }
        }

        fn finish(self, tables: &mut Tables) -> TableId {
            match self {
                Table::Existing(table_id) => table_id,
                Table::New(table_builder) => tables.insert(table_builder.build()),
            }
        }
    }

    let mut table = Table::new(tables, component_ids);
    let mut archetype_component_infos = SparseMap::with_capacity(component_ids.len());

    for component_id in component_ids {
        let component_info = components.get_component_info(*component_id);

        archetype_component_infos
            .insert(component_id, ArchetypeComponentInfo::from(component_info));

        table.add_component(component_info);
    }

    let table_id = table.finish(tables);

    Archetype {
        id: archetype_id,
        table_id,
        entities: Vec::with_capacity(1),
        components: archetype_component_infos.into(),
        edges: Edges::default(),
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
pub enum RemoveBundle {
    Match { archetype_id: ArchetypeId },
    Mismatch,
}

impl RemoveBundle {
    pub fn archetype_id(&self) -> Option<ArchetypeId> {
        match self {
            RemoveBundle::Match { archetype_id } => Some(*archetype_id),
            RemoveBundle::Mismatch => None,
        }
    }
}
