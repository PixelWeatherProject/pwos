//! A ring buffer implementation that can be stored in RTC memory.

use crate::sysc::{
    hash,
    rtcvar::{array::RtcArray, RtcObject},
};
use std::ops::Deref;

/// A statically allocated ring buffer that is safe to store in RTC memory.
///
/// This type is backed by [`RtcArray`].
#[derive(Debug, Clone, Copy)]
pub struct RtcRingBuffer<T: RtcObject, const N: usize> {
    /// The backing array
    inner: RtcArray<T, N>,

    /// The current index at which the buffer will insert.
    current_index: usize,
}

impl<T: RtcObject, const N: usize> RtcRingBuffer<T, N> {
    /// Create a new empty ring buffer of `T`s.
    pub const fn const_new(default_value: T) -> Self {
        Self {
            inner: RtcArray::const_new(default_value),
            current_index: 0,
        }
    }

    /// Inserts new data to the buffer.
    pub const fn insert(&mut self, value: T) {
        if self.inner.len() < N {
            // still filling up
            let _ = self.inner.push(value);
        } else {
            // full: overwrite the oldest entry
            let _ = self.inner.replace(self.current_index, value);
        }
        self.current_index = (self.current_index + 1) % N;
    }

    /// Clears the buffer.
    pub const fn clear(&mut self) {
        self.inner.clear();
        self.current_index = 0;
    }

    /// Returns the amount of data stored by the buffer.
    pub const fn len(&self) -> usize {
        self.inner.len()
    }

    pub fn crc32_hash(&self) -> u32 {
        let idx = hash::crc32(&self.current_index.to_be_bytes());
        self.inner.crc32_hash().rotate_left(5) ^ idx
    }
}

impl<T: RtcObject, const N: usize> Deref for RtcRingBuffer<T, N> {
    type Target = [T];

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}
