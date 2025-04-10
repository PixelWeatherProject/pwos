//! Error types for the firmware.

use crate::os_warn;
use esp_idf_svc::sys::EspError;
use std::fmt::Display;
use thiserror::Error;

/// A wrapper for several different lower-level error types.
#[allow(clippy::doc_markdown)]
#[derive(Debug, Error)]
pub enum OsError {
    /// Failed to connect to WiFi AP.
    #[error("wifi connect: {0}")]
    WifiConnect(EspError),

    /// No usable AP found.
    #[error("offline")]
    NoInternet,

    /// Error during connecting to or processing data from the PWMP server.
    #[error("pwmp: {0}")]
    PwmpError(#[from] pwmp_client::error::Error),

    /// No environment sensor detected.
    #[error("environment sensor")]
    NoEnvSensor,

    /// Generic ESP-IDF error.
    #[error("esp api: {0}")]
    Esp(#[from] EspError),

    /// Specified parameter was too long.
    #[error("Argument too long")]
    ArgumentTooLong,

    /// Partition metadata contains an invalid version string.
    #[error("Unexpected version format")]
    IllegalFirmwareVersion,

    /// Partition metadata is missing.
    #[error("Unexpected version format")]
    MissingPartitionMetadata,

    /// Failiure during conversions to/from [`Decimal`](pwmp_client::pwmp_msg::Decimal).
    #[error("Conversion between float/int and Decimal failed")]
    DecimalConversion,

    /// A buffer has been filled unexpectedly
    #[error("A buffer capacity has been exceeded")]
    UnexpectedBufferFailiure,

    /// A value was `None`, when `Some(..)` was expected.
    #[error("Unexpected NULL")]
    UnexpectedNull,
}

/// Trait for non-fatal error types that can be "reported" to the console.
///
/// This trait is meant to be implemented for [`Result`](Result)s.
pub trait ReportableError {
    /// Log a warning to the console if the [`Result`] variant is an [`Err`], or do nothing if it's [`Ok`].
    fn report(self, desc: &str);
}

impl<T, E: Display> ReportableError for Result<T, E> {
    fn report(self, desc: &str) {
        if let Err(why) = self {
            os_warn!("{desc}: {why}");
        }
    }
}

impl OsError {
    /// Returns whether the error is non-fatal.
    ///
    /// This returns `true`, for errors related to WiFi/Internet and PWNP connectivity/IO issues.
    pub const fn recoverable(&self) -> bool {
        matches!(
            self,
            Self::WifiConnect(..) | Self::NoInternet | Self::PwmpError(..)
        )
    }
}
