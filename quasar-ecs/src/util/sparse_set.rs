use std::fmt::Debug;

use crate::util::sparse_map::{
    self,
    ImmutableSparseMap,
    SparseMap,
    SparseMapKey,
};

#[derive(Clone, Default)]
pub struct SparseSet<K> {
    map: SparseMap<K, ()>,
}

impl<K> SparseSet<K> {
    pub fn new() -> Self {
        Self {
            map: SparseMap::new(),
        }
    }

    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            map: SparseMap::with_capacity(capacity),
        }
    }

    pub fn reserve(&mut self, additional: usize) {
        self.map.reserve(additional);
    }

    pub fn clear(&mut self) {
        self.map.clear();
    }

    pub fn len(&self) -> usize {
        self.map.len()
    }

    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }
}

impl<K: SparseMapKey> SparseSet<K> {
    pub fn contains(&self, key: &K) -> bool {
        self.map.contains_key(key)
    }

    pub fn insert(&mut self, key: &K) -> bool {
        self.map.insert(key, ()).is_some()
    }

    pub fn remove(&mut self, key: &K) -> bool {
        self.map.remove(key).is_some()
    }

    pub fn iter(&self) -> Iter<K> {
        Iter {
            iter: self.map.keys(),
        }
    }
}

impl<'a, K: SparseMapKey> IntoIterator for &'a SparseSet<K> {
    type Item = K;
    type IntoIter = Iter<'a, K>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl<K: SparseMapKey> FromIterator<K> for SparseSet<K> {
    fn from_iter<T: IntoIterator<Item = K>>(iter: T) -> Self {
        Self {
            map: iter.into_iter().map(|key| (key, ())).collect(),
        }
    }
}

impl<K: SparseMapKey + Debug> Debug for SparseSet<K> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_set().entries(self.iter()).finish()
    }
}

pub struct Iter<'a, K> {
    iter: sparse_map::Keys<'a, K, ()>,
}

impl<'a, K: SparseMapKey> Iterator for Iter<'a, K> {
    type Item = K;

    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next()
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.iter.size_hint()
    }
}

impl<'a, K: SparseMapKey> DoubleEndedIterator for Iter<'a, K> {
    fn next_back(&mut self) -> Option<Self::Item> {
        self.iter.next_back()
    }
}

#[derive(Clone, Default)]
pub struct ImmutableSparseSet<K> {
    map: ImmutableSparseMap<K, ()>,
}

impl<K> ImmutableSparseSet<K> {
    pub fn len(&self) -> usize {
        self.map.len()
    }

    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }
}

impl<K: SparseMapKey> ImmutableSparseSet<K> {
    pub fn contains(&self, key: &K) -> bool {
        self.map.contains_key(key)
    }

    pub fn iter(&self) -> Iter<K> {
        Iter {
            iter: self.map.keys(),
        }
    }
}

impl<'a, K: SparseMapKey> IntoIterator for &'a ImmutableSparseSet<K> {
    type Item = K;
    type IntoIter = Iter<'a, K>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl<K: SparseMapKey> FromIterator<K> for ImmutableSparseSet<K> {
    fn from_iter<T: IntoIterator<Item = K>>(iter: T) -> Self {
        SparseSet::from_iter(iter).into()
    }
}

impl<K: SparseMapKey + Debug> Debug for ImmutableSparseSet<K> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_set().entries(self.iter()).finish()
    }
}

impl<K> From<SparseSet<K>> for ImmutableSparseSet<K> {
    fn from(set: SparseSet<K>) -> Self {
        Self {
            map: set.map.into(),
        }
    }
}

impl<K> From<ImmutableSparseSet<K>> for SparseSet<K> {
    fn from(set: ImmutableSparseSet<K>) -> Self {
        Self {
            map: set.map.into(),
        }
    }
}
