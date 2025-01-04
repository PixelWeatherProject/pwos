use crate::os_warn;
use esp_idf_svc::sys::EspError;
use std::fmt::Display;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum OsError {
    #[error("wifi connect: {0}")]
    WifiConnect(EspError),
    #[error("offline")]
    NoInternet,
    #[error("pwmp: {0}")]
    PwmpError(#[from] pwmp_client::error::Error),
    #[error("environment sensor")]
    NoEnvSensor,
    #[error("esp api: {0}")]
    Esp(#[from] EspError),
    #[error("SSID too long")]
    SsidTooLong,
    #[error("PSK too long")]
    PskTooLong,
    #[error("Unexpected version format")]
    IllegalFirmwareVersion,
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
