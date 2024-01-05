use super::EnvironmentSensor;
use crate::{os_debug, sysc::OsResult};
use pwmp_client::pwmp_types::{
    aliases::{AirPressure, Humidity, Temperature},
    Decimal,
};

pub struct FakeEnvSensor;

impl FakeEnvSensor {
    pub const DEV_ADDR: u8 = 0xFF;

    #[allow(clippy::unnecessary_wraps)]
    pub fn new() -> OsResult<Self> {
        os_debug!("Initializing driver");
        Ok(Self)
    }
}

impl EnvironmentSensor for FakeEnvSensor {
    fn connected(&mut self) -> OsResult<bool> {
        Ok(true)
    }

    fn read_temperature(&mut self) -> OsResult<Temperature> {
        Ok(Decimal::default())
    }

    fn read_humidity(&mut self) -> OsResult<Humidity> {
        Ok(69)
    }

    fn read_air_pressure(&mut self) -> Option<OsResult<AirPressure>> {
        Some(Ok(321))
    }
}
