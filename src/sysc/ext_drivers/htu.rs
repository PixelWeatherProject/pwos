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
#[derive(Clone, Copy)]
enum Command {
    /// Request temperature reading
    ReadTemperature,

    /// Request humidity reading
    ReadHumidity,

    /// Reset the device
    Reset,

    /// Read the first part of the device serial number
    ReadSerial1,
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

        match dev.model()? {
            Some(model) => log::debug!("Detected '{model}'"),
            None => log::warn!("Device model is unknown and may not be supported"),
        }

        Ok(dev)
    }

    fn model(&mut self) -> OsResult<Option<&'static str>> {
        let snb3 = self.write_read_u8(Command::ReadSerial1)?;

        match snb3 {
            0x15 => Ok(Some("Si7021")),
            0x32 => Ok(Some("HTU21D")),
            0x14 => Ok(Some("Si7020")),
            0x0D => Ok(Some("Si7013")),
            _ => Ok(None),
        }
    }

    fn reset(&mut self) -> OsResult<()> {
        self.write(Command::Reset)?;
        sleep(Duration::from_millis(Self::CMD_WAIT_TIME));
        Ok(())
    }

    fn write(&mut self, command: Command) -> OsResult<()> {
        re_esp!(
            self.0
                .write(Self::DEV_ADDR, command.as_bytes(), Self::BUS_TIMEOUT),
            I2c
        )
    }

    fn write_read(&mut self, command: Command, buffer: &mut [u8]) -> OsResult<()> {
        re_esp!(
            self.0.write_read(
                Self::DEV_ADDR,
                command.as_bytes(),
                buffer,
                Self::BUS_TIMEOUT,
            ),
            I2c
        )
    }

    fn write_read_u8(&mut self, command: Command) -> OsResult<u8> {
        let mut buffer = [0; 1];
        self.write_read(command, &mut buffer)?;

        let raw = u8::from_be_bytes(buffer);
        Ok(raw)
    }

    fn write_read_u16(&mut self, command: Command) -> OsResult<u16> {
        let mut buffer = [0; 2];
        self.write_read(command, &mut buffer)?;

        let raw = u16::from_be_bytes(buffer);
        Ok(raw)
    }
}

impl EnvironmentSensor for Htu<'_> {
    fn read_temperature(&mut self) -> OsResult<Temperature> {
        let raw = self.write_read_u16(Command::ReadTemperature)?;
        let temp = ((175.72 * f32::from(raw)) / 65536.0) - 46.85;
        Ok(temp)
    }

    #[allow(clippy::cast_sign_loss, clippy::cast_possible_truncation)]
    fn read_humidity(&mut self) -> OsResult<Humidity> {
        let raw = self.write_read_u16(Command::ReadHumidity)?;
        let hum = ((125.0 * f32::from(raw)) / 65536.0) - 6.0;
        let percentage = hum.floor().clamp(0., 100.);

        Ok(percentage as u8)
    }

    fn read_air_pressure(&mut self) -> OsResult<Option<AirPressure>> {
        log::warn!("Air pressure is not supported");
        Ok(None)
    }
}

impl Command {
    /// Get the command as a byte array for I2C transmission.
    const fn as_bytes(self) -> &'static [u8] {
        match self {
            Self::ReadTemperature => &[0xE3],
            Self::ReadHumidity => &[0xE5],
            Self::Reset => &[0xFE],
            Self::ReadSerial1 => &[0xFC, 0xC9],
        }
    }
}
