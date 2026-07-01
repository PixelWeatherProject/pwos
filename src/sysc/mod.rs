pub mod battery;
#[cfg(debug_assertions)]
pub mod brownout;
mod error;
pub mod ext_drivers;
pub mod joined_writer;
pub mod ledctl;
pub mod logging;
mod macros;
pub mod net;
pub mod nvs;
pub mod ota;
pub mod panic;
pub mod periph;
pub mod power;
pub mod usbctl;

pub use error::{OsError, ReportableError};
pub type OsResult<T> = ::std::result::Result<T, OsError>;

#[cfg(debug_assertions)]
/// Try and get the ESP-IDF version.
pub fn get_idf_version() -> std::borrow::Cow<'static, str> {
    use std::ffi::CStr;

    // SAFETY: This is a static string generated during compilation by the ESP-IDF.
    let version = unsafe { CStr::from_ptr(esp_idf_svc::sys::esp_get_idf_version()) };

    version.to_string_lossy()
}
