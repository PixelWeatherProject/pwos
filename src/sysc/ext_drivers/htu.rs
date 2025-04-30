//! Driver for temperature and humidity sensors such as:
//! - HTU21D
//! - Si7021
//!
//! These sensors work over the I2C protocol.

use super::EnvironmentSensor;
use crate::{
    os_debug,
    sysc::{OsError, OsResult},
};
use esp_idf_svc::hal::i2c::I2cDriver;
use pwmp_client::pwmp_msg::aliases::{AirPressure, Humidity, Temperature};
use std::{thread::sleep, time::Duration};

/// Commands for HTU21D (and similar) sensors.
#[derive(PartialEq, Clone, Copy)]
#[repr(u8)]
enum Command {
    /// Request temperature reading
    ReadTemperature = 0xE3,

    /// Request humidity reading
    ReadHumidity = 0xE5,

    /// Reset the device
    Reset = 0xFE,
}

/// Driver handle for HTU21D (and similar) sensors.
pub struct Htu<'s>(I2cDriver<'s>);

impl<'s> Htu<'s> {
    /// Known default address
    pub const DEV_ADDR: u8 = 0x40;

    const BUS_TIMEOUT: u32 = 1000;
    const CMD_WAIT_TIME: u64 = 50;

    /// Initialize the driver with the given I2C driver handle.
    pub fn new_with_driver(driver: I2cDriver<'s>) -> Result<Self, OsError> {
        os_debug!("Loading driver");
        let mut dev = Self(driver);

        dev.command(Command::Reset)?;

        Ok(dev)
    }

    /// Send a command to the device.
    fn command(&mut self, cmd: Command) -> OsResult<u16> {
        let mut buffer = [0u8; 2];

        self.0
            .write(Self::DEV_ADDR, &[cmd as u8], Self::BUS_TIMEOUT)?;
        sleep(Duration::from_millis(Self::CMD_WAIT_TIME));

        if cmd != Command::Reset {
            self.0
                .read(Self::DEV_ADDR, &mut buffer, Self::BUS_TIMEOUT)?;
        }

        Ok(((u16::from(buffer[0])) << 8) | (u16::from(buffer[1])))
    }

    /// Calculate temperature in Celsius from the result measured by the sensor.
    fn calc_temperature(raw: u16) -> Temperature {
        // ((175.72 * raw) / 65536.0) - 46.85
        ((175.72 * f32::from(raw)) / 65536.0) - 46.85
    }
}

impl EnvironmentSensor for Htu<'_> {
    fn connected(&mut self) -> OsResult<bool> {
        self.command(Command::Reset)?;
        Ok(true)
    }

    fn read_temperature(&mut self) -> OsResult<Temperature> {
        let raw = self.command(Command::ReadTemperature)?;

        Ok(Self::calc_temperature(raw))
    }

    fn read_humidity(&mut self) -> OsResult<Humidity> {
        let raw = self.command(Command::ReadHumidity)?;
        let hum = ((125.0 * f32::from(raw)) / 65536.0) - 6.0;
        let percentage = hum.floor().clamp(0., 100.);

        Ok(percentage as u8)
    }

    fn read_air_pressure(&mut self) -> OsResult<Option<AirPressure>> {
        #[cfg(debug_assertions)]
        crate::os_warn!("Air pressure is not supported");
        Ok(None)
    }
}
