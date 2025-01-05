pub mod battery;
pub mod brownout;
mod error;
pub mod ext_drivers;
pub mod ledctl;
pub mod logging;
pub mod net;
pub mod sleep;
pub mod usbctl;
pub mod verification;

pub use error::{OsError, ReportableError};
pub type OsResult<T> = ::std::result::Result<T, OsError>;
