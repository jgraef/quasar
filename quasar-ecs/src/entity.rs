use core::fmt;
use std::{
    hash::Hash,
    num::NonZero,
};

use crate::{
    archetype::{
        ArchetypeId,
        ArchetypeRow,
    },
    storage::table::{
        TableId,
        TableRow,
    },
};

#[derive(Clone, Copy)]
pub struct Entity {
    index: u32,
    generation: EntityGeneration,
}

impl Entity {
    pub const PLACEHOLDER: Self = Self::new(u32::MAX, EntityGeneration::NEW);

    #[inline(always)]
    pub(crate) const fn new(index: u32, generation: EntityGeneration) -> Entity {
        Self { index, generation }
    }

    pub fn generation(&self) -> EntityGeneration {
        self.generation
    }

    pub fn is_placeholder(&self) -> bool {
        self == &Self::PLACEHOLDER
    }

    pub fn to_bits(&self) -> u64 {
        u64::from(self.index) | u64::from(self.generation.0.get()) << 32
    }

    pub fn as_index(&self) -> usize {
        self.index as usize
    }
}

impl PartialEq for Entity {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.to_bits() == other.to_bits()
    }
}

impl Eq for Entity {}

impl PartialOrd for Entity {
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Entity {
    #[inline]
    fn cmp(&self, other: &Self) -> core::cmp::Ordering {
        self.to_bits().cmp(&other.to_bits())
    }
}

impl Hash for Entity {
    #[inline]
    fn hash<H: core::hash::Hasher>(&self, state: &mut H) {
        self.to_bits().hash(state);
    }
}

impl fmt::Debug for Entity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        format_entity(*self, f)
    }
}

fn format_entity(entity: Entity, f: &mut fmt::Formatter) -> fmt::Result {
    if entity.is_placeholder() {
        write!(f, "PLACEHOLDER")
    }
    else {
        write!(f, "{}v{}", entity.index, entity.generation.0,)
    }
}

#[derive(Debug, Default)]
pub struct Entities {
    meta: Vec<EntityMeta>,
    free_list: Vec<Entity>,
}

impl Entities {
    pub fn clear(&mut self) {
        self.meta.clear();
        // todo: don't we need to keep track of entity generations?
        self.free_list.clear();
    }

    pub fn allocate(&mut self) -> Entity {
        if let Some(mut entity) = self.free_list.pop() {
            entity.generation.increment();
            entity
        }
        else {
            let index = self.meta.len();
            self.meta.push(EntityMeta::EMPTY);
            Entity {
                index: index.try_into().expect("Entity index overflow"),
                generation: EntityGeneration::NEW,
            }
        }
    }

    pub fn free(&mut self, entity: Entity) {
        let meta = &mut self.meta[entity.as_index()];
        if meta.generation == entity.generation {
            *meta = EntityMeta::EMPTY;
            self.free_list.push(entity);
        }
        else {
            assert!(entity.generation < meta.generation);
        }
    }

    pub fn set_location(&mut self, entity: Entity, location: EntityLocation) {
        let meta = &mut self.meta[entity.as_index()];
        meta.generation = entity.generation;
        meta.location = location;
    }

    pub fn get_location(&self, entity: Entity) -> Option<EntityLocation> {
        let meta = self.meta.get(entity.index as usize)?;
        if entity.generation == meta.generation {
            Some(meta.location)
        }
        else {
            assert!(entity.generation < meta.generation);
            None
        }
    }

    pub fn iter(&self) -> EntitiesIter {
        EntitiesIter {
            iter: self
                .meta
                .iter()
                .enumerate()
                .filter_map(entities_iter_filter_map),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct EntityMeta {
    generation: EntityGeneration,
    location: EntityLocation,
}

impl EntityMeta {
    const EMPTY: Self = Self {
        generation: EntityGeneration::INVALID,
        location: EntityLocation::INVALID,
    };

    pub fn is_empty(&self) -> bool {
        *self == Self::EMPTY
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct EntityGeneration(NonZero<u32>);

impl EntityGeneration {
    pub const INVALID: Self = Self(NonZero::<u32>::MAX);
    pub const NEW: Self = Self(NonZero::<u32>::MIN);

    pub fn is_invalid(&self) -> bool {
        *self == Self::INVALID
    }
}

impl EntityGeneration {
    pub fn increment(&mut self) {
        self.0 = NonZero::new(self.0.get() + 1).expect("EntityGeneration overflow");
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EntityLocation {
    pub archetype_id: ArchetypeId,
    pub archetype_row: ArchetypeRow,
    pub table_id: TableId,
    pub table_row: TableRow,
}

impl EntityLocation {
    pub const INVALID: Self = Self {
        archetype_id: ArchetypeId::INVALID,
        archetype_row: ArchetypeRow::INVALID,
        table_id: TableId::INVALID,
        table_row: TableRow::INVALID,
    };

    pub fn is_invalid(&self) -> bool {
        *self == Self::INVALID
    }
}

pub struct EntitiesIter<'a> {
    iter: std::iter::FilterMap<
        std::iter::Enumerate<std::slice::Iter<'a, EntityMeta>>,
        fn((usize, &'a EntityMeta)) -> Option<(Entity, EntityLocation)>,
    >,
}

impl<'a> Iterator for EntitiesIter<'a> {
    type Item = (Entity, EntityLocation);

    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next()
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.iter.size_hint()
    }
}

fn entities_iter_filter_map(
    (index, entity_meta): (usize, &EntityMeta),
) -> Option<(Entity, EntityLocation)> {
    if entity_meta.is_empty() {
        None
    }
    else {
        Some((
            Entity {
                index: index.try_into().unwrap(),
                generation: entity_meta.generation,
            },
            entity_meta.location,
        ))
    }
}
