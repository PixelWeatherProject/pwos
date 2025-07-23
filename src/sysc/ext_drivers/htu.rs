//! Driver for temperature and humidity sensors such as:
//! - HTU21D
//! - Si7021
//!
//! These sensors work over the I2C protocol.

use super::EnvironmentSensor;
use crate::{
    os_debug, re_esp,
    sysc::{OsError, OsResult},
};
use esp_idf_svc::hal::i2c::I2cDriver;
use pwmp_client::pwmp_msg::aliases::{AirPressure, Humidity, Temperature};
use std::{thread::sleep, time::Duration};

// CRC polynomial for SI7021
const POLYNOMIAL: u8 = 0x31;

/// Commands for HTU21D (and similar) sensors.
#[derive(PartialEq, Clone, Copy)]
#[repr(u8)]
enum Command {
    /// Request temperature reading
    ReadTemperature = 0xE3,

    /// Request humidity reading
    ReadHumidity = 0xE5,

    /// First part of the request to read the first 4 bytes of the hardware serial number
    ReadHwSerialBeginFirst = 0xFA,

    /// Second part of the request to read the first 4 bytes of the hardware serial number
    HeadHwSerialEndFirst = 0x0F,

    /// First part of the request to read the last 4 bytes of the hardware serial number
    ReadHwSerialBeginLast = 0xFC,

    /// Second part of the request to read the last 4 bytes of the hardware serial number
    HeadHwSerialEndLast = 0xC9,

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

        re_esp!(
            self.0
                .write(Self::DEV_ADDR, &[cmd as u8], Self::BUS_TIMEOUT),
            I2cWrite
        )?;
        sleep(Duration::from_millis(Self::CMD_WAIT_TIME));

        if cmd != Command::Reset {
            re_esp!(
                self.0.read(Self::DEV_ADDR, &mut buffer, Self::BUS_TIMEOUT),
                I2cRead
            )?;
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

    #[allow(clippy::cast_sign_loss, clippy::cast_possible_truncation)]
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

    fn get_hw_serial(&mut self) -> OsResult<u64> {
        // datasheet: https://www.silabs.com/documents/public/data-sheets/Si7021-A20.pdf
        // page: 23

        /*
         * The CRC byte is the checksum for the data bytes that have been received since the first response byte.
         *
         * Example response:
         * 0x0D 0x0E 0x0A 0x0D 0x0B 0x0E 0x0A 0x0F
         * ^^^^ ^^^^ ^^^^ ^^^^ ^^^^ ^^^^      ^^^^
         *   |    |    |    |    |    |         |-> crc for [0x0D, 0x0A, 0x0B, 0x0A]
         *   |    |    |    |    |   ...
         *   |    |    |    |    -> data
         *   |    |    |    -> crc for [0x0D, 0x0A]
         *   |    |    -> data
         *   |    -> crc for [0x0D]
         *   -> data
         *
         * So when checking the resulting CRC, we only need the last CRC byte and compare it
         * to the CRC of all the data bytes. So in the above example, we need to calculate the CRC
         * of [0x0D, 0x0A, 0x0B, 0x0A] and compare it to 0x0F (last CRC byte).
         */

        // SNA_3, CRC, SNA_2, CRC, SNA_1, CRC, SNA_0, CRC
        let mut first_bytes = [0; 8];
        // SNB_3, SNB_2, CRC, SNB_1, SNB_0, CRC
        let mut second_bytes = [0; 6];

        // read the first 8 bytes
        re_esp!(
            self.0.write_read(
                Self::DEV_ADDR,
                &[
                    Command::ReadHwSerialBeginFirst as _,
                    Command::HeadHwSerialEndFirst as _,
                ],
                &mut first_bytes,
                Self::BUS_TIMEOUT,
            ),
            I2cRead
        )?;

        // read the last 6 bytes
        re_esp!(
            self.0.write_read(
                Self::DEV_ADDR,
                &[
                    Command::ReadHwSerialBeginLast as _,
                    Command::HeadHwSerialEndLast as _,
                ],
                &mut second_bytes,
                Self::BUS_TIMEOUT,
            ),
            I2cRead
        )?;

        let first_crc = calculate_crc8_checksum(&[
            first_bytes[0],
            first_bytes[2],
            first_bytes[4],
            first_bytes[6],
        ]);
        let last_crc = calculate_crc8_checksum(&[
            second_bytes[0],
            second_bytes[1],
            second_bytes[3],
            second_bytes[4],
        ]);

        if first_crc != first_bytes[7] {
            panic!("crc error");
        }
        if last_crc != second_bytes[5] {
            panic!("crc error");
        }

        let serial = u64::from_ne_bytes([
            first_bytes[0],  // SNA_3
            first_bytes[2],  // SNA_2
            first_bytes[4],  // SNA_1
            first_bytes[6],  // SNA_0
            second_bytes[0], // SNB_3
            second_bytes[1], // SNB_2
            second_bytes[3], // SNB_1
            second_bytes[4], // SNB_0
        ]);

        Ok(serial)
    }
}

pub fn calculate_crc8_checksum(data: &[u8]) -> u8 {
    let mut crc: u8 = 0x00; // Initialization value

    for &byte in data {
        crc ^= byte;

        for _ in 0..8 {
            if crc & 0x80 != 0 {
                crc = (crc << 1) ^ POLYNOMIAL;
            } else {
                crc <<= 1;
            }
        }
    }

    crc
}
