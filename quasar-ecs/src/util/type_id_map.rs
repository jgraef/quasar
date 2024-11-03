use std::{
    any::{type_name, TypeId}, collections::{
        hash_map,
        HashMap,
    }, fmt::Debug, iter::FusedIterator, marker::PhantomData
};

#[derive(Clone)]
pub struct TypeIdMap<T> {
    inner: HashMap<TypeId, Item<T>>,
}

#[derive(Clone, Debug)]
struct Item<T> {
    key_type_name: &'static str,
    value: T,
}

impl<T> Default for TypeIdMap<T> {
    fn default() -> Self {
        Self {
            inner: HashMap::new(),
        }
    }
}

impl<T> TypeIdMap<T> {
    pub fn len(&self) -> usize {
        self.inner.len()
    }

    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    pub fn clear(&mut self) {
        self.inner.clear();
    }

    pub fn insert<K: 'static>(&mut self, value: T) {
        self.inner.insert(TypeId::of::<K>(), Item {
            key_type_name: type_name::<K>(),
            value,
        });
    }

    pub fn get<K: 'static>(&self) -> Option<&T> {
        Some(&self.inner.get(&TypeId::of::<K>())?.value)
    }

    pub fn get_mut<K: 'static>(&mut self) -> Option<&mut T> {
        Some(&mut self.inner.get_mut(&TypeId::of::<K>())?.value)
    }

    pub fn entry<K: 'static>(&mut self) -> Entry<T> {
        match self.inner.entry(TypeId::of::<K>()) {
            hash_map::Entry::Occupied(occupied_entry) => {
                Entry::Occupied(OccupiedEntry {
                    key_type_name: type_name::<K>(),
                    inner: occupied_entry,
                })
            },
            hash_map::Entry::Vacant(vacant_entry) => {
                Entry::Vacant(VacantEntry {
                    key_type_name: type_name::<K>(),
                    inner: vacant_entry,
                })
            },
        }
    }

    pub fn values(&self) -> Values<T> {
        Values {
            inner: self.inner.iter(),
        }
    }

    pub fn values_mut(&mut self) -> ValuesMut<T> {
        ValuesMut {
            inner: self.inner.iter_mut(),
        }
    }
}

impl<T: Debug> Debug for TypeIdMap<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut map = f.debug_map();
        for item in self.inner.values() {
            map.entry(&item.key_type_name, &item.value);
        }
        map.finish()
    }
}

#[derive(Debug)]
pub enum Entry<'a, T> {
    Occupied(OccupiedEntry<'a, T>),
    Vacant(VacantEntry<'a, T>),
}


impl<'a, V> Entry<'a, V> {
    pub fn and_modify<F: FnOnce(&mut V)>(mut self, f: F) -> Self {
        match &mut self {
            Entry::Occupied(occupied_entry) => f(occupied_entry.get_mut()),
            Entry::Vacant(_vacant_entry) => {}
        }
        self
    }

    pub fn insert(self, value: V) -> (Option<V>, OccupiedEntry<'a, V>) {
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

    pub fn or_insert_with<F: FnOnce() -> V>(self, default: F) -> OccupiedEntry<'a, V> {
        match self {
            Entry::Occupied(occupied_entry) => occupied_entry,
            Entry::Vacant(vacant_entry) => vacant_entry.insert(default()),
        }
    }

    pub fn or_insert(self, value: V) -> OccupiedEntry<'a, V> {
        self.or_insert_with(move || value)
    }

    pub fn remove(self) -> Option<V> {
        match self {
            Entry::Occupied(occupied_entry) => {
                Some(occupied_entry.remove())
            }
            Entry::Vacant(_vacant_entry) => None,
        }
    }
}

impl<'a, V: Default> Entry<'a, V> {
    pub fn or_default(self) -> OccupiedEntry<'a, V> {
        self.or_insert_with(Default::default)
    }
}

pub struct OccupiedEntry<'a, V> {
    key_type_name: &'static str,
    inner: hash_map::OccupiedEntry<'a, TypeId, Item<V>>
}

impl<'a, V> OccupiedEntry<'a, V> {
    pub fn get(&self) -> &V {
        &self.inner.get().value
    }

    pub fn get_mut(&mut self) -> &mut V {
        &mut self.inner.get_mut().value
    }

    pub fn into_mut(self) -> &'a mut V {
        &mut self.inner.into_mut().value
    }

    pub fn insert(&mut self, value: V) -> V {
        self.inner.insert(Item {
            key_type_name: self.key_type_name,
            value,
        }).value
    }

    pub fn remove(self) -> V {
        self.inner.remove().value
    }
}

impl<'a, V: Debug> Debug for OccupiedEntry<'a, V> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OccupiedEntry")
            .field("key", &self.key_type_name)
            .field("value", self.get())
            .finish()
    }
}

pub struct VacantEntry<'a, V> {
    key_type_name: &'static str,
    inner: hash_map::VacantEntry<'a, TypeId, Item<V>>,
}

impl<'a, V> VacantEntry<'a, V> {
    pub fn insert(self, value: V) -> OccupiedEntry<'a, V> {
        OccupiedEntry {
            key_type_name: self.key_type_name,
            inner: self.inner.insert_entry(Item {
                key_type_name: self.key_type_name,
                value 
            }),
        }
    }
}

impl<'a, V: Debug> Debug for VacantEntry<'a, V> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("VacantEntry")
            .field("key", &self.key_type_name)
            .finish()
    }
}

#[derive(Debug)]
pub struct Values<'a, T> {
    inner: hash_map::Iter<'a, TypeId, Item<T>>,
}

impl<'a, T> Iterator for Values<'a, T> {
    type Item = &'a T;

    fn next(&mut self) -> Option<Self::Item> {
        let (_key, item) = self.inner.next()?;
        Some(&item.value)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }
}

impl<'a, T> ExactSizeIterator for Values<'a, T> {}

impl<'a, T> FusedIterator for Values<'a, T> {}

#[derive(Debug)]
pub struct ValuesMut<'a, T> {
    inner: hash_map::IterMut<'a, TypeId, Item<T>>,
}

impl<'a, T> Iterator for ValuesMut<'a, T> {
    type Item = &'a mut T;

    fn next(&mut self) -> Option<Self::Item> {
        let (_key, item) = self.inner.next()?;
        Some(&mut item.value)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }
}

impl<'a, T> ExactSizeIterator for ValuesMut<'a, T> {}

impl<'a, T> FusedIterator for ValuesMut<'a, T> {}

#[derive(Debug)]
pub struct IntoValues<T> {
    inner: hash_map::IntoIter<TypeId, Item<T>>,
}
impl<T> Iterator for IntoValues<T> {
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        let (_key, item) = self.inner.next()?;
        Some(item.value)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }
}

impl<T> ExactSizeIterator for IntoValues<T> {}

impl<T> FusedIterator for IntoValues<T> {}

impl<T> IntoIterator for TypeIdMap<T> {
    type Item = T;
    type IntoIter = IntoValues<T>;

    fn into_iter(self) -> Self::IntoIter {
        IntoValues {
            inner: self.inner.into_iter(),
        }
    }
}

impl<'a, T> IntoIterator for &'a TypeIdMap<T> {
    type Item = &'a T;
    type IntoIter = Values<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.values()
    }
}

impl<'a, T> IntoIterator for &'a mut TypeIdMap<T> {
    type Item = &'a mut T;
    type IntoIter = ValuesMut<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.values_mut()
    }
}