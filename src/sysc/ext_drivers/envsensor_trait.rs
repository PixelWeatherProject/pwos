use crate::sysc::OsResult;
use pwmp_client::pwmp_msg::aliases::{AirPressure, Humidity, Temperature};

/// Contains functionality that an environment sensor must be able to do.
pub trait EnvironmentSensor {
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

    /// Returns whether a heater is supported by the sensor.
    /// The implementation should be simple const-fn-like, without any I/O operations, like this:
    ///
    /// ```rust
    /// fn heater_supported(&self) -> bool {
    ///    true
    /// }
    /// ```
    fn heater_supported(&self) -> bool;

    /// Enable or disable the sensor's heater, if supported.
    ///
    /// This method should not be called if the sensor does not support a heater, as indicated by [`heater_supported`](Self::heater_supported).
    ///
    /// # Errors
    /// Upon a connection or communication error, an `Err(..)` value will be returned.
    /// Additionally, if the sensor does not support a heater, [`OsError::UnsupportedOperation`](crate::sysc::error::OsError::UnsupportedOperation) will be returned.
    fn set_heater(&mut self, enabled: bool) -> OsResult<()>;
}
