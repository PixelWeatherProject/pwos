pub mod battery;
#[cfg(debug_assertions)]
pub mod brownout;
mod error;
pub mod ext_drivers;
pub mod ledctl;
pub mod logging;
mod macros;
pub mod net;
pub mod ota;
pub mod periph;
pub mod power;
pub mod usbctl;

pub use error::{OsError, ReportableError};
pub type OsResult<T> = ::std::result::Result<T, OsError>;

#[cfg(debug_assertions)]
/// Try and get the ESP-IDF version.
pub fn get_idf_version() -> Option<std::borrow::Cow<'static, str>> {
    use std::{ffi::CStr, ptr::NonNull};

    let version_str_ptr =
        NonNull::new(unsafe { esp_idf_svc::sys::esp_get_idf_version().cast_mut() })?;
    let version_str = unsafe { CStr::from_ptr(version_str_ptr.as_ptr().cast_const()) };

    Some(version_str.to_string_lossy())
}
