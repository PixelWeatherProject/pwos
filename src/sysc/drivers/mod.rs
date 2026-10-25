use crate::sysc::OsResult;
use pwmp_client::pwmp_msg::aliases::Temperature;
use std::{ops::Deref, sync::Arc};

pub mod internal_temp_sensor;
pub mod shared_i2c;
pub mod store;

pub trait Driver: 'static + Send + Sync {}

pub trait TemperatureDriver: Driver {
    /// Read the temperature in decidegrees Celsius.
    fn read(&self) -> OsResult<Temperature>;

    fn is_internal(&self) -> bool;
}

pub trait HumidityDriver: Driver {
    fn read(&self) -> OsResult<u8>;
}

pub trait AirPressureDriver: Driver {
    fn read(&self) -> OsResult<u16>;
}

pub struct TemperatureSensor {
    driver: Arc<dyn TemperatureDriver>,
}

impl TemperatureSensor {
    pub fn get_internal() -> Option<Self> {
        store::GLOBAL_STORE
            .try_read()
            .ok()?
            .temperature_sensors
            .iter()
            .find(|s| s.is_internal())
            .map(|s| Self {
                driver: Arc::clone(s),
            })
    }

    pub fn get_external() -> Option<Self> {
        store::GLOBAL_STORE
            .try_read()
            .ok()?
            .temperature_sensors
            .iter()
            .find(|s| !s.is_internal())
            .map(|s| Self {
                driver: Arc::clone(s),
            })
    }
}

impl Deref for TemperatureSensor {
    type Target = dyn TemperatureDriver;

    fn deref(&self) -> &Self::Target {
        self.driver.as_ref()
    }
}

pub struct HumiditySensor {
    driver: Arc<dyn HumidityDriver>,
}

impl HumiditySensor {
    pub fn get() -> Option<Self> {
        store::GLOBAL_STORE
            .try_read()
            .ok()?
            .humidity_sensor
            .as_ref()
            .map(|d| Self {
                driver: Arc::clone(d),
            })
    }
}

impl Deref for HumiditySensor {
    type Target = dyn HumidityDriver;

    fn deref(&self) -> &Self::Target {
        self.driver.as_ref()
    }
}

pub struct AirPressureSensor {
    driver: Arc<dyn AirPressureDriver>,
}

impl AirPressureSensor {
    pub fn get() -> Option<Self> {
        store::GLOBAL_STORE
            .try_read()
            .ok()?
            .air_pressure_sensor
            .as_ref()
            .map(|d| Self {
                driver: Arc::clone(d),
            })
    }
}

impl Deref for AirPressureSensor {
    type Target = dyn AirPressureDriver;

    fn deref(&self) -> &Self::Target {
        self.driver.as_ref()
    }
}
