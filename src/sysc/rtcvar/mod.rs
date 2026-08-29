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
//! Internally, this wrapper assumes that the value is initialized and safe to read when the following
//! conditions are met in order:
//! 1. The reset reason is a wakeup from deep sleep or a software reset,
//! 2. The stored magic byte is correct,
//! 3. The stored checksum equals the checksum calculated based on the currently stored value.
//!
//! These checks are not 100% accurate, but for the use cases in this firmware, it's good enough.
//!
//! # Supported types (`T`s)
//! This wrapper is primarily designed to hold simple primitives, like integers, `bool`s, integer arrays
//! and simple `struct`s. It is **not** designed to hold complex types such as dynamic `String`s, `Box`es
//! and `Vec`s. Such types will never be supported.
//!
//! The type `T` must implement [`RtcObject`], which further requires `T` to implement:
//! - [`Sized`],
//! - [`Copy`],
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
use std::{cell::UnsafeCell, mem::MaybeUninit};

mod checksum_impls;

const MAGIC_START_BYTE: u32 = 0xFEED_BEEF;

/// A wrapper that simplifies reading and writing variables stored in RTC memory.
pub struct RtcValue<T: RtcObject>(UnsafeCell<MaybeUninit<Inner<T>>>);
/// Wraps the internal content of the RTC object.
#[repr(C)]
struct Inner<T> {
    /// Magic starting byte
    magic: u32,

    /// CRC32
    crc32: u32,

    /// The actual value
    value: MaybeUninit<T>,
}

impl<T: RtcObject> RtcValue<T> {
    /// Creates a new instance. This method only exists because the Rust syntax requires specifying
    /// a value/constructor call when defining a `static`.
    ///
    /// ## Warning
    /// This method is never executed!
    pub const fn new() -> Self {
        Self(UnsafeCell::new(MaybeUninit::uninit()))
    }

    /// Reads the underlying value, initializing it when necessary.
    ///
    /// Initialization sets the value to `T::new_empty()` before returning.
    pub fn read(&self) -> T {
        if !self.is_init() {
            let value = T::new_empty();
            self.set(value);
            return value;
        }

        self.read_raw()
    }

    /// Overwites the current value with the specified one.
    ///
    /// This will also mark the value as initialized and safe to read afterwards.
    pub fn set(&self, val: T) {
        let checksum = val.checksum();

        self.write_magic(0);
        self.write_raw(val);
        self.write_crc(checksum);
        self.write_magic(MAGIC_START_BYTE);
    }

    /// Returns whether the value has been initialized before.
    ///
    /// Read module-level documentation on what checks are performed.
    fn is_init(&self) -> bool {
        let reset_reason_is_safe = matches!(
            power::get_reset_reason(),
            ResetReason::DeepSleep | ResetReason::Software | ResetReason::USBPeripheral
        );

        if !reset_reason_is_safe {
            return false;
        }

        if self.read_magic() != MAGIC_START_BYTE {
            return false;
        }

        let stored_checksum = self.read_crc();
        let value = self.read_raw();
        let calculated_checksum = value.checksum();

        stored_checksum == calculated_checksum
    }

    /// Returns a mutable pointer to the inner storage.
    const fn inner_ptr(&self) -> *mut Inner<T> {
        // `MaybeUninit<Inner<T>>` and `Inner<T>` have identical layout.
        self.0.get().cast::<Inner<T>>()
    }

    /// Reads the stored magic byte.
    fn read_magic(&self) -> u32 {
        // SAFETY: `&raw mut` projects to the field without creating a
        // reference to uninitialized memory; the read is volatile so the
        // optimizer cannot fold it against the static's initializer.
        unsafe { (&raw mut (*self.inner_ptr()).magic).read_volatile() }
    }

    /// Stores a new magic byte.
    fn write_magic(&self, val: u32) {
        // SAFETY: Writes are safe.
        unsafe { (&raw mut (*self.inner_ptr()).magic).write_volatile(val) }
    }

    /// Reads the stored CRC32 checksum.
    fn read_crc(&self) -> u32 {
        // SAFETY: field projection via `&raw mut`, no reference to uninit memory;
        // volatile so it can't be folded against the static's initializer.
        unsafe { (&raw mut (*self.inner_ptr()).crc32).read_volatile() }
    }

    /// Stores a new CRC32 checksum.
    fn write_crc(&self, val: u32) {
        // SAFETY: Writes are safe.
        unsafe { (&raw mut (*self.inner_ptr()).crc32).write_volatile(val) }
    }

    /// Reads the stored value of `T` **without performing any safety checks**.
    ///
    /// This should only be used after the validation checks have been performed.
    fn read_raw(&self) -> T {
        // SAFETY: If this method is called after validation checks have been performed, then
        //         it's safe. Otherwise it's UB and the returned value will have garbage data.
        unsafe {
            (&raw mut (*self.inner_ptr()).value)
                .cast::<T>()
                .read_volatile()
        }
    }

    /// Stores a new value in the RTC memory.
    fn write_raw(&self, val: T) {
        // SAFETY: Writes are safe.
        unsafe {
            (&raw mut (*self.inner_ptr()).value)
                .cast::<T>()
                .write_volatile(val);
        }
    }
}

/// A trait for objects that are safe and possible to store with [`RtcValue`].
pub trait RtcObject: Sized + Send + Copy {
    /// Calculate the checksum of `T`.
    fn checksum(&self) -> u32;

    /// Create a new, valid, initialized instance of `T`.
    ///
    /// For types that implement [`Default`], this should return `T::default()`.
    /// For others, a custom implementation is recommended.
    fn new_empty() -> Self;
}

unsafe impl<T: RtcObject> Send for RtcValue<T> {}
unsafe impl<T: RtcObject> Sync for RtcValue<T> {}
