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
        ChangedLocation,
        Entities,
        EntitiesIter,
        Entity,
        EntityLocation,
    },
    resources::Resources,
    storage::{
        table::Tables,
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
            entity_location: EntityLocation::EMPTY,
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
        // get info for this bundle
        let bundle_info = self
            .world
            .bundles
            .get_mut_or_insert(&bundle, &mut self.world.components);

        // add bundle to the archetype graph. this creates an AddBundle edge from
        // `self.entity_location.archetype_id` to whatever archetype we get
        // after the insertion.
        //
        // this also creates the resulting archetype (and table) if necessary.
        //
        // if adding this bundle doesn't change the archetype (e.g. adding `()`), this
        // returns `None`. if it does change the entity's archetype,
        // this returns a mutable borrow for the old and new archetype.
        if let Some((from_archetype, to_archetype)) = self.world.archetypes.add_bundle(
            self.entity_location.archetype_id,
            bundle_info,
            |component_id| self.world.components.get_component_info(component_id),
            |table| self.world.tables.insert(table),
        ) {
            // create a new location for our entity. we'll populate it as we get the
            // information.
            let mut new_entity_location = self.entity_location.clone();
            new_entity_location.archetype_id = to_archetype.id();
            new_entity_location.table_id = to_archetype.table_id();

            // `Table::get_mut_pair` either returns a pair of mutable borrows of tables for
            // the supplied table IDs, if they're not identical, or a single
            // mutable borrow for the table
            match self
                .world
                .tables
                .get_mut_pair(from_archetype.table_id(), to_archetype.table_id())
            {
                Ok((from_table, to_table)) => {
                    // moving our entity actually involves moving from a table to another table.
                    //
                    // `Table::move_row` will move our entity's row from `from_table` to `to_table`,
                    // moving all the data in the columns. Note that this will
                    // only populate columns in `to_table` that exist in both tables. In our case
                    // we'll still need to add some components from the bundle.
                    //
                    // `Table::move_row` handily also returns a `InsertIntoTable`, with which we can
                    // insert the remaining components later.

                    let mut move_result =
                        unsafe { from_table.move_row(self.entity_location.table_row, to_table, self.entity) };

                    new_entity_location.table_row = move_result.to_row();

                    // while removing our entity from `from_table`, another row was swapped into its
                    // place. we need to update its information
                    if let Some(changed_location) = move_result.swapped {
                        changed_location.apply(&mut self.world.entities);
                    }

                    // get the AddBundle edge. we need its metadata about duplicate components to
                    // not add components from the bundle that were also moved over from
                    // `from_table`.
                    let add_bundle = from_archetype.add_bundle(bundle_info.id()).unwrap();

                    // insert the remaining components from the bundle
                    bundle.into_each_component(InsertComponentsIntoTable::new(
                        &bundle_info,
                        |component_id| !add_bundle.duplicate.contains(&component_id),
                        &mut move_result.insert,
                    ));
                }
                Err(table) => {
                    // either both archetypes have the same table, or `from_row` is invalid, so there's nothing
                    // to do. the bundle also can't add any components we don't
                    // already have.
                }
            };

            // remove our entity from `from_archetype`. this might again involve updating
            // metadata from another entity due to swapping.
            if let Some(changed_location) =
                from_archetype.remove_entity(self.entity_location.archetype_row)
            {
                changed_location.apply(&mut self.world.entities);
            }

            // we can finally insert our entity into the new archetype
            new_entity_location.archetype_row = to_archetype.insert_entity(ArchetypeEntity {
                entity: self.entity,
                table_row: new_entity_location.table_row,
            });

            // update our entity's location metadata
            ChangedLocation {
                entity: self.entity,
                changed_value: new_entity_location,
            }
            .apply(&mut self.world.entities);

            // update the cached `EntityLocation`
            self.entity_location = new_entity_location;
        }

        self
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
    dbg!(&entity_location);

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

#[cfg(test)]
mod tests {
    use quasar_ecs_derive::Component;

    use crate::World;

    #[test]
    fn spawn_component() {
        #[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Component)]
        struct MyComponent {
            position: u32,
            velocity: u32,
        }

        let mut world = World::new();

        let component = MyComponent { position: 42, velocity: 1312 };
        let entity = world.spawn(component);
        dbg!(&entity);
        
        // test if we can access the component from the returned `EntityWorldMut`
        let component2 = entity.get::<MyComponent>().unwrap();
        assert_eq!(component, *component2);

        // test if we can access the entity and component from the world
        let entity = entity.id();
        let entity = world.get_entity(entity).unwrap();
        let component2 = entity.get::<MyComponent>().unwrap();
        assert_eq!(component, *component2);
    }
}