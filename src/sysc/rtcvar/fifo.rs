//! A bounded FIFO queue that can be stored in RTC memory.
//!
//! When the queue is full, pushing a new item drops the oldest one, so the queue
//! always holds the `N` most recent items. Items are always stored oldest-first,
//! so [`collect()`](RtcFifo::collect) returns them in chronological order.
//!
//! Pushing into a full queue shifts every item down by one slot, which costs O(N).
//! This is fine for small capacities that are written rarely, such as once per wakeup.

use crate::sysc::rtcvar::{array::RtcArray, RtcObject};

/// A bounded FIFO queue that drops the oldest item when full.
///
/// When the queue is full, pushing a new item drops the oldest one, so the queue
/// always holds the `N` most recent items. Items are always stored oldest-first,
/// so [`collect()`](RtcFifo::collect) returns them in chronological order.
///
/// Pushing into a full queue shifts every item down by one slot, which costs O(N).
/// This is fine for small capacities that are written rarely, such as once per wakeup.
#[derive(Clone, Copy)]
pub struct RtcFifo<T: RtcObject, const N: usize>(RtcArray<T, N>);

impl<T: RtcObject, const N: usize> RtcFifo<T, N> {
    /// Creates a new buffer with the provided default value.
    pub const fn const_new(default_value: T) -> Self {
        Self(RtcArray::const_new(default_value))
    }

    /// Push a new value into the buffer.
    pub fn push(&mut self, value: T) {
        if self.0.len() == N {
            self.0.rotate_left(1);
            self.0.pop();
        }

        // SAFETY: At least one slot is always available.
        unsafe { self.0.push(value).unwrap_unchecked() };
    }

    /// Returns whether the buffer is empty or not.
    pub fn is_empy(&self) -> bool {
        self.0.is_empty()
    }

    /// Collects all items in the buffer into a new array in heap.
    pub fn collect(&self) -> Box<[T]> {
        self.0.collect()
    }

    /// Clear the buffer.
    pub const fn clear(&mut self) {
        self.0.clear();
    }

    /// Computes a CRC32 hash of the buffer.
    pub fn crc32_hash(&self) -> u32 {
        self.0.crc32_hash()
    }
}
