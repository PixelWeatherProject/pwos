use crate::sysc::{
    drivers::{AirPressureDriver, HumidityDriver, TemperatureDriver},
    OsResult,
};
use std::sync::{Arc, RwLock};

pub(super) static GLOBAL_STORE: RwLock<RegisteredDevices> = RwLock::new(RegisteredDevices::new());

pub struct RegisteredDevices {
    // We have an internal and external sensor, so we allow multiple
    pub(super) temperature_sensors: Vec<Arc<dyn TemperatureDriver>>,

    // Only a single sensor is supported
    pub(super) humidity_sensor: Option<Arc<dyn HumidityDriver>>,

    // Only a single sensor is supported
    pub(super) air_pressure_sensor: Option<Arc<dyn AirPressureDriver>>,
}

impl RegisteredDevices {
    const fn new() -> Self {
        Self {
            temperature_sensors: Vec::new(),
            humidity_sensor: None,
            air_pressure_sensor: None,
        }
    }

    pub fn register_temperature_device<D: TemperatureDriver>(driver: Arc<D>) -> OsResult<()> {
        GLOBAL_STORE
            .try_write()
            .map(|mut store| store.temperature_sensors.push(driver))?;
        Ok(())
    }

    pub fn register_humidity_device<D: HumidityDriver>(driver: Arc<D>) -> OsResult<()> {
        GLOBAL_STORE
            .try_write()
            .map(|mut store| store.humidity_sensor = Some(driver))?;
        Ok(())
    }

    pub fn register_air_pressure_device<D: AirPressureDriver>(driver: Arc<D>) -> OsResult<()> {
        GLOBAL_STORE
            .try_write()
            .map(|mut store| store.air_pressure_sensor = Some(driver))?;
        Ok(())
    }
}
