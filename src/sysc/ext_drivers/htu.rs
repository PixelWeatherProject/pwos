//! Driver for temperature and humidity sensors such as:
//! - HTU21D
//! - Si7021
//!
//! These sensors work over the I2C protocol.

use super::EnvironmentSensor;
use crate::{
    re_esp,
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

    /// Read user register
    ReadUserRegister = 0xE7,

    /// Write user register
    WriteUserRegister = 0xE6,
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
        log::debug!("Loading driver");
        let mut dev = Self(driver);

        dev.reset()?;
        sleep(Duration::from_millis(Self::CMD_WAIT_TIME));

        Ok(dev)
    }

    /// Calculate temperature in Celsius from the result measured by the sensor.
    fn calc_temperature(raw: u16) -> Temperature {
        // ((175.72 * raw) / 65536.0) - 46.85
        ((175.72 * f32::from(raw)) / 65536.0) - 46.85
    }

    fn set_heater(&mut self, enabled: bool) -> OsResult<()> {
        let mut reg = self.read_user_register()?;

        if enabled {
            reg |= 0b0000_0100;
        } else {
            reg &= !0b0000_0100;
        }

        self.write_user_register(reg)
    }

    fn read_user_register(&mut self) -> OsResult<u8> {
        let mut buffer = [0; 1];
        self.write_read(&[Command::ReadUserRegister as u8], &mut buffer)?;
        Ok(buffer[0])
    }

    fn write_user_register(&mut self, value: u8) -> OsResult<()> {
        re_esp!(
            self.0.write(
                Self::DEV_ADDR,
                &[Command::WriteUserRegister as u8, value],
                Self::BUS_TIMEOUT
            ),
            I2c
        )?;

        Ok(())
    }

    fn reset(&mut self) -> OsResult<()> {
        self.write(&[Command::Reset as u8])
    }

    fn write(&mut self, command: &[u8]) -> OsResult<()> {
        re_esp!(
            self.0.write(Self::DEV_ADDR, command, Self::BUS_TIMEOUT),
            I2c
        )
    }

    fn write_read(&mut self, command: &[u8], buffer: &mut [u8]) -> OsResult<()> {
        re_esp!(
            self.0
                .write_read(Self::DEV_ADDR, command, buffer, Self::BUS_TIMEOUT),
            I2c
        )
    }
}

impl EnvironmentSensor for Htu<'_> {
    fn read_temperature(&mut self) -> OsResult<Temperature> {
        let mut buffer = [0; 2];
        self.write_read(&[Command::ReadTemperature as u8], &mut buffer)?;
        let raw = u16::from_be_bytes(buffer);

        Ok(Self::calc_temperature(raw))
    }

    #[allow(clippy::cast_sign_loss, clippy::cast_possible_truncation)]
    fn read_humidity(&mut self) -> OsResult<Humidity> {
        let mut buffer = [0; 2];
        self.write_read(&[Command::ReadHumidity as u8], &mut buffer)?;
        let raw = u16::from_be_bytes(buffer);

        let hum = ((125.0 * f32::from(raw)) / 65536.0) - 6.0;
        let percentage = hum.floor().clamp(0., 100.);

        Ok(percentage as u8)
    }

    fn read_air_pressure(&mut self) -> OsResult<Option<AirPressure>> {
        #[cfg(debug_assertions)]
        log::warn!("Air pressure is not supported");
        Ok(None)
    }

    fn heater_supported(&self) -> bool {
        true
    }

    fn set_heater(&mut self, enabled: bool) -> OsResult<()> {
        self.set_heater(enabled)
    }
}
