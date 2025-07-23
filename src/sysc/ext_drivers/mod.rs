mod envsensor_trait;
mod htu;

use super::OsResult;
pub use envsensor_trait::EnvironmentSensor;
pub use htu::Htu;
use pwmp_client::pwmp_msg::aliases::{AirPressure, Humidity, Temperature};

/// A wrapper that allows abstracting the underlying sensor driver without the use of generics.
pub enum AnySensor<'s> {
    HtuCompatible(Htu<'s>),
    // add future sensors here...
}

/// A structure for holding all possible measurements that an environment sensor is
/// expected to support.
pub struct MeasurementResults {
    /// Temperature in degrees Celsius
    pub temperature: Temperature,

    /// Relative humidity (`0..=100`) in percentage
    pub humidity: Humidity,

    /// Air pressure in hecto-Pascals
    ///
    /// This may not be supported by all sensors.
    pub air_pressure: Option<AirPressure>,
}

impl EnvironmentSensor for AnySensor<'_> {
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

    fn get_hw_serial(&mut self) -> OsResult<u64> {
        match self {
            Self::HtuCompatible(dev) => dev.get_hw_serial(),
        }
    }
}
