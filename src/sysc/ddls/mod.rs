use esp_idf_svc::hal::temp_sensor::TempSensorDriver;

use crate::sysc::{
    drivers::{
        internal_temp_sensor::CpuTemperatureSensor, shared_i2c::SharedI2cDriver,
        store::RegisteredDevices,
    },
    ext_drivers::{Bosch, Htu},
    OsResult,
};
use std::sync::Arc;

pub fn init(i2c: &SharedI2cDriver, int_temp_sensor: TempSensorDriver<'static>) -> OsResult<()> {
    // register I2C devices
    init_i2c_devices(i2c);

    // register the internal temperature sensor
    RegisteredDevices::register_temperature_device(Arc::new(CpuTemperatureSensor::from(
        int_temp_sensor,
    )))?;

    // ... implement more interfaces here

    #[cfg(debug_assertions)]
    log::debug!(
        "Loaded {} devices",
        RegisteredDevices::loaded_device_count()
    );

    Ok(())
}

fn init_i2c_devices(i2c: &SharedI2cDriver) {
    for addr in 1..128 {
        if i2c.write(addr, []).is_ok() {
            log::debug!("Found device @ I2C/0x{addr:X}");

            match init_i2c_device(i2c.clone(), addr) {
                Ok(()) => log::debug!("Device @ I2C/0x{addr:X} set up successfully"),
                Err(why) => log::error!("Device I2C/0x{addr:X} could not be set up: {why}"),
            }
        }
    }
}

fn init_i2c_device(i2c: SharedI2cDriver, address: u8) -> OsResult<()> {
    match address {
        Htu::DEV_ADDR => {
            log::debug!("Detected HTU-compatible sensor");

            let htu = Arc::new(Htu::new_with_driver(i2c)?);

            RegisteredDevices::register_temperature_device(Arc::clone(&htu))?;
            RegisteredDevices::register_humidity_device(htu)?;
        }
        bosch_addr if Bosch::DEV_ADDRS.contains(&bosch_addr) => {
            log::debug!("Deteched Bosch sensor");

            let mut driver = Bosch::new_with_driver(i2c, bosch_addr)?;
            driver.initialize()?;
            let driver = Arc::new(driver);

            RegisteredDevices::register_temperature_device(Arc::clone(&driver))?;
            RegisteredDevices::register_humidity_device(Arc::clone(&driver))?;
            RegisteredDevices::register_air_pressure_device(driver)?;
        }
        other => {
            log::warn!("Unrecognised device @ I2C/0x{other:X}");
        }
    }

    Ok(())
}
