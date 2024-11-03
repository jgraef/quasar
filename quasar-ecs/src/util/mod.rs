pub mod bit_set;
pub mod blob_vec;
pub mod sparse_map;
pub mod sparse_set;
pub mod type_id_map;

use std::{
    fmt::{Debug, Display},
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

pub struct Joined<'a, T> {
    sep: &'a str,
    parts: &'a [T],
}

impl<'a, T> Joined<'a, T> {
    pub fn new(sep: &'a str, parts: &'a [T]) -> Self {
        Self { sep, parts }
    }

    fn format(&self, formatter: &mut std::fmt::Formatter, display: impl Fn(&T, &mut std::fmt::Formatter) -> std::fmt::Result) -> std::fmt::Result {
        let mut iter = self.parts.iter();
        if let Some(first) = iter.next() {
            display(first, formatter)?;

            while let Some(next) = iter.next() {
                write!(formatter, "{}", self.sep)?;
                display(next, formatter)?;
            }
        }
        Ok(())
    }
}

impl<'a, T: Display> Display for Joined<'a, T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.format(f, <T as Display>::fmt)
    }
}

impl<'a, T: Debug> Debug for Joined<'a, T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.format(f, <T as Debug>::fmt)
    }
}

pub fn slice_get_mut_pair<'a, T>(
    slice: &'a mut [T],
    first: usize,
    second: usize,
) -> Result<(&'a mut T, &'a mut T), &'a mut T> {
    if first == second {
        Err(&mut slice[first])
    }
    else {
        let (left, right) = slice.split_at_mut(second);
        Ok((&mut left[first], &mut right[0]))
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
