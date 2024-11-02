use std::fmt::Debug;

#[derive(Clone)]
pub struct BitSet<S> {
    words: Vec<S>,
    len: usize,
}

impl<S> BitSet<S> {
    pub fn new() -> Self {
        Self {
            words: vec![],
            len: 0,
        }
    }

    pub fn clear(&mut self) {
        self.words.clear();
        self.len = 0;
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }
}

impl<S: BitSetStorage> BitSet<S> {
    pub fn contains(&self, value: usize) -> bool {
        let (index, mask) = S::index_and_mask(value);
        self.words
            .get(index)
            .map_or(false, |word| word.contains(mask))
    }

    pub fn insert(&mut self, value: usize) {
        let (index, mask) = S::index_and_mask(value);
        self.words
            .resize_with(self.words.len().max(index + 1), Default::default);
        if !self.words[index].insert(mask) {
            self.len += 1;
        }
    }

    pub fn remove(&mut self, value: usize) {
        let (index, mask) = S::index_and_mask(value);
        if let Some(word) = self.words.get_mut(index) {
            if word.remove(mask) {
                self.len += 1;
            }
        }
    }

    pub fn iter(&self) -> Iter<S> {
        Iter {
            iter: IterImpl::new(self.words.iter().copied()),
        }
    }
}

impl<S> Default for BitSet<S> {
    fn default() -> Self {
        Self::new()
    }
}

impl<S: BitSetStorage> Debug for BitSet<S> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_set().entries(self.iter()).finish()
    }
}

impl<S: BitSetStorage> IntoIterator for BitSet<S> {
    type Item = usize;
    type IntoIter = IntoIter<S>;

    fn into_iter(self) -> Self::IntoIter {
        IntoIter {
            iter: IterImpl::new(self.words.into_iter()),
        }
    }
}

impl<'a, S: BitSetStorage> IntoIterator for &'a BitSet<S> {
    type Item = usize;
    type IntoIter = Iter<'a, S>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl<'a, S: BitSetStorage> FromIterator<usize> for BitSet<S> {
    fn from_iter<T: IntoIterator<Item = usize>>(iter: T) -> Self {
        let mut set = BitSet::default();

        for value in iter {
            set.insert(value);
        }

        set
    }
}

#[derive(Clone)]
pub struct ImmutableBitSet<S> {
    words: Box<[S]>,
    len: usize,
}

impl<S> ImmutableBitSet<S> {
    pub fn new() -> Self {
        Self {
            words: Default::default(),
            len: 0,
        }
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }
}

impl<S: BitSetStorage> ImmutableBitSet<S> {
    pub fn contains(&self, value: usize) -> bool {
        let (index, mask) = S::index_and_mask(value);
        self.words
            .get(index)
            .map_or(false, |word| word.contains(mask))
    }

    pub fn iter(&self) -> Iter<S> {
        Iter {
            iter: IterImpl::new(self.words.iter().copied()),
        }
    }
}

impl<S> Default for ImmutableBitSet<S> {
    fn default() -> Self {
        Self::new()
    }
}

impl<S: BitSetStorage> Debug for ImmutableBitSet<S> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_set().entries(self.iter()).finish()
    }
}

impl<S: BitSetStorage> IntoIterator for ImmutableBitSet<S> {
    type Item = usize;
    type IntoIter = IntoIter<S>;

    fn into_iter(self) -> Self::IntoIter {
        IntoIter {
            iter: IterImpl::new(self.words.into_vec().into_iter()),
        }
    }
}

impl<'a, S: BitSetStorage> IntoIterator for &'a ImmutableBitSet<S> {
    type Item = usize;
    type IntoIter = Iter<'a, S>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl<'a, S: BitSetStorage> FromIterator<usize> for ImmutableBitSet<S> {
    fn from_iter<T: IntoIterator<Item = usize>>(iter: T) -> Self {
        BitSet::from_iter(iter).into()
    }
}

impl<S> From<BitSet<S>> for ImmutableBitSet<S> {
    fn from(set: BitSet<S>) -> Self {
        Self {
            words: set.words.into(),
            len: set.len,
        }
    }
}

impl<S> From<ImmutableBitSet<S>> for BitSet<S> {
    fn from(set: ImmutableBitSet<S>) -> Self {
        Self {
            words: set.words.into(),
            len: set.len,
        }
    }
}

#[derive(Debug)]
struct IterImpl<W, S> {
    words: W,
    word: Option<S>,
    value: usize,
    masks: MaskIter<S>,
}

impl<W: Iterator<Item = S>, S: BitSetStorage> IterImpl<W, S> {
    pub fn new(mut words: W) -> Self {
        let word = words.next();
        IterImpl {
            words,
            word,
            value: 0,
            masks: S::mask_iter(),
        }
    }
}

impl<W: Iterator<Item = S>, S: BitSetStorage> Iterator for IterImpl<W, S> {
    type Item = usize;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let word = self.word?;
            if let Some(mask) = self.masks.next() {
                if word.contains(mask) {
                    return Some(self.value);
                }
                self.value += 1;
            }
            else {
                self.masks = S::mask_iter();
                self.word = self.words.next();
            }
        }
    }
}

#[derive(Debug)]
pub struct Iter<'a, S> {
    iter: IterImpl<std::iter::Copied<std::slice::Iter<'a, S>>, S>,
}

impl<'a, S: BitSetStorage> Iterator for Iter<'a, S> {
    type Item = usize;

    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next()
    }
}

#[derive(Debug)]
pub struct IntoIter<S> {
    iter: IterImpl<std::vec::IntoIter<S>, S>,
}

impl<'a, S: BitSetStorage> Iterator for IntoIter<S> {
    type Item = usize;

    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next()
    }
}

pub trait BitSetStorage: Copy + Default {
    fn capacity(capacity: usize) -> usize;
    fn index_and_mask(value: usize) -> (usize, Self);
    fn mask_iter() -> MaskIter<Self>;
    fn mask_iter_next(mask: &mut Self) -> Option<Self>;
    fn item_from_index_and_bit(index: usize, bit: usize) -> usize;
    fn insert(&mut self, mask: Self) -> bool;
    fn remove(&mut self, mask: Self) -> bool;
    fn contains(&self, mask: Self) -> bool;
}

macro_rules! impl_storage {
    ($ty:ty, $bits:expr) => {
        impl BitSetStorage for $ty {
            fn capacity(capacity: usize) -> usize {
                capacity.div_ceil($bits)
            }

            fn index_and_mask(value: usize) -> (usize, Self) {
                (value / $bits, 1 << (value % $bits))
            }

            fn mask_iter() -> MaskIter<Self> {
                MaskIter { mask: 1 }
            }

            fn mask_iter_next(mask: &mut Self) -> Option<Self> {
                (*mask != 0).then(|| {
                    *mask <<= 1;
                    *mask
                })
            }

            fn item_from_index_and_bit(index: usize, bit: usize) -> usize {
                index * $bits + bit
            }

            fn insert(&mut self, mask: Self) -> bool {
                let replaced = *self & mask != 0;
                *self |= mask;
                replaced
            }

            fn remove(&mut self, mask: Self) -> bool {
                let removed = *self & mask != 0;
                *self &= !mask;
                removed
            }

            fn contains(&self, mask: Self) -> bool {
                *self & mask != 0
            }
        }
    };
}

impl_storage!(u64, 64);

#[derive(Debug)]
struct MaskIter<S> {
    mask: S,
}

impl<S: BitSetStorage> Iterator for MaskIter<S> {
    type Item = S;

    fn next(&mut self) -> Option<Self::Item> {
        S::mask_iter_next(&mut self.mask)
    }
}
