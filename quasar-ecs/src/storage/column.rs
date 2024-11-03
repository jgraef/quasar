use std::{
    cell::UnsafeCell,
    marker::PhantomData,
    ops::{
        Deref,
        DerefMut,
    },
};

use bevy_ptr::OwningPtr;

use crate::{
    component::ComponentDescriptor,
    util::blob_vec::BlobVec,
};

#[derive(Debug)]
pub struct Column {
    data: BlobVec,
}

impl Column {
    pub fn new(component_descriptor: &ComponentDescriptor, capacity: usize) -> Self {
        Self {
            data: unsafe {
                // SAFETY: the components stored in this BlobVec will match the
                // ComponentDescriptor
                BlobVec::new(
                    component_descriptor.layout(),
                    component_descriptor.drop_fn(),
                    capacity,
                )
            },
        }
    }

    pub fn reserve(&mut self, additional: usize) {
        self.data.reserve(additional);
    }

    pub fn clear(&mut self) {
        self.data.clear();
    }

    pub fn len(&self) -> usize {
        self.data.len()
    }

    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    pub unsafe fn get_slice_unsafe<T>(&self) -> &[UnsafeCell<T>] {
        self.data.get_slice()
    }

    pub unsafe fn get_slice<T>(&self) -> &[T] {
        self.data.get_slice()
    }

    pub unsafe fn get_mut_slice<T>(&mut self) -> &mut [T] {
        self.data.get_mut_slice()
    }

    pub unsafe fn push<T>(&mut self, value: T) {
        OwningPtr::make(value, |ptr| {
            self.data.push(ptr);
        });
    }

    pub unsafe fn move_item(&mut self, index: usize, to_column: &mut Self) {
        let ptr = self.data.swap_remove_and_forget_unchecked(index);
        to_column.push(ptr);
    }

    pub unsafe fn remove_item(&mut self, index: usize) {
        self.data.swap_remove_and_drop_unchecked(index);
    }
}
