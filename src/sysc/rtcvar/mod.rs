//! A safe wrapper for `static`s that are stored in uninitialized RTC memory.
//!
//! # Usage
//! This wrapper can only be used as follows:
//! ```rust
//! #[link_section = ".rtc_noinit"]
//! static MY_DATA: RtcValue<T> = RtcValue::new();
//! ```
//!
//! You must link the location of this variable to `.rtc_noinit`!
//!
//! Note that while the constructor [`new()`](RtcValue::new) is specified in the example
//! above, **it never actually runs**. The section `.rtc_noinit` does not handle initialization.
//!
//! # Safety
//! Internally, this wrapper assumes that the value is initialized and safe to read
//! if the reset reason is a wakeup from deep sleep or a software reset, **and** the stored CRC32
//! matches the CRC32 calculated at runtime. This is not a 100% bulletproof solution, but for
//! the use cases in this firmware, it's good enough.
//!
//! ## Possible undefined behavior #1
//! ```rust
//! static MY_DATA: RtcValue<u8> = RtcValue::new();
//!
//! let check = MY_DATA.is_init(); // marks the value as initialized
//!
//! /* ... device reboots with a reset reason that is marked as safe ... */
//!
//! let value = MY_DATA.read(); // this will read from uninitialized memory.
//! ```
//!
//!
//! This is currently not possible because [`is_init()`](RtcValue::is_init) is private, but
//! at any point it needs to be made public, this is something to keep in mind.
//!
//! # Supported types (`T`s)
//! The type `T` must implement [`RtcObject`], which further requires `T` to implement:
//! - [`Sized`],
//! - [`Send`].
//!
//! Additionally, there must be a way to create new, valid and initialized instances of `T`.
//! Check the docs for [`RtcObject::new_empty()`] for details. Not requiring a [`Default`]
//! implementation for `T` makes it easier to implement [`RtcObject`] even if there is a way
//! to create a default value, but it's not implemented for `T`. Otherwise this would require
//! creating wrapper types, which would result in more code and potencial pain points with conversions.
//!
//! # Thread safety
//! This type is **not** thread-safe.

use crate::sysc::power;
use esp_idf_svc::hal::reset::ResetReason;
use std::mem::MaybeUninit;

mod checksum_impls;

/// A wrapper that simplifies reading and writing variables stored in RTC memory.
pub struct RtcValue<T: RtcObject> {
    /// The actual value
    value: MaybeUninit<T>,
    /// CRC32
    crc32: MaybeUninit<u32>,
    /// Whether `value` was ever initialized/set
    is_valid: MaybeUninit<bool>,
}

impl<T: RtcObject> RtcValue<T> {
    /// Creates a new instance. This method only exists because the Rust syntax requires specifying
    /// a value/constructor call when defining a `static`.
    ///
    /// ## Warning
    /// This method is never executed!
    pub const fn new() -> Self {
        Self {
            value: MaybeUninit::uninit(),
            crc32: MaybeUninit::uninit(),
            is_valid: MaybeUninit::uninit(),
        }
    }

    /// Reads the underlying value, initializing it when necessary.
    ///
    /// Initialization sets the value to `T::default()` before returning.
    pub fn read(&self) -> T {
        if !self.is_init() {
            self.set(T::new_empty());
        }

        unsafe { self.value.assume_init_read() }
    }

    /// Overwites the current value with the specified one.
    ///
    /// This will also mark the value as initialized and safe to read afterwards.
    pub fn set(&self, val: T) {
        self.set_without_mark(val);
        self.set_validity(true);
    }

    /// Returns whether the value has been initialized before.
    ///
    /// If the reset reason is not a deep-sleep wakeup or a software reset, this will
    /// always return `false`. Otherwise, `true` is returned.
    ///
    /// The return value is always saved into [`Self::is_valid`].
    fn is_init(&self) -> bool {
        let reset_reason_is_safe = matches!(
            power::get_reset_reason(),
            ResetReason::DeepSleep | ResetReason::Software | ResetReason::USBPeripheral
        );

        if !reset_reason_is_safe {
            self.set_validity(false);
            return false;
        }

        let value = unsafe { self.value.assume_init_read() };
        let stored_checksum = unsafe { self.crc32.assume_init_read() };
        let calculated_checksum = value.checksum();

        stored_checksum == calculated_checksum
    }

    fn set_validity(&self, valid: bool) {
        // SAFETY: Writing to uninitialized MaybeUninit<T> is safe
        unsafe {
            // We use pointer writes to avoid having to use `&mut self` in the method
            self.is_valid.as_ptr().cast_mut().write_volatile(valid);
        }
    }

    fn set_without_mark(&self, value: T) {
        let checksum = value.checksum();

        // SAFETY: Writing to uninitialized MaybeUninit<T> is safe
        unsafe {
            // We use pointer writes to avoid having to use `&mut self` in the method
            self.value.as_ptr().cast_mut().write_volatile(value);
            self.crc32.as_ptr().cast_mut().write_volatile(checksum);
        }

        self.set_validity(true);
    }
}

/// A trait for objects that are safe and possible to store with [`RtcValue`].
pub trait RtcObject: Sized + Send {
    /// Calculate the checksum of `T`.
    fn checksum(&self) -> u32;

    /// Create a new, valid, initialized instance of `T`.
    ///
    /// For types that implement [`Default`], this should return `T::default()`.
    /// For others, a custom implementation is recommended.
    fn new_empty() -> Self;
}
