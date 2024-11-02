use std::{
    fmt::Debug,
    iter::Enumerate,
    marker::PhantomData,
};

pub trait SparseMapKey {
    fn index(&self) -> usize;
    fn from_index(index: usize) -> Self;
}

#[derive(Clone)]
pub struct SparseMap<K, V> {
    values: Vec<Option<V>>,
    len: usize,
    _key: PhantomData<fn(K)>,
}

impl<K, V> SparseMap<K, V> {
    pub fn new() -> Self {
        Self::with_capacity(0)
    }

    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            values: Vec::with_capacity(capacity),
            len: 0,
            _key: PhantomData,
        }
    }

    pub fn clear(&mut self) {
        self.values.clear();
        self.len = 0;
    }

    pub fn reserve(&mut self, additional: usize) {
        self.values.reserve(additional);
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub fn iter(&self) -> Iter<K, V> {
        Iter {
            iter: self.values.iter().enumerate(),
            len: self.len,
            _key: PhantomData,
        }
    }

    pub fn iter_mut(&mut self) -> IterMut<K, V> {
        IterMut {
            iter: self.values.iter_mut().enumerate(),
            len: self.len,
            _key: PhantomData,
        }
    }

    pub fn values(&self) -> Values<V> {
        Values {
            iter: self.values.iter(),
            len: self.len,
        }
    }

    pub fn values_mut(&mut self) -> ValuesMut<V> {
        ValuesMut {
            iter: self.values.iter_mut(),
            len: self.len,
        }
    }

    pub fn keys(&self) -> Keys<K, V> {
        Keys {
            iter: self.values.iter().enumerate(),
            len: self.len,
            _key: PhantomData,
        }
    }
}

impl<K: SparseMapKey, V> SparseMap<K, V> {
    pub fn entry(&mut self, key: &K) -> Entry<K, V> {
        let index = key.index();
        if self.values.get(index).map_or(false, |o| o.is_some()) {
            Entry::Occupied(OccupiedEntry { index, map: self })
        }
        else {
            Entry::Vacant(VacantEntry { index, map: self })
        }
    }

    pub fn contains_key(&self, key: &K) -> bool {
        let index = key.index();
        self.values.get(index).map_or(false, |o| o.is_some())
    }

    pub fn get(&self, key: &K) -> Option<&V> {
        let index = key.index();
        self.values.get(index).map(|o| o.as_ref()).flatten()
    }

    pub fn get_mut(&mut self, key: &K) -> Option<&mut V> {
        let index = key.index();
        self.values.get_mut(index).map(|o| o.as_mut()).flatten()
    }

    pub fn insert(&mut self, key: &K, value: V) -> Option<V> {
        self.entry(key).insert(value).0
    }

    pub fn remove(&mut self, key: &K) -> Option<V> {
        self.entry(key).remove().0
    }
}

impl<K, V> Default for SparseMap<K, V> {
    fn default() -> Self {
        Self::new()
    }
}

impl<K: SparseMapKey + Debug, V: Debug> Debug for SparseMap<K, V> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_map().entries(self.iter()).finish()
    }
}

impl<K: SparseMapKey, V> FromIterator<(K, V)> for SparseMap<K, V> {
    fn from_iter<T: IntoIterator<Item = (K, V)>>(iter: T) -> Self {
        let iter = iter.into_iter();

        let size_hint = iter.size_hint();
        let capacity = size_hint.1.unwrap_or(size_hint.0);
        let mut map = SparseMap::with_capacity(capacity);

        for (key, value) in iter {
            map.insert(&key, value);
        }

        map
    }
}

pub enum Entry<'a, K, V> {
    Occupied(OccupiedEntry<'a, K, V>),
    Vacant(VacantEntry<'a, K, V>),
}

impl<'a, K, V> Entry<'a, K, V> {
    pub fn and_modify<F: FnOnce(&mut V)>(mut self, f: F) -> Self {
        match &mut self {
            Entry::Occupied(occupied_entry) => f(occupied_entry.get_mut()),
            Entry::Vacant(_vacant_entry) => {}
        }
        self
    }

    pub fn insert(self, value: V) -> (Option<V>, OccupiedEntry<'a, K, V>) {
        match self {
            Entry::Occupied(mut occupied_entry) => {
                let old_value = occupied_entry.insert(value);
                (Some(old_value), occupied_entry)
            }
            Entry::Vacant(vacant_entry) => {
                let occupied_entry = vacant_entry.insert(value);
                (None, occupied_entry)
            }
        }
    }

    pub fn or_insert_with<F: FnOnce() -> V>(self, default: F) -> OccupiedEntry<'a, K, V> {
        match self {
            Entry::Occupied(occupied_entry) => occupied_entry,
            Entry::Vacant(vacant_entry) => vacant_entry.insert(default()),
        }
    }

    pub fn or_insert(self, value: V) -> OccupiedEntry<'a, K, V> {
        self.or_insert_with(move || value)
    }

    pub fn remove(self) -> (Option<V>, VacantEntry<'a, K, V>) {
        match self {
            Entry::Occupied(occupied_entry) => {
                let (old_value, vacant_entry) = occupied_entry.remove();
                (Some(old_value), vacant_entry)
            }
            Entry::Vacant(vacant_entry) => (None, vacant_entry),
        }
    }
}

impl<'a, K: SparseMapKey, V: Default> Entry<'a, K, V> {
    pub fn or_default(self) -> OccupiedEntry<'a, K, V> {
        self.or_insert_with(Default::default)
    }
}

impl<'a, K: SparseMapKey, V> Entry<'a, K, V> {
    pub fn key(&self) -> K {
        match self {
            Entry::Occupied(occupied_entry) => occupied_entry.key(),
            Entry::Vacant(vacant_entry) => vacant_entry.key(),
        }
    }
}

impl<'a, K: SparseMapKey + Debug, V: Debug> Debug for Entry<'a, K, V> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Occupied(arg0) => f.debug_tuple("Occupied").field(arg0).finish(),
            Self::Vacant(arg0) => f.debug_tuple("Vacant").field(arg0).finish(),
        }
    }
}

pub struct OccupiedEntry<'a, K, V> {
    index: usize,
    map: &'a mut SparseMap<K, V>,
}

impl<'a, K, V> OccupiedEntry<'a, K, V> {
    pub fn get(&self) -> &V {
        self.map.values[self.index].as_ref().unwrap()
    }

    pub fn get_mut(&mut self) -> &mut V {
        self.map.values[self.index].as_mut().unwrap()
    }

    pub fn into_mut(self) -> &'a mut V {
        self.map.values[self.index].as_mut().unwrap()
    }

    pub fn insert(&mut self, value: V) -> V {
        std::mem::replace(&mut self.map.values[self.index].as_mut().unwrap(), value)
    }

    pub fn remove(self) -> (V, VacantEntry<'a, K, V>) {
        let value = self.map.values[self.index].take().unwrap();
        self.map.len -= 1;
        let vacant_entry = VacantEntry {
            index: self.index,
            map: self.map,
        };
        (value, vacant_entry)
    }
}

impl<'a, K: SparseMapKey, V> OccupiedEntry<'a, K, V> {
    pub fn key(&self) -> K {
        K::from_index(self.index)
    }
}

impl<'a, K: SparseMapKey + Debug, V: Debug> Debug for OccupiedEntry<'a, K, V> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OccupiedEntry")
            .field("key", &K::from_index(self.index))
            .field("value", &self.map.values[self.index])
            .finish()
    }
}

pub struct VacantEntry<'a, K, V> {
    index: usize,
    map: &'a mut SparseMap<K, V>,
}

impl<'a, K, V> VacantEntry<'a, K, V> {
    pub fn insert(self, value: V) -> OccupiedEntry<'a, K, V> {
        if self.index > self.map.values.len() {
            self.map.values.resize_with(self.index, || None);
        }
        self.map.values[self.index] = Some(value);
        self.map.len += 1;
        OccupiedEntry {
            index: self.index,
            map: self.map,
        }
    }
}

impl<'a, K: SparseMapKey, V> VacantEntry<'a, K, V> {
    pub fn key(&self) -> K {
        K::from_index(self.index)
    }
}
impl<'a, K: SparseMapKey + Debug, V: Debug> Debug for VacantEntry<'a, K, V> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("VacantEntry")
            .field("key", &K::from_index(self.index))
            .finish()
    }
}

#[derive(Debug)]
pub struct Iter<'a, K, V> {
    iter: std::iter::Enumerate<std::slice::Iter<'a, Option<V>>>,
    len: usize,
    _key: PhantomData<fn() -> K>,
}

impl<'a, K: SparseMapKey, V> Iterator for Iter<'a, K, V> {
    type Item = (K, &'a V);

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let (index, value) = self.iter.next()?;
            self.len -= 1;
            if let Some(value) = value {
                break Some((K::from_index(index), value));
            }
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.len, Some(self.len))
    }
}

impl<'a, K: SparseMapKey, V> DoubleEndedIterator for Iter<'a, K, V> {
    fn next_back(&mut self) -> Option<Self::Item> {
        loop {
            let (index, value) = self.iter.next_back()?;
            self.len -= 1;
            if let Some(value) = value {
                break Some((K::from_index(index), value));
            }
        }
    }
}

impl<'a, K: SparseMapKey, V> ExactSizeIterator for Iter<'a, K, V> {}

#[derive(Debug)]
pub struct IterMut<'a, K, V> {
    iter: std::iter::Enumerate<std::slice::IterMut<'a, Option<V>>>,
    len: usize,
    _key: PhantomData<fn() -> K>,
}

impl<'a, K: SparseMapKey, V> Iterator for IterMut<'a, K, V> {
    type Item = (K, &'a mut V);

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let (index, value) = self.iter.next()?;
            self.len -= 1;
            if let Some(value) = value {
                break Some((K::from_index(index), value));
            }
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.len, Some(self.len))
    }
}

impl<'a, K: SparseMapKey, V> DoubleEndedIterator for IterMut<'a, K, V> {
    fn next_back(&mut self) -> Option<Self::Item> {
        loop {
            let (index, value) = self.iter.next_back()?;
            self.len -= 1;
            if let Some(value) = value {
                break Some((K::from_index(index), value));
            }
        }
    }
}

impl<'a, K: SparseMapKey, V> ExactSizeIterator for IterMut<'a, K, V> {}

#[derive(Debug)]
pub struct Values<'a, V> {
    iter: std::slice::Iter<'a, Option<V>>,
    len: usize,
}

impl<'a, V> Iterator for Values<'a, V> {
    type Item = &'a V;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let value = self.iter.next()?;
            self.len -= 1;
            if let Some(value) = value {
                break Some(value);
            }
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.len, Some(self.len))
    }
}

impl<'a, V> DoubleEndedIterator for Values<'a, V> {
    fn next_back(&mut self) -> Option<Self::Item> {
        loop {
            let value = self.iter.next_back()?;
            self.len -= 1;
            if let Some(value) = value {
                break Some(value);
            }
        }
    }
}

impl<'a, V> ExactSizeIterator for Values<'a, V> {}

#[derive(Debug)]
pub struct ValuesMut<'a, V> {
    iter: std::slice::IterMut<'a, Option<V>>,
    len: usize,
}

impl<'a, V> Iterator for ValuesMut<'a, V> {
    type Item = &'a mut V;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let value = self.iter.next()?;
            self.len -= 1;
            if let Some(value) = value {
                break Some(value);
            }
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.len, Some(self.len))
    }
}

impl<'a, V> DoubleEndedIterator for ValuesMut<'a, V> {
    fn next_back(&mut self) -> Option<Self::Item> {
        loop {
            let value = self.iter.next_back()?;
            self.len -= 1;
            if let Some(value) = value {
                break Some(value);
            }
        }
    }
}

impl<'a, V> ExactSizeIterator for ValuesMut<'a, V> {}

#[derive(Debug)]
pub struct Keys<'a, K, V> {
    iter: std::iter::Enumerate<std::slice::Iter<'a, Option<V>>>,
    len: usize,
    _key: PhantomData<fn() -> K>,
}

impl<'a, K: SparseMapKey, V> Iterator for Keys<'a, K, V> {
    type Item = K;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let (index, value) = self.iter.next()?;
            self.len -= 1;
            if value.is_some() {
                break Some(K::from_index(index));
            }
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.len, Some(self.len))
    }
}

impl<'a, K: SparseMapKey, V> DoubleEndedIterator for Keys<'a, K, V> {
    fn next_back(&mut self) -> Option<Self::Item> {
        loop {
            let (index, value) = self.iter.next_back()?;
            self.len -= 1;
            if value.is_some() {
                break Some(K::from_index(index));
            }
        }
    }
}

impl<'a, K: SparseMapKey, V> ExactSizeIterator for Keys<'a, K, V> {}

pub struct IntoIter<K, V> {
    iter: Enumerate<std::vec::IntoIter<Option<V>>>,
    len: usize,
    _key: PhantomData<fn() -> K>,
}

impl<'a, K: SparseMapKey, V> Iterator for IntoIter<K, V> {
    type Item = (K, V);

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let (index, value) = self.iter.next()?;
            if let Some(value) = value {
                break Some((K::from_index(index), value));
            }
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.len, Some(self.len))
    }
}

impl<'a, K: SparseMapKey, V> DoubleEndedIterator for IntoIter<K, V> {
    fn next_back(&mut self) -> Option<Self::Item> {
        loop {
            let (index, value) = self.iter.next_back()?;
            if let Some(value) = value {
                break Some((K::from_index(index), value));
            }
        }
    }
}

impl<K: SparseMapKey, V> ExactSizeIterator for IntoIter<K, V> {}

impl<K: SparseMapKey, V> IntoIterator for SparseMap<K, V> {
    type Item = (K, V);
    type IntoIter = IntoIter<K, V>;

    fn into_iter(self) -> Self::IntoIter {
        IntoIter {
            iter: self.values.into_iter().enumerate(),
            len: self.len,
            _key: PhantomData,
        }
    }
}

impl<'a, K: SparseMapKey, V> IntoIterator for &'a SparseMap<K, V> {
    type Item = (K, &'a V);
    type IntoIter = Iter<'a, K, V>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl<'a, K: SparseMapKey, V> IntoIterator for &'a mut SparseMap<K, V> {
    type Item = (K, &'a mut V);
    type IntoIter = IterMut<'a, K, V>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter_mut()
    }
}

#[derive(Clone)]
pub struct ImmutableSparseMap<K, V> {
    values: Box<[Option<V>]>,
    len: usize,
    _key: PhantomData<fn(K)>,
}

impl<K, V> Default for ImmutableSparseMap<K, V> {
    fn default() -> Self {
        Self {
            values: std::iter::empty().collect(),
            len: 0,
            _key: PhantomData,
        }
    }
}

impl<K, V> ImmutableSparseMap<K, V> {
    pub fn len(&self) -> usize {
        self.len
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub fn iter(&self) -> Iter<K, V> {
        Iter {
            iter: self.values.iter().enumerate(),
            len: self.len,
            _key: PhantomData,
        }
    }

    pub fn iter_mut(&mut self) -> IterMut<K, V> {
        IterMut {
            iter: self.values.iter_mut().enumerate(),
            len: self.len,
            _key: PhantomData,
        }
    }

    pub fn values(&self) -> Values<V> {
        Values {
            iter: self.values.iter(),
            len: self.len,
        }
    }

    pub fn values_mut(&mut self) -> ValuesMut<V> {
        ValuesMut {
            iter: self.values.iter_mut(),
            len: self.len,
        }
    }

    pub fn keys(&self) -> Keys<K, V> {
        Keys {
            iter: self.values.iter().enumerate(),
            len: self.len,
            _key: PhantomData,
        }
    }
}

impl<K: SparseMapKey, V> ImmutableSparseMap<K, V> {
    pub fn contains_key(&self, key: &K) -> bool {
        let index = key.index();
        self.values.get(index).map_or(false, |o| o.is_some())
    }

    pub fn get(&self, key: &K) -> Option<&V> {
        let index = key.index();
        self.values.get(index).map(|o| o.as_ref()).flatten()
    }

    pub fn get_mut(&mut self, key: &K) -> Option<&mut V> {
        let index = key.index();
        self.values.get_mut(index).map(|o| o.as_mut()).flatten()
    }
}

impl<K, V> From<SparseMap<K, V>> for ImmutableSparseMap<K, V> {
    fn from(map: SparseMap<K, V>) -> Self {
        Self {
            values: map.values.into(),
            len: map.len,
            _key: PhantomData,
        }
    }
}

impl<K, V> From<ImmutableSparseMap<K, V>> for SparseMap<K, V> {
    fn from(map: ImmutableSparseMap<K, V>) -> Self {
        Self {
            values: map.values.into(),
            len: map.len,
            _key: PhantomData,
        }
    }
}

impl<K: SparseMapKey, V> FromIterator<(K, V)> for ImmutableSparseMap<K, V> {
    fn from_iter<T: IntoIterator<Item = (K, V)>>(iter: T) -> Self {
        SparseMap::from_iter(iter).into()
    }
}

impl<K: SparseMapKey + Debug, V: Debug> Debug for ImmutableSparseMap<K, V> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_map().entries(self.iter()).finish()
    }
}

impl<K: SparseMapKey, V> IntoIterator for ImmutableSparseMap<K, V> {
    type Item = (K, V);
    type IntoIter = IntoIter<K, V>;

    fn into_iter(self) -> Self::IntoIter {
        IntoIter {
            iter: self.values.into_vec().into_iter().enumerate(),
            len: self.len,
            _key: PhantomData,
        }
    }
}

impl<'a, K: SparseMapKey, V> IntoIterator for &'a ImmutableSparseMap<K, V> {
    type Item = (K, &'a V);
    type IntoIter = Iter<'a, K, V>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl<'a, K: SparseMapKey, V> IntoIterator for &'a mut ImmutableSparseMap<K, V> {
    type Item = (K, &'a mut V);
    type IntoIter = IterMut<'a, K, V>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter_mut()
    }
}
