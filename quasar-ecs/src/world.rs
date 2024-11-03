use std::{
    marker::PhantomData,
    num::NonZeroUsize,
    sync::atomic::{
        AtomicUsize,
        Ordering,
    },
};

use crate::{
    archetype::{
        create_archetype,
        Archetype,
        ArchetypeEntity,
        ArchetypeId,
        Archetypes,
    },
    bundle::{
        Bundle,
        BundleInfo,
        Bundles,
        DynamicBundle,
        InsertComponentsIntoTable,
        TakeComponentsFromTable,
    },
    component::{
        Component,
        ComponentId,
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
        table::{
            InsertIntoTable,
            MoveRowDropUnmatched,
            MoveRowForgetUnmatched,
            MoveRowHandleUnmatched,
            MoveRowPanicUnmatched,
            Table,
            TableId,
            TableRow,
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
            entity_location: EntityLocation::EMPTY,
        }
    }

    pub fn spawn(&mut self, bundle: impl DynamicBundle) -> EntityWorldMut {
        let mut entity = self.spawn_empty();
        entity.insert(bundle);
        entity
    }

    pub fn despawn(&mut self, entity: Entity) {
        if let Some(entity) = self.get_entity_world_mut(entity) {
            entity.despawn();
        }
    }

    pub fn take<B: Bundle>(&mut self, entity: Entity) -> Option<B> {
        self.get_entity_world_mut(entity)?.take()
    }

    pub fn remove<B: Bundle>(&mut self, entity: Entity) {
        if let Some(mut entity) = self.get_entity_world_mut(entity) {
            entity.remove::<B>();
        }
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

    pub fn get_entity_world_mut(&mut self, entity: Entity) -> Option<EntityWorldMut> {
        let entity_location = self.entities.get_location(entity)?;
        Some(EntityWorldMut {
            world: self,
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

    pub fn insert(&mut self, bundle: impl DynamicBundle) -> &mut Self {
        self.insert_remove_take_inner(InsertOp { bundle });
        self
    }

    pub fn remove<B: Bundle>(&mut self) {
        self.insert_remove_take_inner(RemoveOp::<B> {
            _bundle: PhantomData,
        });
    }

    #[must_use]
    pub fn take<B: Bundle>(&mut self) -> Option<B> {
        self.insert_remove_take_inner(TakeOp::<B> {
            _bundle: PhantomData,
        })
    }

    /// Helper method to perform [`insert`], [`remove`] and [`take`].
    ///
    /// [`insert`], [`remove`] and [`take`] are very similar since they all move
    /// an entity from one archetype to another, moving its data from one table
    /// to another. More specifically, sometimes they don't actually need to
    /// move between archetypes or tables (e.g. inserting `()`). All these
    /// operations are done using this general method, and are specialized
    /// via the `op` parameter and the [`InsertRemoveTakeOp`] trait.
    fn insert_remove_take_inner<O: InsertRemoveTakeOp>(&mut self, op: O) -> Option<O::Output> {
        // if this op produces an output (i.e. take), it might store it here. this might
        // also stay None, if the operation fails (e.g. the entity doesn't contain the
        // full bundle)
        let mut output = None;

        // get info for this bundle
        let bundle_info = op.get_bundle_info(&mut self.world.bundles, &mut self.world.components);

        // add/remove bundle to the archetype graph. this creates an
        // AddBundle/RemoveBundle edge from `self.entity_location.archetype_id`
        // to whatever archetype we get after the insertion.
        //
        // this also creates the resulting archetype (and table) if necessary, by
        // calling the provided closure.
        //
        // if adding/removing this bundle doesn't change the archetype (e.g. adding
        // `()`), this returns `None`. if it does change the entity's archetype,
        // this returns a mutable borrow for the old and new archetype.

        if let Some((from_archetype, to_archetype)) = op.get_bundle_edge(
            &mut self.world.archetypes,
            self.entity_location.archetype_id,
            bundle_info,
            |archetype_id, component_ids| {
                create_archetype(
                    archetype_id,
                    component_ids,
                    &self.world.components,
                    &mut self.world.tables,
                )
            },
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

                    // first take out anything we want to return
                    // note: if the op takes out anything it must make sure it's only components
                    // that are not moved to the new table, and those are forgotten when
                    // `from_table.move_row` handles them as unmatched.
                    output = Some(op.take(bundle_info, from_table, self.entity_location.table_row));

                    // `Table::move_row` will move our entity's row from `from_table` to `to_table`,
                    // moving all the data in the columns. Note that this will
                    // only populate columns in `to_table` that exist in both tables. In our case
                    // we'll still need to add some components from the bundle.
                    //
                    // `Table::move_row` handily also returns a `InsertIntoTable`, with which we can
                    // insert the remaining components later.

                    let mut move_result = unsafe {
                        from_table.move_row(
                            self.entity_location.table_row,
                            to_table,
                            self.entity,
                            op.handle_unmatched(),
                        )
                    };

                    new_entity_location.table_row = move_result.to_row();

                    // while removing our entity from `from_table`, another row was swapped into its
                    // place. we need to update its information
                    if let Some(changed_location) = move_result.swapped {
                        changed_location.apply(&mut self.world.entities);
                    }

                    // insert the remaining components from the bundle
                    op.insert(bundle_info, &mut move_result.insert, from_archetype);
                }
                Err(_table) => {
                    // either both archetypes have the same table, or `from_row`
                    // is invalid, so there's nothing to do.
                    // the bundle also can't add any components we don't
                    // already have, or remove any components.
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

        output
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

unsafe trait InsertRemoveTakeOp {
    type Output;

    fn get_bundle_info<'a>(
        &self,
        bundles: &'a mut Bundles,
        components: &mut Components,
    ) -> &'a BundleInfo;

    fn get_bundle_edge<'a>(
        &self,
        archetypes: &'a mut Archetypes,
        archetype_id: ArchetypeId,
        bundle_info: &BundleInfo,
        create_archetype: impl FnOnce(ArchetypeId, &[ComponentId]) -> Archetype,
    ) -> Option<(&'a mut Archetype, &'a mut Archetype)>;

    fn handle_unmatched(&self) -> impl MoveRowHandleUnmatched;

    fn insert(
        self,
        bundle_info: &BundleInfo,
        insert_into_table: &mut InsertIntoTable,
        from_archetype: &Archetype,
    );

    fn take(
        &self,
        bundle_info: &BundleInfo,
        table: &mut Table,
        table_row: TableRow,
    ) -> Self::Output;
}

struct InsertOp<B> {
    bundle: B,
}

unsafe impl<B: DynamicBundle> InsertRemoveTakeOp for InsertOp<B> {
    type Output = ();

    fn get_bundle_info<'a>(
        &self,
        bundles: &'a mut Bundles,
        components: &mut Components,
    ) -> &'a BundleInfo {
        bundles.get_mut_or_insert_dynamic(&self.bundle, components)
    }

    fn get_bundle_edge<'a>(
        &self,
        archetypes: &'a mut Archetypes,
        archetype_id: ArchetypeId,
        bundle_info: &BundleInfo,
        create_archetype: impl FnOnce(ArchetypeId, &[ComponentId]) -> Archetype,
    ) -> Option<(&'a mut Archetype, &'a mut Archetype)> {
        archetypes.add_bundle(archetype_id, bundle_info, create_archetype)
    }

    fn handle_unmatched(&self) -> impl MoveRowHandleUnmatched {
        MoveRowPanicUnmatched
    }

    fn insert(
        self,
        bundle_info: &BundleInfo,
        insert_into_table: &mut InsertIntoTable,
        from_archetype: &Archetype,
    ) {
        // get the AddBundle edge. we need its metadata about duplicate components to
        // not add components from the bundle that were also moved over from
        // `from_table`.
        let add_bundle = from_archetype.add_bundle(bundle_info.id()).unwrap();

        // insert the remaining components from the bundle
        self.bundle.into_components(InsertComponentsIntoTable::new(
            bundle_info,
            |component_id| !add_bundle.duplicate.contains(&component_id),
            insert_into_table,
        ));
    }

    fn take(
        &self,
        _bundle_info: &BundleInfo,
        _table: &mut Table,
        _table_row: TableRow,
    ) -> Self::Output {
        ()
    }
}

struct RemoveOp<B> {
    _bundle: PhantomData<B>,
}

unsafe impl<B: Bundle> InsertRemoveTakeOp for RemoveOp<B> {
    type Output = ();

    fn get_bundle_info<'a>(
        &self,
        bundles: &'a mut Bundles,
        components: &mut Components,
    ) -> &'a BundleInfo {
        bundles.get_mut_or_insert_static::<B>(components)
    }

    fn get_bundle_edge<'a>(
        &self,
        archetypes: &'a mut Archetypes,
        archetype_id: ArchetypeId,
        bundle_info: &BundleInfo,
        create_archetype: impl FnOnce(ArchetypeId, &[ComponentId]) -> Archetype,
    ) -> Option<(&'a mut Archetype, &'a mut Archetype)> {
        archetypes.remove_bundle(archetype_id, bundle_info, create_archetype)
    }

    fn handle_unmatched(&self) -> impl MoveRowHandleUnmatched {
        MoveRowDropUnmatched
    }

    fn insert(
        self,
        _bundle_info: &BundleInfo,
        _insert_into_table: &mut InsertIntoTable,
        _from_archetype: &Archetype,
    ) {
    }

    fn take(
        &self,
        _bundle_info: &BundleInfo,
        _table: &mut Table,
        _table_row: TableRow,
    ) -> Self::Output {
        ()
    }
}

struct TakeOp<B> {
    _bundle: PhantomData<B>,
}

unsafe impl<B: Bundle> InsertRemoveTakeOp for TakeOp<B> {
    type Output = B;

    fn get_bundle_info<'a>(
        &self,
        bundles: &'a mut Bundles,
        components: &mut Components,
    ) -> &'a BundleInfo {
        bundles.get_mut_or_insert_static::<B>(components)
    }

    fn get_bundle_edge<'a>(
        &self,
        archetypes: &'a mut Archetypes,
        archetype_id: ArchetypeId,
        bundle_info: &BundleInfo,
        create_archetype: impl FnOnce(ArchetypeId, &[ComponentId]) -> Archetype,
    ) -> Option<(&'a mut Archetype, &'a mut Archetype)> {
        archetypes.remove_bundle(archetype_id, bundle_info, create_archetype)
    }

    fn handle_unmatched(&self) -> impl MoveRowHandleUnmatched {
        MoveRowForgetUnmatched
    }

    fn insert(
        self,
        _bundle_info: &BundleInfo,
        _insert_into_table: &mut InsertIntoTable,
        _from_archetype: &Archetype,
    ) {
    }

    fn take(
        &self,
        bundle_info: &BundleInfo,
        table: &mut Table,
        table_row: TableRow,
    ) -> Self::Output {
        B::from_components(TakeComponentsFromTable::new(bundle_info, table, table_row))
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{
        AtomicBool,
        Ordering,
    };

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

        let component = MyComponent {
            position: 42,
            velocity: 1312,
        };
        let entity = world.spawn(component);

        // test if we can access the component from the returned `EntityWorldMut`
        let component2 = entity.get::<MyComponent>().unwrap();
        assert_eq!(component, *component2);

        // test if we can access the entity and component from the world
        let entity = entity.id();
        let entity = world.get_entity(entity).unwrap();
        let component2 = entity.get::<MyComponent>().unwrap();
        assert_eq!(component, *component2);
    }

    #[test]
    fn spawn_empty() {
        let mut world = World::new();
        let entity = world.spawn_empty().id();
        world.get_entity(entity).unwrap();
    }

    #[test]
    fn remove_component() {
        #[derive(Component)]
        struct MyComponent;

        let mut world = World::new();
        let mut entity = world.spawn(MyComponent);
        entity.remove::<MyComponent>();

        assert!(entity.get::<MyComponent>().is_none());
    }

    #[test]
    fn it_doesnt_drop_inserted_components() {
        static WAS_DROPPED: AtomicBool = AtomicBool::new(false);

        #[derive(Component)]
        struct MyComponent;

        impl Drop for MyComponent {
            fn drop(&mut self) {
                WAS_DROPPED.store(true, Ordering::Relaxed);
            }
        }

        let mut world = World::new();
        let _ = world.spawn(MyComponent);

        assert!(!WAS_DROPPED.load(Ordering::Relaxed));
    }

    #[test]
    fn it_doesnt_drop_taken_components() {
        static WAS_DROPPED: AtomicBool = AtomicBool::new(false);

        #[derive(Component)]
        struct MyComponent;

        impl Drop for MyComponent {
            fn drop(&mut self) {
                WAS_DROPPED.store(true, Ordering::Relaxed);
            }
        }

        let mut world = World::new();
        let mut entity = world.spawn(MyComponent);
        let _component = entity.take::<MyComponent>().unwrap();

        assert!(!WAS_DROPPED.load(Ordering::Relaxed));
    }

    #[test]
    fn it_does_drop_components_when_world_is_dropped() {
        static WAS_DROPPED: AtomicBool = AtomicBool::new(false);

        #[derive(Component)]
        struct MyComponent;

        impl Drop for MyComponent {
            fn drop(&mut self) {
                WAS_DROPPED.store(true, Ordering::Relaxed);
            }
        }

        let mut world = World::new();
        let _ = world.spawn(MyComponent);
        drop(world);

        assert!(WAS_DROPPED.load(Ordering::Relaxed));
    }

    #[test]
    fn it_does_drop_components_when_theyre_removed() {
        static WAS_DROPPED: AtomicBool = AtomicBool::new(false);

        #[derive(Component)]
        struct MyComponent;

        impl Drop for MyComponent {
            fn drop(&mut self) {
                WAS_DROPPED.store(true, Ordering::Relaxed);
            }
        }

        let mut world = World::new();
        let mut entity = world.spawn(MyComponent);
        entity.remove::<MyComponent>();

        assert!(WAS_DROPPED.load(Ordering::Relaxed));
    }

    #[test]
    fn take_component() {
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Component)]
        struct MyComponent(u32);

        let mut world = World::new();
        let component = MyComponent(1312);
        let mut entity = world.spawn(component);

        let component2 = entity.take::<MyComponent>().unwrap();
        assert_eq!(component, component2);
    }

    #[test]
    fn taking_a_component_twice_fails() {
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Component)]
        struct MyComponent(u32);

        let mut world = World::new();
        let component = MyComponent(1312);
        let mut entity = world.spawn(component);

        let component2 = entity.take::<MyComponent>().unwrap();
        assert_eq!(component, component2);

        assert!(entity.take::<MyComponent>().is_none());
    }
}
