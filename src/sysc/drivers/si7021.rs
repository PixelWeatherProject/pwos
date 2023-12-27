use super::EnvironmentSensor;
use crate::{os_debug, os_warn, sysc::OsResult};
use esp_idf_svc::hal::i2c::I2cDriver;
use pwmp_client::{
    bigdecimal::{BigDecimal, FromPrimitive},
    pwmp_types::aliases::{AirPressure, Humidity, Temperature},
};
use std::{thread::sleep, time::Duration};

#[derive(PartialEq, Clone, Copy)]
#[repr(u8)]
enum Command {
    ReadTemperature = 0xF3,
    ReadHumidity = 0xF5,
    Reset = 0xFE,
}

pub struct Si7021<'s>(I2cDriver<'s>);

impl<'s> Si7021<'s> {
    pub const DEV_ADDR: u8 = 0x40;

    const BUS_TIMEOUT: u32 = 1000;
    const CMD_WAIT_TIME: u64 = 50;

    pub fn new_with_driver(driver: I2cDriver<'s>) -> OsResult<Self> {
        os_debug!("Loading driver");
        let mut dev = Self(driver);
        dev.command(Command::Reset)?;

        Ok(dev)
    }

    fn command(&mut self, cmd: Command) -> OsResult<u16> {
        let mut buffer = [0u8; 2];

        self.0
            .write(Self::DEV_ADDR, &[cmd as u8], Self::BUS_TIMEOUT)?;
        sleep(Duration::from_millis(Self::CMD_WAIT_TIME));

        if cmd != Command::Reset {
            self.0
                .read(Self::DEV_ADDR, &mut buffer, Self::BUS_TIMEOUT)?;
        }

        Ok(((buffer[0] as u16) << 8) | (buffer[1] as u16))
    }
}

impl<'s> EnvironmentSensor for Si7021<'s> {
    fn connected(&mut self) -> OsResult<bool> {
        self.command(Command::Reset)?;
        Ok(true)
    }

    fn read_temperature(&mut self) -> OsResult<Temperature> {
        let raw = self.command(Command::ReadTemperature)?;
        let temp = ((175.72 * raw as f32) / 65536.0) - 46.85;

        Ok(BigDecimal::from_f32(temp).unwrap().with_scale(2))
    }

    fn read_humidity(&mut self) -> OsResult<Humidity> {
        let raw = self.command(Command::ReadHumidity)?;
        let hum = ((125.0 * raw as f32) / 65536.0) - 6.0;

        Ok(hum.floor().clamp(0.0, 100.0) as u8)
    }

    fn read_air_pressure(&mut self) -> Option<OsResult<AirPressure>> {
        os_warn!("Air pressure is not supported");
        None
    }
}
