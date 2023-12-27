use crate::sysc::OsResult;
use pwmp_client::pwmp_types::aliases::{AirPressure, Humidity, Temperature};

pub trait EnvironmentSensor {
    fn connected(&mut self) -> OsResult<bool>;
    fn read_temperature(&mut self) -> OsResult<Temperature>;
    fn read_humidity(&mut self) -> OsResult<Humidity>;
    fn read_air_pressure(&mut self) -> Option<OsResult<AirPressure>>;
}
