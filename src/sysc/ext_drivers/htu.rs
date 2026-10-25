//! Driver for temperature and humidity sensors such as:
//! - HTU21D
//! - Si7021
//!
//! There are similiar models that may work as well but are not officially
//! tested nor supported.
//!
//! These sensors work over the I2C protocol.

use crate::sysc::{
    drivers::{shared_i2c::SharedI2cDriver, Driver, HumidityDriver, TemperatureDriver},
    OsError, OsResult,
};
use pwmp_client::pwmp_msg::aliases::{Humidity, Temperature};
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
pub struct Htu(SharedI2cDriver);

impl Htu {
    /// Known default address
    pub const DEV_ADDR: u8 = 0x40;

    const BUS_TIMEOUT: u32 = 2000;
    const CMD_WAIT_TIME: u64 = 50;

    /// Initialize the driver with the given I2C driver handle.
    pub fn new_with_driver(driver: SharedI2cDriver) -> Result<Self, OsError> {
        log::debug!("Loading driver");
        let dev = Self(driver);

        dev.reset()?;

        match dev.model()? {
            Some(model) => log::debug!("Detected '{model}'"),
            None => log::warn!("Device model is unknown and may not be supported"),
        }

        Ok(dev)
    }

    pub fn model(&self) -> OsResult<Option<&'static str>> {
        let mut buf = [0u8; 6];
        self.write_read(Command::ReadSerial1, &mut buf)?;

        let snb3 = buf[0];

        match snb3 {
            0x15 => Ok(Some("Si7021")),
            0x32 => Ok(Some("HTU21D")),
            0x14 => Ok(Some("Si7020")),
            0x0D => Ok(Some("Si7013")),
            _ => Ok(None),
        }
    }

    pub fn read_temperature(&self) -> OsResult<Temperature> {
        let raw = self.write_read_u16(Command::ReadTemperature)?;
        let temp = ((175.72 * f32::from(raw)) / 65536.0) - 46.85;
        Ok(temp)
    }

    #[allow(clippy::cast_sign_loss, clippy::cast_possible_truncation)]
    pub fn read_humidity(&self) -> OsResult<Humidity> {
        let raw = self.write_read_u16(Command::ReadHumidity)?;
        let hum = ((125.0 * f32::from(raw)) / 65536.0) - 6.0;
        let percentage = hum.floor().clamp(0., 100.);

        Ok(percentage as u8)
    }

    fn reset(&self) -> OsResult<()> {
        self.write(Command::Reset)?;
        sleep(Duration::from_millis(Self::CMD_WAIT_TIME));
        Ok(())
    }

    fn write(&self, command: Command) -> OsResult<()> {
        self.0
            .write(Self::DEV_ADDR, command.as_bytes(), Self::BUS_TIMEOUT)
    }

    fn write_read(&self, command: Command, buffer: &mut [u8]) -> OsResult<()> {
        self.0.write_read(
            Self::DEV_ADDR,
            command.as_bytes(),
            buffer,
            Self::BUS_TIMEOUT,
        )
    }

    fn write_read_u16(&self, command: Command) -> OsResult<u16> {
        let mut buffer = [0; 2];
        self.write_read(command, &mut buffer)?;

        let raw = u16::from_be_bytes(buffer);
        Ok(raw)
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

impl Driver for Htu {}

impl TemperatureDriver for Htu {
    fn is_internal(&self) -> bool {
        false
    }

    fn read(&self) -> OsResult<f32> {
        self.read_temperature()
    }
}

impl HumidityDriver for Htu {
    fn read(&self) -> OsResult<u8> {
        self.read_humidity()
    }
}
