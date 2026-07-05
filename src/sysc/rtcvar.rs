//! A safe wrapper for `static`s that are stored in uninitialized RTC memory.
//!
//! # Usage
//! This wrapper can only be used as follows:
//! ```rust
//! #[link_section = ".rtc_noinit"]
//! static TESTING: RtcValue<T> = RtcValue::new();
//! ```
//!
//! You must link the location of this variable to `.rtc_noinit`!
//!
//! Note that while the constructor [`new()`](RtcValue::new) is specified in the example
//! above, **it never actually runs**. The section `.rtc_noinit` does not handle initialization.
//!
//! # Safety
//! Internally, this wrapper assumes that the value is initialized and safe to read
//! if the reset reason is a wakeup from deep sleep or a software reset. This is somewhat
//! aggressive but also not fully safe. There is no checksum implemented, nor any other
//! secondary mechanism to check if the value is truly safe to read.
//!
//! For the use cases in this firmware, this is enough. A CRC32 checksum might be added in
//! the future.
//!
//! ## Possible undefined behaviour #1
//! ```rust
//! static TESTING: RtcValue<u8> = RtcValue::new();
//!
//! let check = TESTING.is_init(); // marks the value as initialized
//!
//! /* ... device reboots with a reset reason that is marked as safe ... */
//!
//! let value = TESTING.read(); // this will read from uninitialized memory.
//! ```
//!
//! This is currently not possible because [`is_init()`](RtcValue::is_init) is private, but
//! at any point it needs to be made public, this is something to keep in mind.
//!
//! # Supported types (`T`s)
//! The type `T` must implement [`RtcObject`], which further requires `T` to implement:
//! - [`Sized`],
//! - [`Default`],
//! - [`Send`].
//!
//! [`Send`] and [`Sync`] are implemented for [`RtcValue`] with any `T`.

use crate::sysc::power;
use esp_idf_svc::hal::reset::ResetReason;
use pwmp_client::pwmp_msg::settings::NodeSettings;
use std::mem::MaybeUninit;

/// A wrapper that simplifies reading and writing variables stored in RTC memory.
pub struct RtcValue<T: RtcObject> {
    /// The actual value
    value: MaybeUninit<T>,
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
            is_valid: MaybeUninit::uninit(),
        }
    }

    /// Reads the underlying value, initializing it when necessary.
    ///
    /// Initialization sets the value to `T::default()` before returning.
    pub fn read(&self) -> T {
        if !self.is_init() {
            self.set(T::default());
        }

        unsafe { self.value.assume_init_read() }
    }

    /// Overwites the current value with the specified one.
    ///
    /// This will also mark the value as initialized and safe to read afterwards.
    pub fn set(&self, val: T) {
        // SAFETY: Writing to uninitialized MaybeUninit<T> is safe
        unsafe {
            // We use pointer writes to avoid having to use `&mut self` in the method
            self.value.as_ptr().cast_mut().write_volatile(val);
            self.is_valid.as_ptr().cast_mut().write_volatile(true);
        }
    }

    /// Returns whether the value has been initialized before.
    ///
    /// If the reset reason is not a deep-sleep wakeup or a software reset, this will
    /// always return `false`. Otherwise, `true` is returned.
    ///
    /// The return value is always saved into [`Self::is_valid`].
    fn is_init(&self) -> bool {
        let is_valid = matches!(
            power::get_reset_reason(),
            ResetReason::DeepSleep | ResetReason::Software | ResetReason::USBPeripheral
        );

        // SAFETY: Writing to uninitialized MaybeUninit<T> is safe
        unsafe {
            // We use pointer writes to avoid having to use `&mut self` in the method
            self.is_valid.as_ptr().cast_mut().write_volatile(is_valid);
        }

        is_valid
    }
}

/// A trait for objects that are safe and possible to store with [`RtcValue`].
pub trait RtcObject: Sized + Default + Send {}

impl RtcObject for bool {}
impl RtcObject for u8 {}
impl RtcObject for NodeSettings {}

unsafe impl<T: RtcObject> Send for RtcValue<T> {}
unsafe impl<T: RtcObject> Sync for RtcValue<T> {}
