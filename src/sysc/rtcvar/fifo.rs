use crate::sysc::rtcvar::{array::RtcArray, RtcObject};

/// A bounded FIFO queue that drops the oldest item when full
#[derive(Clone, Copy)]
pub struct RtcFifo<T: RtcObject, const N: usize>(RtcArray<T, N>);

impl<T: RtcObject, const N: usize> RtcFifo<T, N> {
    pub const fn const_new(default_value: T) -> Self {
        Self(RtcArray::const_new(default_value))
    }

    pub fn push(&mut self, value: T) {
        if self.0.len() == N {
            self.0.rotate_left(1);
            self.0.pop();
        }

        // SAFETY: At least one slot is always available.
        unsafe { self.0.push(value).unwrap_unchecked() };
    }

    pub fn is_empy(&self) -> bool {
        self.0.is_empty()
    }

    pub fn collect(&self) -> Box<[T]> {
        self.0.collect()
    }

    pub const fn clear(&mut self) {
        self.0.clear();
    }

    pub fn crc32_hash(&self) -> u32 {
        self.0.crc32_hash()
    }
}
