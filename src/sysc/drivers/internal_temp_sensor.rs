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

impl Driver for CpuTemperatureSensor {}

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
