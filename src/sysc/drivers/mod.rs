mod envsensor_trait;
mod htu;

use super::OsResult;
pub use envsensor_trait::EnvironmentSensor;
pub use htu::Htu;
use pwmp_client::pwmp_types::aliases::{AirPressure, Humidity, Temperature};

pub enum AnySensor<'s> {
    HtuCompatible(Htu<'s>),
    // add future sensors here...
}

pub struct MeasurementResults {
    pub temperature: Temperature,
    pub humidity: Humidity,
    pub air_pressure: Option<AirPressure>,
}

impl<'s> EnvironmentSensor for AnySensor<'s> {
    fn connected(&mut self) -> OsResult<bool> {
        match self {
            Self::HtuCompatible(dev) => dev.connected(),
        }
    }

    fn read_temperature(&mut self) -> OsResult<Temperature> {
        match self {
            Self::HtuCompatible(dev) => dev.read_temperature(),
        }
    }

    fn read_humidity(&mut self) -> OsResult<Humidity> {
        match self {
            Self::HtuCompatible(dev) => dev.read_humidity(),
        }
    }

    fn read_air_pressure(&mut self) -> OsResult<Option<AirPressure>> {
        match self {
            Self::HtuCompatible(dev) => dev.read_air_pressure(),
        }
    }
}
