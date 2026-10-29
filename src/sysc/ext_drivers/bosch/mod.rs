use crate::sysc::{drivers::shared_i2c::SharedI2cDriver, OsError, OsResult};
use pwmp_client::pwmp_msg::aliases::{AirPressure, Humidity, Temperature};

mod bme280;
mod bme690;

const REG_CHIP_ID: u8 = 0xD0;
const REG_VARIANT_ID: u8 = 0xF0;

const CHIP_ID_BME280: u8 = 0x60;
const CHIP_ID_BME6XX: u8 = 0x61;
const CHIP_ID_BME690_VARIANT: u8 = 0x02;

pub struct Bosch {
    /// Exact driver for the model.
    model_driver: ModelDriver,
}

enum ModelDriver {
    Bme280(bme280::Driver),
    Bme690(bme690::Driver),
}

impl Bosch {
    /// Known default address
    pub const DEV_ADDRS: [u8; 2] = [0x76, 0x77];

    pub fn new_with_driver(i2c: SharedI2cDriver, addr: u8) -> OsResult<Self> {
        let model = read_model(&i2c, addr)?;

        let model_driver = match model {
            "BME280" => ModelDriver::Bme280(bme280::Driver::new(addr, i2c)),
            "BME690" => ModelDriver::Bme690(bme690::Driver::new(addr, i2c)),
            _ => unreachable!(),
        };

        Ok(Self { model_driver })
    }

    pub fn initialize(&mut self) -> OsResult<()> {
        self.model_driver.initialize()
    }

    pub fn read_temperature(&self) -> OsResult<Temperature> {
        self.model_driver.read_temperature()
    }

    pub fn read_humidity(&self) -> OsResult<Humidity> {
        self.model_driver.read_humidity()
    }

    pub fn read_air_pressure(&self) -> OsResult<AirPressure> {
        self.model_driver.read_air_pressure()
    }
}

impl ModelDriver {
    pub fn initialize(&mut self) -> OsResult<()> {
        match self {
            Self::Bme280(d) => d.initialize(),
            Self::Bme690(d) => d.initialize(),
        }?;

        Ok(())
    }

    pub fn read_temperature(&self) -> OsResult<Temperature> {
        match self {
            Self::Bme280(d) => d.read_temperature(),
            Self::Bme690(d) => d.read_temperature(),
        }
    }

    pub fn read_humidity(&self) -> OsResult<Humidity> {
        match self {
            Self::Bme280(d) => d.read_humidity(),
            Self::Bme690(d) => d.read_humidity(),
        }
    }

    pub fn read_air_pressure(&self) -> OsResult<AirPressure> {
        match self {
            Self::Bme280(d) => d.read_air_pressure(),
            Self::Bme690(d) => d.read_air_pressure(),
        }
    }
}

impl crate::sysc::drivers::Driver for Bosch {}

impl crate::sysc::drivers::TemperatureDriver for Bosch {
    fn read(&self) -> OsResult<Temperature> {
        self.read_temperature()
    }

    fn is_internal(&self) -> bool {
        false
    }
}

impl crate::sysc::drivers::HumidityDriver for Bosch {
    fn read(&self) -> OsResult<u8> {
        self.read_humidity()
    }
}

impl crate::sysc::drivers::AirPressureDriver for Bosch {
    fn read(&self) -> OsResult<u16> {
        self.read_air_pressure()
    }
}

fn read_model(i2c: &SharedI2cDriver, addr: u8) -> OsResult<&'static str> {
    let mut buf = [0];
    i2c.write_read(addr, [REG_CHIP_ID], &mut buf)?;

    match buf[0] {
        CHIP_ID_BME280 => Ok("BME280"),
        CHIP_ID_BME6XX => {
            // only this family has the variant register
            i2c.write_read(addr, [REG_VARIANT_ID], &mut buf)?;

            match buf[0] {
                CHIP_ID_BME690_VARIANT => Ok("BME690"),
                o => Err(OsError::BmeUnknownVariant(o)),
            }
        }
        o => Err(OsError::BmeUnknownChipId(o)),
    }
}

/// Converts a 12-bit signed integer into a normal [i16].
const fn sign_extend_12(x: i16) -> i16 {
    (x << 4) >> 4
}
