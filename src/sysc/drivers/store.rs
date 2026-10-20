use crate::sysc::drivers::{AirPressureDriver, HumidityDriver, TemperatureDriver};
use std::sync::Arc;

pub(super) static mut GLOBAL_STORE: RegisteredDevices = RegisteredDevices::new();

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

    pub fn register_temperature_device<D: TemperatureDriver>(driver: Arc<D>) {
        unsafe { GLOBAL_STORE.temperature_sensors.push(driver) };
    }

    pub fn register_humidity_device<D: HumidityDriver>(driver: Arc<D>) {
        unsafe {
            GLOBAL_STORE.humidity_sensor = Some(driver);
        }
    }

    pub fn register_air_pressure_device<D: AirPressureDriver>(driver: Arc<D>) {
        unsafe {
            GLOBAL_STORE.air_pressure_sensor = Some(driver);
        }
    }
}
