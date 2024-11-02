pub mod bit_set;
pub mod blob_vec;
pub mod sparse_map;
pub mod sparse_set;
pub mod type_id_map;

use std::{
    fmt::Display,
    mem::ManuallyDrop,
};

use bevy_ptr::OwningPtr;

pub type DropFn = unsafe fn(OwningPtr<'_>);

#[derive(Debug)]
pub struct OnDrop<F: FnOnce()> {
    callback: ManuallyDrop<F>,
}

impl<F: FnOnce()> OnDrop<F> {
    pub fn new(callback: F) -> Self {
        Self {
            callback: ManuallyDrop::new(callback),
        }
    }
}

impl<F: FnOnce()> Drop for OnDrop<F> {
    fn drop(&mut self) {
        let callback = unsafe { ManuallyDrop::take(&mut self.callback) };
        callback();
    }
}

pub unsafe fn drop_ptr<T>(x: OwningPtr<'_>) {
    // SAFETY: Contract is required to be upheld by the caller.
    unsafe {
        x.drop_as::<T>();
    }
}

pub fn partition_dedup<T: PartialEq>(slice: &mut [T]) -> (&mut [T], &mut [T]) {
    if slice.is_empty() {
        (slice, &mut [])
    }
    else {
        let mut remaining = slice.len() - 1;
        let mut read = 1;
        let mut write = 1;

        while remaining > 0 {
            if slice[read] != slice[write - 1] {
                if write != read {
                    slice.swap(write, read);
                }
                write += 1;
            }
            read += 1;
            remaining -= 1;
        }

        slice.split_at_mut(write)
    }
}

pub struct Joined<'a> {
    sep: &'a str,
    parts: &'a [&'a str],
}

impl<'a> Joined<'a> {
    pub fn new(sep: &'a str, parts: &'a [&'a str]) -> Self {
        Self { sep, parts }
    }
}

impl<'a> Display for Joined<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut iter = self.parts.iter();
        if let Some(first) = iter.next() {
            write!(f, "{first}")?;

            while let Some(next) = iter.next() {
                write!(f, "{}", self.sep)?;
                write!(f, "{next}")?;
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::util::partition_dedup;

    #[test]
    fn it_dedups_correctly() {
        let mut input = [1, 2, 2, 3, 4];
        let (left, right) = partition_dedup(&mut input);
        assert_eq!(left, [1, 2, 3, 4]);
        assert_eq!(right, [2]);
        assert_eq!(input, [1, 2, 3, 4, 2]);

        let mut input = [1, 2, 2, 2, 3, 4];
        let (left, right) = partition_dedup(&mut input);
        assert_eq!(left, [1, 2, 3, 4]);
        assert_eq!(right, [2, 2]);
        assert_eq!(input, [1, 2, 3, 4, 2, 2]);

        let mut input = [1, 2, 2, 3, 4, 4];
        let (left, right) = partition_dedup(&mut input);
        assert_eq!(left, [1, 2, 3, 4]);
        assert_eq!(right, [2, 4]);
        assert_eq!(input, [1, 2, 3, 4, 2, 4]);
    }
}
