use crate::os_warn;
use esp_idf_svc::sys::EspError;
use std::fmt::Display;
use thiserror::Error;

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

    /// Specified ESSID was too long. (>=32 chars)
    #[error("SSID too long")]
    SsidTooLong,

    /// Specified PSK was too long.
    #[error("PSK too long")]
    PskTooLong,

    /// Partition metadata contains an invalid version string.
    #[error("Unexpected version format")]
    IllegalFirmwareVersion,

    /// Failiure during conversions to/from [`Decimal`](pwmp_client::pwmp_msg::Decimal).
    #[error("Conversion beteen float/int and Decimal failed")]
    DecimalConversion,

    /// Invalid battery voltage result.
    #[error("Invalid battery voltage")]
    IllegalBatteryVoltage,

    /// Invalid partition metadata.
    #[error("Invalid/unexpected partition metadata")]
    IllegalPartitionMeta,
}

pub trait ReportableError {
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
    pub const fn recoverable(&self) -> bool {
        matches!(
            self,
            Self::WifiConnect(..) | Self::NoInternet | Self::PwmpError(..)
        )
    }
}
