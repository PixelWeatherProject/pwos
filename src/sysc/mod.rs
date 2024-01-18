pub mod battery;
pub mod brownout;
pub mod drivers;
mod error;
pub mod ledctl;
pub mod logging;
pub mod net;
pub mod sleep;

pub use error::{OsError, ReportableError};
pub type OsResult<T> = ::std::result::Result<T, OsError>;
