//! Error types for the firmware.

use crate::os_warn;
use esp_idf_svc::sys::EspError;
use std::{fmt::Display, string::FromUtf8Error};
use thiserror::Error;

/// A wrapper for several different lower-level error types.
#[allow(clippy::doc_markdown)]
#[derive(Debug, Error)]
pub enum OsError {
    /// Failed to initialize WiFi.
    #[error("wifi init: {0}")]
    WifiInit(EspError),

    /// Failed to connect to WiFi AP.
    #[error("wifi connect: {0}")]
    WifiConnect(EspError),

    /// Failed to set up a WiFi parameter.
    #[error("wifi param: {0}")]
    WifiParam(EspError),

    /// Failed to set WiFi configuration.
    #[error("wifi config: {0}")]
    WifiConfig(EspError),

    /// Failed to start the WiFi interface.
    #[error("wifi start: {0}")]
    WifiStart(EspError),

    /// Failed to start AP scan or fetch the results.
    #[error("wifi scan: {0}")]
    WifiScan(EspError),

    /// Failed to read WiFi interface information.
    #[error("wifi info read: {0}")]
    WifiInfo(EspError),

    /// Timeout while waiting for an event.
    #[error("event timeout: {0}")]
    EventTimeout(EspError),

    /// Failed to initialize event waiter ([Wait](esp_idf_svc::eventloop::Wait)).
    #[error("event waiter: {0}")]
    EventWaiterInit(EspError),

    /// No usable AP found.
    #[error("offline")]
    NoInternet,

    /// Error during connecting to or processing data from the PWMP server.
    #[error("pwmp: {0}")]
    PwmpError(#[from] pwmp_client::error::Error),

    /// No environment sensor detected.
    #[error("environment sensor")]
    NoEnvSensor,

    /// OTA module or update initialization has failed.
    #[error("Failed to initialize an OTA update ({0})")]
    OtaInit(EspError),

    /// Failed to write an OTA update chunk to flash.
    #[error("OTA chunk write failed ({0})")]
    OtaWrite(EspError),

    /// Failed to abort OTA update.
    #[error("OTA abort failed ({0})")]
    OtaAbort(EspError),

    /// An I/O operation on an OTA slot failed.
    #[error("Failed to operate on an OTA slot ({0})")]
    OtaSlot(EspError),

    /// NVS initialization has failed
    #[error("Failed to initialize NVS ({0})")]
    NvsInit(EspError),

    /// Error while reading from NVS.
    #[error("Failed to read from NVS ({0})")]
    NvsRead(EspError),

    /// Error while writing to NVS.
    #[error("Failed to write to NVS ({0})")]
    NvsWrite(EspError),

    /// Failed to initialize a GPIO pin.
    #[error("Failed to initialize a GPIO pin ({0})")]
    GpioInit(EspError),

    /// Failed to initialize ADC.
    #[error("Failed to initialize a GPIO pin ({0})")]
    AdcInit(EspError),

    /// Failed to read from the ADC.
    #[error("Failed to initialize a GPIO pin ({0})")]
    AdcRead(EspError),

    /// Error while reading from I2C.
    #[error("Failed to read from I2C ({0})")]
    I2cRead(EspError),

    /// Error while writing to I2C.
    #[error("Failed to write to I2C ({0})")]
    I2cWrite(EspError),

    /// Specified parameter was too long.
    #[error("Argument too long")]
    ArgumentTooLong,

    /// Partition metadata contains an invalid version string.
    #[error("Unexpected version format")]
    IllegalFirmwareVersion,

    /// Partition metadata is missing.
    #[error("Unexpected version format")]
    MissingPartitionMetadata,

    /// A buffer has been filled unexpectedly
    #[error("A buffer capacity has been exceeded")]
    UnexpectedBufferFailiure,

    /// A value was `None`, when `Some(..)` was expected.
    #[error("Unexpected NULL")]
    UnexpectedNull,

    /// Expected a UTF-8 string.
    #[error("String is not UTF-8 encoded")]
    InvalidUtf8(#[from] FromUtf8Error),

    /// Key not found in NVS storage.
    #[error("NVS key does not exist")]
    InvalidNvsKey,
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
