use std::sync::{Arc, Mutex};

use crate::sysc::OsResult;

mod global_store;
mod temperature_sensors;

static mut LOADED_PERIPHERALS: Vec<Box<dyn Driver>> = Vec::new();

pub struct SharedDriver(Arc<Mutex<Box<dyn Driver>>>);

pub trait Driver: 'static + Send + Sync {
    fn driver_name(&self) -> &'static str;

    fn driver_version(&self) -> &'static str;

    fn device_name(&self) -> &'static str;

    fn device_model(&self) -> &'static str;
}

pub trait TemperatureDriver: Driver {
    fn read(&self) -> f32;

    fn is_internal(&self) -> bool;
}

pub trait HumidityDriver: Driver {
    fn read(&self) -> u8;
}

impl<T: Driver> From<T> for SharedDriver {
    fn from(value: T) -> Self {
        Self(Arc::new(Mutex::new(Box::new(value))))
    }
}

pub fn init_subsystem() -> OsResult<()> {
    Ok(())
}
