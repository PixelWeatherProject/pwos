mod envsensor_trait;
mod htu;

#[cfg(feature = "fake")]
mod fake;

pub use envsensor_trait::EnvironmentSensor;
#[cfg(feature = "fake")]
pub use fake::MockSensor;
pub use htu::Htu;
use pwmp_client::pwmp_msg::aliases::{AirPressure, Humidity, Temperature};

pub struct AnySensor<'s>(Box<dyn EnvironmentSensor + 's>);

pub struct MeasurementResults {
    pub temperature: Temperature,
    pub humidity: Humidity,
    pub air_pressure: Option<AirPressure>,
}

impl EnvironmentSensor for AnySensor<'_> {
    fn connected(&mut self) -> super::OsResult<bool> {
        self.0.connected()
    }

    fn read_air_pressure(&mut self) -> super::OsResult<Option<AirPressure>> {
        self.0.read_air_pressure()
    }

    fn read_humidity(&mut self) -> super::OsResult<Humidity> {
        self.0.read_humidity()
    }

    fn read_temperature(&mut self) -> super::OsResult<Temperature> {
        self.0.read_temperature()
    }
}
