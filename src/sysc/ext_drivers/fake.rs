use super::EnvironmentSensor;
use crate::{os_debug, sysc::OsError};
use esp_idf_svc::hal::i2c::I2cDriver;
use pwmp_client::pwmp_msg::{dec, Decimal};
use std::marker::PhantomData;

pub struct MockSensor<'s>(PhantomData<&'s ()>);

impl<'s> MockSensor<'s> {
    pub fn new_with_driver(_: I2cDriver<'s>) -> Result<Self, OsError> {
        os_debug!("Loading driver");
        Ok(Self(PhantomData))
    }
}

impl EnvironmentSensor for MockSensor<'_> {
    fn connected(&mut self) -> crate::sysc::OsResult<bool> {
        Ok(true)
    }

    fn read_air_pressure(
        &mut self,
    ) -> crate::sysc::OsResult<Option<pwmp_client::pwmp_msg::aliases::AirPressure>> {
        Ok(Some(5000))
    }

    fn read_humidity(&mut self) -> crate::sysc::OsResult<pwmp_client::pwmp_msg::aliases::Humidity> {
        Ok(40)
    }

    fn read_temperature(
        &mut self,
    ) -> crate::sysc::OsResult<pwmp_client::pwmp_msg::aliases::Temperature> {
        Ok(dec!(20.00))
    }
}

impl<'s> From<MockSensor<'s>> for super::AnySensor<'s> {
    fn from(value: MockSensor<'s>) -> Self {
        Self(Box::new(value))
    }
}
