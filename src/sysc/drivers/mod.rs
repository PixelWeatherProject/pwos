mod envsensor_trait;
mod fakedev;
mod htu21d;
mod si7021;

use super::OsResult;
pub use envsensor_trait::EnvironmentSensor;
pub use fakedev::FakeEnvSensor;
pub use htu21d::Htu21d;
use pwmp_client::pwmp_types::aliases::{AirPressure, Humidity, Temperature};
pub use si7021::Si7021;

pub enum AnySensor<'s> {
    Si7021(Si7021<'s>),
    Htu21d(Htu21d<'s>),
    Fake(FakeEnvSensor),
}

pub struct MeasurementResults {
    pub temperature: Temperature,
    pub humidity: Humidity,
    pub air_pressure: Option<AirPressure>,
}

impl<'s> EnvironmentSensor for AnySensor<'s> {
    fn connected(&mut self) -> OsResult<bool> {
        match self {
            Self::Si7021(dev) => dev.connected(),
            Self::Htu21d(dev) => dev.connected(),
            Self::Fake(dev) => dev.connected(),
        }
    }

    fn read_temperature(&mut self) -> OsResult<Temperature> {
        match self {
            Self::Si7021(dev) => dev.read_temperature(),
            Self::Htu21d(dev) => dev.read_temperature(),
            Self::Fake(dev) => dev.read_temperature(),
        }
    }

    fn read_humidity(&mut self) -> OsResult<Humidity> {
        match self {
            Self::Si7021(dev) => dev.read_humidity(),
            Self::Htu21d(dev) => dev.read_humidity(),
            Self::Fake(dev) => dev.read_humidity(),
        }
    }

    fn read_air_pressure(&mut self) -> Option<OsResult<AirPressure>> {
        match self {
            Self::Si7021(dev) => dev.read_air_pressure(),
            Self::Htu21d(dev) => dev.read_air_pressure(),
            Self::Fake(dev) => dev.read_air_pressure(),
        }
    }
}
