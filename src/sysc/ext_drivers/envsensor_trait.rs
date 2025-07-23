use crate::sysc::OsResult;
use pwmp_client::pwmp_msg::aliases::{AirPressure, Humidity, Temperature};

/// Contains functionality that an environment sensor must be able to do.
pub trait EnvironmentSensor {
    /// Check if the environment sensor is connected.
    ///
    /// # Errors
    /// Upon a connection or communication error, an `Err(..)` value will be returned.
    fn connected(&mut self) -> OsResult<bool>;

    /// Read environment temperature.
    ///
    /// # Errors
    /// Upon a connection or communication error, an `Err(..)` value will be returned.
    fn read_temperature(&mut self) -> OsResult<Temperature>;

    /// Read environment *(relative)* humidity.
    ///
    /// # Errors
    /// Upon a connection or communication error, an `Err(..)` value will be returned.
    fn read_humidity(&mut self) -> OsResult<Humidity>;

    /// Read environment air pressure in hPa. If the sensor does not support this
    /// feature, `Ok(None)` will be returned.
    ///
    /// # Errors
    /// Upon a connection or communication error, an `Err(..)` value will be returned.
    fn read_air_pressure(&mut self) -> OsResult<Option<AirPressure>>;

    /// Get the serial number of the sensor.
    ///
    /// # Errors
    /// Upon a connection or communication error, an `Err(..)` value will be returned.
    fn get_hw_serial(&mut self) -> OsResult<u64>;
}
