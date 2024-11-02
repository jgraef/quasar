use std::{
    num::NonZeroUsize,
    sync::atomic::{
        AtomicUsize,
        Ordering,
    },
};

use crate::{
    archetype::{
        ArchetypeEntity,
        Archetypes,
    },
    bundle::{
        Bundle,
        Bundles,
        InsertComponentsIntoTable,
    },
    component::{
        Component,
        Components,
    },
    entity::{
        Entities,
        EntitiesIter,
        Entity,
        EntityLocation,
    },
    resources::Resources,
    storage::{
        table::{
            TableBuilder,
            Tables,
        },
        StorageType,
    },
};

#[derive(Debug)]
pub struct World {
    id: WorldId,
    entities: Entities,
    components: Components,
    archetypes: Archetypes,
    tables: Tables,
    bundles: Bundles,
    resources: Resources,
}

impl Default for World {
    fn default() -> Self {
        Self::new()
    }
}

impl World {
    pub fn new() -> Self {
        Self {
            id: WorldId::new(),
            entities: Entities::default(),
            components: Default::default(),
            archetypes: Default::default(),
            tables: Tables::default(),
            bundles: Bundles::default(),
            resources: Resources::default(),
        }
    }

    pub fn id(&self) -> WorldId {
        self.id
    }

    pub fn clear_entities(&mut self) {
        self.entities.clear();
        self.tables.clear();
    }

    pub fn clear_resources(&mut self) {
        self.resources.clear();
    }

    pub fn clear_all(&mut self) {
        self.clear_entities();
        self.clear_resources();
    }

    pub fn spawn_empty(&mut self) -> EntityWorldMut {
        let entity = self.entities.allocate();
        EntityWorldMut {
            world: self,
            entity,
            entity_location: EntityLocation::INVALID,
        }
    }

    pub fn spawn(&mut self, bundle: impl Bundle) -> EntityWorldMut {
        let mut entity = self.spawn_empty();
        entity.insert(bundle);
        entity
    }

    pub fn get_entity(&self, entity: Entity) -> Option<EntityRef> {
        let entity_location = self.entities.get_location(entity)?;
        Some(EntityRef {
            components: &self.components,
            archetypes: &self.archetypes,
            tables: &self.tables,
            entity,
            entity_location,
        })
    }

    pub fn get_entity_mut(&mut self, entity: Entity) -> Option<EntityMut> {
        let entity_location = self.entities.get_location(entity)?;
        Some(EntityMut {
            components: &self.components,
            archetypes: &self.archetypes,
            tables: &mut self.tables,
            entity,
            entity_location,
        })
    }

    pub fn iter_entities(&self) -> EntityIter {
        EntityIter {
            components: &self.components,
            archetypes: &self.archetypes,
            tables: &self.tables,
            iter: self.entities.iter(),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct WorldId(NonZeroUsize);

impl WorldId {
    fn new() -> Self {
        static IDS: AtomicUsize = AtomicUsize::new(1);
        Self(NonZeroUsize::new(IDS.fetch_add(1, Ordering::Relaxed)).expect("WorldId overflow"))
    }
}

pub struct EntityRef<'world> {
    components: &'world Components,
    archetypes: &'world Archetypes,
    tables: &'world Tables,
    entity: Entity,
    entity_location: EntityLocation,
}

impl<'a> EntityRef<'a> {
    pub fn id(&self) -> Entity {
        self.entity
    }

    pub fn contains<C: Component>(&self) -> bool {
        contains_component::<C>(self.entity_location, self.components, self.archetypes)
    }

    pub fn get<C: Component>(&self) -> Option<&C> {
        get_component(self.entity_location, self.components, self.tables)
    }
}

#[derive(Debug)]
pub struct EntityMut<'world> {
    components: &'world Components,
    archetypes: &'world Archetypes,
    tables: &'world mut Tables,
    entity: Entity,
    entity_location: EntityLocation,
}

impl<'a> EntityMut<'a> {
    pub fn id(&self) -> Entity {
        self.entity
    }

    pub fn contains<C: Component>(&self) -> bool {
        contains_component::<C>(self.entity_location, self.components, self.archetypes)
    }

    pub fn get<C: Component>(&self) -> Option<&C> {
        get_component(self.entity_location, self.components, self.tables)
    }

    pub fn get_mut<C: Component>(&mut self) -> Option<&mut C> {
        get_component_mut(self.entity_location, self.components, self.tables)
    }

    pub fn as_readonly(&self) -> EntityRef {
        EntityRef {
            components: self.components,
            archetypes: self.archetypes,
            tables: self.tables,
            entity: self.entity,
            entity_location: self.entity_location,
        }
    }
}

#[derive(Debug)]
pub struct EntityWorldMut<'world> {
    world: &'world mut World,
    entity: Entity,
    entity_location: EntityLocation,
}

impl<'a> EntityWorldMut<'a> {
    pub fn id(&self) -> Entity {
        self.entity
    }

    pub fn contains<C: Component>(&self) -> bool {
        contains_component::<C>(
            self.entity_location,
            &self.world.components,
            &self.world.archetypes,
        )
    }

    pub fn get<C: Component>(&self) -> Option<&C> {
        get_component(
            self.entity_location,
            &self.world.components,
            &self.world.tables,
        )
    }

    pub fn get_mut<C: Component>(&mut self) -> Option<&mut C> {
        get_component_mut(
            self.entity_location,
            &self.world.components,
            &mut self.world.tables,
        )
    }

    pub fn despawn(self) {
        todo!();
    }

    pub fn insert(&mut self, bundle: impl Bundle) -> &mut Self {
        let bundle_info = self
            .world
            .bundles
            .insert(&bundle, &mut self.world.components);

        let add_bundle_edge = self.world.archetypes.add_bundle(
            self.entity_location.archetype_id,
            bundle_info,
            |component_id| self.world.components.get_component_info(component_id),
            |table| self.world.tables.insert(table),
        );

        //archetype.add_bundle(bundle_info.id())

        todo!();
    }

    pub fn remove<B: Bundle>(&mut self) -> &mut Self {
        let _ = self.take::<B>();
        self
    }

    #[must_use]
    pub fn take<B: Bundle>(&mut self) -> Option<B> {
        let bundle_info = self.world.bundles.get::<B>()?;

        todo!();
    }

    pub fn world(&self) -> &World {
        self.world
    }

    pub fn world_mut(&mut self) -> &mut World {
        self.world
    }

    pub fn into_world_mut(self) -> &'a mut World {
        self.world
    }
}

pub struct EntityIter<'a> {
    components: &'a Components,
    archetypes: &'a Archetypes,
    tables: &'a Tables,
    iter: EntitiesIter<'a>,
}

impl<'a> Iterator for EntityIter<'a> {
    type Item = EntityRef<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        let (entity, entity_location) = self.iter.next()?;
        Some(EntityRef {
            components: self.components,
            archetypes: self.archetypes,
            tables: self.tables,
            entity,
            entity_location,
        })
    }
}

fn contains_component<C: Component>(
    entity_location: EntityLocation,
    components: &Components,
    archetypes: &Archetypes,
) -> bool {
    let Some(component_id) = components.get_component_id::<C>()
    else {
        return false;
    };
    let archetype = archetypes.get(entity_location.archetype_id);
    archetype.contains_component(component_id)
}

fn get_component<'a, C: Component>(
    entity_location: EntityLocation,
    components: &Components,
    tables: &'a Tables,
) -> Option<&'a C> {
    let component_id = components.get_component_id::<C>()?;
    match C::STORAGE_TYPE {
        StorageType::Table => {
            let table = tables.get(entity_location.table_id);
            unsafe {
                // SAFETY: The type `C` is the type stored in the column with `component_id`.
                table.get_component(component_id, entity_location.table_row)
            }
        }
        _ => todo!(),
    }
}

fn get_component_mut<'a, C: Component>(
    entity_location: EntityLocation,
    components: &Components,
    tables: &'a mut Tables,
) -> Option<&'a mut C> {
    let component_id = components.get_component_id::<C>()?;
    match C::STORAGE_TYPE {
        StorageType::Table => {
            let table = tables.get_mut(entity_location.table_id);
            unsafe {
                // SAFETY: The type `C` is the type stored in the column with `component_id`.
                table.get_component_mut(component_id, entity_location.table_row)
            }
        }
        _ => todo!(),
    }
}
