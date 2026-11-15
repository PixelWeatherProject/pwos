//! Provides an RTC-memory safe and compatible array type.

use crate::sysc::{hash, rtcvar::RtcObject};
use std::ops::Deref;

/// An array that can be safely stored in RTC memory.
///
/// This is a statically sized array, as dynamic resizing is not possible
/// in RTC memory without a specialized allocator.
#[derive(Debug, Clone, Copy)]
pub struct RtcArray<T: RtcObject, const N: usize> {
    /// The internal array holding the data
    inner: [T; N],

    /// Length of the array
    len: usize,
}

impl<T: RtcObject, const N: usize> RtcArray<T, N> {
    /// Creates a new zero-length array.
    pub const fn const_new(default_value: T) -> Self {
        Self {
            inner: [default_value; N],
            len: 0,
        }
    }

    /// Pushes a new value to the end of the array.
    pub const fn push(&mut self, value: T) -> Result<(), ()> {
        if self.len() >= N {
            return Err(());
        }

        self.inner[self.len] = value;
        self.len += 1;

        Ok(())
    }

    /// Removes the item at `index` from the array.
    ///
    /// # Performance note
    /// This operation may be slow on large arrays because shifting is needed.
    pub const fn remove(&mut self, index: usize) {
        if index >= self.len() {
            return;
        }

        let mut current = index;
        while current + 1 < self.len {
            self.inner[current] = self.inner[current + 1];
            current += 1;
        }

        self.len -= 1;
    }

    /// Returns the current length of the array.
    pub const fn len(&self) -> usize {
        self.len
    }

    /// Computes a CRC32 hash based on the contents of the array.
    ///
    /// _This calculation takes into account unused slots of `T`._
    pub fn crc32_hash(&self) -> u32 {
        // Fold `len` and the position of each item in, so length corruption
        // and reordering are detected.
        let mut hash = hash::crc32(&self.len.to_be_bytes());
        for item in &self.inner {
            hash = hash.rotate_left(5) ^ item.checksum();
        }

        hash
    }
}

impl<T: RtcObject, const N: usize> Deref for RtcArray<T, N> {
    type Target = [T];

    fn deref(&self) -> &Self::Target {
        &self.inner[..self.len]
    }
}
