use super::{Driver, TemperatureDriver};
use crate::sysc::{OsError, OsResult};
use esp_idf_svc::hal::temp_sensor::TempSensorDriver;
use std::sync::{Arc, Mutex};

pub struct CpuTemperatureSensor(Arc<Mutex<TempSensorDriver<'static>>>);

impl From<TempSensorDriver<'static>> for CpuTemperatureSensor {
    fn from(value: TempSensorDriver<'static>) -> Self {
        Self(Arc::new(Mutex::new(value)))
    }
}

impl Driver for CpuTemperatureSensor {
    fn driver_name(&self) -> &'static str {
        "ESP32S3 Internal Temperature Sensor Driver"
    }

    fn driver_version(&self) -> &'static str {
        "v1.0.0"
    }

    fn device_name(&self) -> OsResult<String> {
        Ok("Internal Temperature Sesnor".to_string())
    }

    fn device_model(&self) -> OsResult<String> {
        Ok("Generic".to_string())
    }
}

impl TemperatureDriver for CpuTemperatureSensor {
    fn read(&self) -> OsResult<f32> {
        self.0
            .lock()?
            .get_celsius()
            .map_err(OsError::InternalTempSensorRead)
    }

    fn is_internal(&self) -> bool {
        true
    }
}
