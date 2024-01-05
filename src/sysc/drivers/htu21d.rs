use super::EnvironmentSensor;
use crate::{
    os_debug, os_error, os_warn,
    sysc::{OsError, OsResult},
};
use esp_idf_svc::{
    hal::i2c::I2cDriver,
    sys::{EspError, ESP_ERR_TIMEOUT, ESP_FAIL},
};
use pwmp_client::pwmp_types::{
    aliases::{AirPressure, Humidity, Temperature},
    dec, Decimal,
};
use std::{
    thread::sleep,
    time::{Duration, Instant},
};

#[derive(PartialEq, Clone, Copy)]
#[repr(u8)]
enum Command {
    ReadTemperature = 0xE3,
    ReadHumidity = 0xE5,
    Reset = 0xFE,
}

pub struct Htu21d<'s>(I2cDriver<'s>);

impl<'s> Htu21d<'s> {
    pub const DEV_ADDR: u8 = 0x40;

    const CMD_WAITTIME_MS: u64 = 50;
    const BUS_TIMEOUT: u32 = 1000;
    const OPERATION_TIMEOUT: Duration = Duration::from_millis(1000);

    pub fn new_with_driver(driver: I2cDriver<'s>) -> OsResult<Self> {
        os_debug!("Loading driver");
        let mut dev = Self(driver);

        dev.command(Command::Reset)?;
        sleep(Duration::from_millis(Self::CMD_WAITTIME_MS));

        Ok(dev)
    }

    fn command(&mut self, cmd: Command) -> OsResult<u16> {
        let mut buffer: [u8; 3] = [0; 3];

        if !self.write(cmd) {
            return Err(OsError::Esp(EspError::from_infallible::<ESP_FAIL>()));
        }

        if cmd != Command::Reset {
            self.read(&mut buffer)?;
        }

        Ok(((buffer[0] as u16) << 8) + buffer[1] as u16)
    }

    fn read(&mut self, buf: &mut [u8]) -> OsResult<()> {
        let start = Instant::now();

        loop {
            if start.elapsed() >= Self::OPERATION_TIMEOUT {
                os_error!("Timeout reading from device");
                return Err(OsError::Esp(EspError::from_infallible::<ESP_ERR_TIMEOUT>()));
            }

            let res = self.0.read(Self::DEV_ADDR, buf, Self::BUS_TIMEOUT);
            if res.is_ok() {
                return Ok(());
            }

            sleep(Duration::from_millis(Self::CMD_WAITTIME_MS));
            os_warn!("Error reading from device, retrying");
        }
    }

    fn write(&mut self, cmd: Command) -> bool {
        let start = Instant::now();

        loop {
            if start.elapsed() >= Self::OPERATION_TIMEOUT {
                os_error!("Timeout while sending command to device");
                return false;
            }

            let res = self
                .0
                .write(Self::DEV_ADDR, &[cmd as u8], Self::BUS_TIMEOUT);
            sleep(Duration::from_millis(Self::CMD_WAITTIME_MS));

            if res.is_ok() {
                return true;
            }
            os_warn!("Error sending command to device, retrying");
        }
    }

    fn calc_temperature(raw: u16) -> Temperature {
        // -46.85 + ((175.72 * raw) / 65536.0)
        let mut value = Decimal::new(raw as i64 * 100, 2);

        value *= dec!(175.72);
        value /= dec!(65536.00);
        value.rescale(2);

        value
    }
}

impl<'s> EnvironmentSensor for Htu21d<'s> {
    fn connected(&mut self) -> OsResult<bool> {
        self.command(Command::Reset)?;

        sleep(Duration::from_millis(Self::CMD_WAITTIME_MS));
        Ok(true)
    }

    fn read_temperature(&mut self) -> OsResult<Temperature> {
        let raw = self.command(Command::ReadTemperature)?;

        Ok(Self::calc_temperature(raw))
    }

    fn read_humidity(&mut self) -> OsResult<Humidity> {
        let raw = self.command(Command::ReadHumidity)?;
        let hum = -6.0 + (125.0 * raw as f32 / 65536.0);

        Ok(hum.floor().clamp(0.0, 100.0) as u8)
    }

    fn read_air_pressure(&mut self) -> Option<OsResult<AirPressure>> {
        None
    }
}
