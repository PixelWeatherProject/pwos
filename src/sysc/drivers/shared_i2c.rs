use crate::sysc::{OsError, OsResult};
use esp_idf_svc::hal::i2c::I2cDriver;
use std::sync::{Arc, Mutex};

pub struct SharedI2cDriver(Arc<Mutex<I2cDriver<'static>>>);

impl SharedI2cDriver {
    pub fn wrap(driver: I2cDriver<'static>) -> Self {
        Self(Arc::new(Mutex::new(driver)))
    }

    pub fn write(&self, addr: u8, data: impl AsRef<[u8]>, timeout: u32) -> OsResult<()> {
        OsError::from_i2c_writeop(
            self.0.lock()?.write(addr, data.as_ref(), timeout),
            addr,
            data.as_ref(),
            false,
        )
    }

    pub fn write_read(
        &self,
        addr: u8,
        data: impl AsRef<[u8]>,
        buffer: &mut [u8],
        timeout: u32,
    ) -> OsResult<()> {
        OsError::from_i2c_writeop(
            self.0
                .lock()?
                .write_read(addr, data.as_ref(), buffer, timeout),
            addr,
            data.as_ref(),
            true,
        )
    }
}
