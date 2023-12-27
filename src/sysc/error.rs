use esp_idf_svc::sys::EspError;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum OsError {
    #[error("wifi init")]
    WifiInit(i32),
    #[error("wifi blocking driver")]
    WifiBlockingInit(i32),
    #[error("wifi configuration")]
    WifiConfig(i32),
    #[error("wifi start")]
    WifiStart(i32),
    #[error("wifi connect")]
    WifiConnect(i32),
    #[error("offline")]
    NoInternet,
    #[error("pwmp")]
    PwmpError(pwmp_client::error::Error),
    #[error("pwmp server reject")]
    PwmpRejected,
    #[error("environment sensor")]
    NoEnvSensor,
    #[error("esp api")]
    Esp(#[from] EspError),
}

#[macro_export]
macro_rules! wrap_oserr {
    ($e: expr, $variant: ident) => {
        $e.map_err(|err| OsError::$variant(err.code()))
    };
}

impl From<pwmp_client::error::Error> for OsError {
    fn from(value: pwmp_client::error::Error) -> Self {
        use pwmp_client::error::Error;

        if matches!(value, Error::Rejected) {
            Self::PwmpRejected
        } else {
            Self::PwmpError(value)
        }
    }
}

impl OsError {
    pub const fn recoverable(&self) -> bool {
        matches!(
            self,
            Self::WifiInit(..)
                | Self::WifiBlockingInit(..)
                | Self::WifiConfig(..)
                | Self::WifiStart(..)
                | Self::WifiConnect(..)
                | Self::NoInternet
                | Self::PwmpError(..)
        )
    }
}
