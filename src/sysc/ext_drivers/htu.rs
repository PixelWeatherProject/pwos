use super::EnvironmentSensor;
use crate::{
    os_debug,
    sysc::{OsError, OsResult},
};
use esp_idf_svc::hal::i2c::I2cDriver;
use pwmp_client::pwmp_msg::{
    aliases::{AirPressure, Humidity, Temperature},
    dec, Decimal,
};
use std::{thread::sleep, time::Duration};

#[derive(PartialEq, Clone, Copy)]
#[repr(u8)]
enum Command {
    ReadTemperature = 0xE3,
    ReadHumidity = 0xE5,
    Reset = 0xFE,
}

pub struct Htu<'s>(I2cDriver<'s>);

impl<'s> Htu<'s> {
    pub const DEV_ADDR: u8 = 0x40;

    const BUS_TIMEOUT: u32 = 1000;
    const CMD_WAIT_TIME: u64 = 50;

    pub fn new_with_driver(driver: I2cDriver<'s>) -> Result<Self, OsError> {
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

        Ok(((u16::from(buffer[0])) << 8) | (u16::from(buffer[1])))
    }

    fn calc_temperature(raw: u16) -> Temperature {
        // ((175.72 * raw) / 65536.0) - 46.85
        let mut temp = ((dec!(175.72) * (Decimal::from(raw))) / dec!(65536.0)) - dec!(46.85);
        temp.rescale(2);

        temp
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
        let hum = ((dec!(125.0) * Decimal::from(raw)) / dec!(65536.0)) - dec!(6.0);
        let percentage = hum.floor().clamp(Decimal::ZERO, Decimal::ONE_HUNDRED);

        // SAFETY: The value of `percentage` is clamped between 0 and 100, which is a valid `u8`.
        Ok(unsafe { u8::try_from(percentage).unwrap_unchecked() })
    }

    fn read_air_pressure(&mut self) -> OsResult<Option<AirPressure>> {
        #[cfg(debug_assertions)]
        crate::os_warn!("Air pressure is not supported");
        Ok(None)
    }
}
