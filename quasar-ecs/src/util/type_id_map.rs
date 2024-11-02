use std::{
    any::TypeId,
    collections::{
        hash_map,
        HashMap,
    },
};

#[derive(Clone, Debug)]
pub struct TypeIdMap<T> {
    inner: HashMap<TypeId, T>,
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

    pub fn iter(&self) -> impl Iterator<Item = (TypeId, &T)> {
        self.inner.iter().map(|(key, value)| (*key, value))
    }

    pub fn clear(&mut self) {
        self.inner.clear();
    }

    pub fn insert<K: 'static>(&mut self, value: T) {
        self.inner.insert(TypeId::of::<K>(), value);
    }

    pub fn get<K: 'static>(&self) -> Option<&T> {
        self.inner.get(&TypeId::of::<K>())
    }

    pub fn get_mut<K: 'static>(&mut self) -> Option<&mut T> {
        self.inner.get_mut(&TypeId::of::<K>())
    }

    pub fn entry<K: 'static>(&mut self) -> hash_map::Entry<TypeId, T> {
        self.inner.entry(TypeId::of::<K>())
    }
}
