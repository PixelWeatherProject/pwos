pub mod battery;
#[cfg(debug_assertions)]
pub mod brownout;
mod error;
pub mod ext_drivers;
pub mod gpio;
pub mod ledctl;
pub mod logging;
mod macros;
pub mod net;
pub mod ota;
pub mod power;
pub mod usbctl;

use std::{borrow::Cow, ffi::CStr, ptr::NonNull};

pub use error::{OsError, ReportableError};
pub type OsResult<T> = ::std::result::Result<T, OsError>;

/// Try and get the ESP-IDF version.
pub fn get_idf_version() -> Option<Cow<'static, str>> {
    let version_str_ptr =
        NonNull::new(unsafe { esp_idf_svc::sys::esp_get_idf_version().cast_mut() })?;
    let version_str = unsafe { CStr::from_ptr(version_str_ptr.as_ptr().cast_const()) };

    Some(version_str.to_string_lossy())
}
