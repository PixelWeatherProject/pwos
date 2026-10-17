use crate::sysc::ddls::{Driver, TemperatureDriver};
use std::time::Instant;

pub struct InternalTemperatureSensor;

impl Driver for InternalTemperatureSensor {
    fn driver_name(&self) -> &'static str {
        "InternalTemperatureSensorDriver"
    }

    fn driver_version(&self) -> &'static str {
        "1.0.0"
    }

    fn device_name(&self) -> &'static str {
        "Generic ESP32S3 Internal Temperature Sensor"
    }

    fn device_model(&self) -> &'static str {
        "Generic"
    }
}

impl TemperatureDriver for InternalTemperatureSensor {
    fn read(&self) -> f32 {
        todo!()
    }

    fn is_internal(&self) -> bool {
        true
    }
}

pub struct TemperatureSensor {
    last_value: f32,
    last_read: Instant,
    is_internal: bool,
    driver: Box<dyn TemperatureDriver>,
}

impl TemperatureSensor {}
