//! Driver for the Bosch BME280 temperature, humidity and air pressure sensor.
//!
//! ## Compatibility
//! **This driver only supports the BME280**. Other models like the BMP280, BME680 are **not** supported.
//!
//! ## Measurement parameters
//! This driver uses the maximum precision and multisampling (16x).
//! This cannot be changed and is not intended to be configurable.
//!
//! These sensors work over the I2C protocol.

use crate::sysc::{
    drivers::{
        shared_i2c::SharedI2cDriver, AirPressureDriver, Driver, HumidityDriver, TemperatureDriver,
    },
    OsError, OsResult, ReportableError,
};
use pwmp_client::pwmp_msg::aliases::{AirPressure, Humidity, Temperature};
use std::{
    sync::Mutex,
    time::{Duration, Instant},
};

/// Driver handle for Bosch BME280 sensors.
pub struct BoschME280 {
    /// I2C driver handle for communication with the sensor.
    i2c: SharedI2cDriver,

    /// I2C address of the sensor.
    addr: u8,

    /// Factory calibration data read from the sensor.
    cal: CalibrationData,

    /// Time at which the sensor should be initialized.
    expected_init_time: Mutex<Option<Instant>>,
}

/// Commands for BME280 sensors
#[derive(Clone, Copy)]
enum Command {
    /// Read calibration data part 1
    ReadCalibrationPart1,

    /// Read calibration data part 2
    ReadCalibrationPart2,

    /// Set humidity control register
    SetHumidityCtlRegister(u8),

    /// Set temperature control register
    SetMeasurementCtlRegister(u8),

    /// Read the device ID register
    ReadIdRegister,

    /// Read the first data register.
    ReadMeasurementsStartRegister,
}

/// Sensor calibration data.
///
/// Since every sensor has a slightly different accuracy, a calibration step is performed at the factory.
/// The calibration data is them stored in the sensor's memory after calibration.
///
/// The values are used to compensate the raw measurements read from the sensor, which results in more accurate readings.
/// The compensation formulas are provided in the BME280 datasheet.
#[derive(Default)]
struct CalibrationData {
    // Temperature
    t1: u16,
    t2: i16,
    t3: i16,

    // Pressure
    p1: u16,
    p2: i16,
    p3: i16,
    p4: i16,
    p5: i16,
    p6: i16,
    p7: i16,
    p8: i16,
    p9: i16,

    // Humidity
    h1: u8,
    h2: i16,
    h3: u8,
    h4: i16,
    h5: i16,
    h6: i8,
}

/// Raw, uncompensated measurements read from the sensor.
struct RawMeasurements {
    temperature: i32,
    humidity: i32,
    pressure: i32,
}

impl BoschME280 {
    /// Known default address
    pub const DEV_ADDRS: [u8; 2] = [0x76, 0x77];

    const BUS_TIMEOUT: u32 = 1000;

    /// Initialize the driver with the given I2C driver handle.
    pub fn new_with_driver(driver: SharedI2cDriver, addr: u8) -> Result<Self, OsError> {
        log::debug!("Loading driver");
        let mut dev = Self {
            i2c: driver,
            addr,
            cal: CalibrationData::default(),
            expected_init_time: Mutex::default(),
        };

        /*
         * Basic setup
         *
         * The BMx280 sensors require some additional setup to work.
         */

        // read the factory calibration data
        dev.read_calibration_data()?;

        match dev.model()? {
            Some(model) => log::debug!("Detected '{model}'"),
            None => log::warn!("Device model is unknown and may not be supported"),
        }

        // set temperature and humidity oversampling to 16x
        // also set the reading mode to normal
        dev.write(Command::SetHumidityCtlRegister(0b0000_0101))?;
        dev.write(Command::SetMeasurementCtlRegister(0b1011_0111))?;

        // due to the highest quality and sampling settings, measurement takes
        // about 112ms, so we expect the sensor to be initialized 120ms after
        // this constructor runs
        *dev.expected_init_time.get_mut()? = Some(Instant::now() + Duration::from_millis(120));

        Ok(dev)
    }

    /// Read the temperature in decidegrees Celsius.
    #[allow(clippy::cast_possible_truncation)]
    pub fn read_temperature(&self) -> OsResult<Temperature> {
        let raw = self.read_raw_measurements()?;

        // temperature compensation
        // T[°C] = t_fine / 5120, so T[d°C] = t_fine / 512, rounded to nearest
        let t_fine = self.calc_t_fine(raw.temperature);
        let temperature_dc = (t_fine + 256) >> 9;

        // the sensor's range is -40..85°C, so this always fits
        Ok(temperature_dc as Temperature)
    }

    /// Read the relative humidity in percent.
    ///
    /// Uses the 32-bit integer compensation formula from the datasheet.
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    pub fn read_humidity(&self) -> OsResult<Humidity> {
        let raw = self.read_raw_measurements()?;
        let t_fine = self.calc_t_fine(raw.temperature);

        let h1 = i32::from(self.cal.h1);
        let h2 = i32::from(self.cal.h2);
        let h3 = i32::from(self.cal.h3);
        let h4 = i32::from(self.cal.h4);
        let h5 = i32::from(self.cal.h5);
        let h6 = i32::from(self.cal.h6);

        let x = t_fine - 76800;
        let offset = ((raw.humidity << 14) - (h4 << 20) - (h5 * x) + 16384) >> 15;
        let scale =
            (((((x * h6) >> 10) * (((x * h3) >> 11) + 32768)) >> 10) + 2_097_152) * h2 + 8192;
        let mut h = offset * (scale >> 14);
        h -= ((((h >> 15) * (h >> 15)) >> 7) * h1) >> 4;

        // `h` is %RH scaled by 2^22, so after clamping to 0..=100% the integer part always fits
        let h = h.clamp(0, 100 << 22) >> 22;
        Ok(h as Humidity)
    }

    /// Read the air pressure in hectopascals.
    ///
    /// Uses the 64-bit integer compensation formula from the datasheet.
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    pub fn read_air_pressure(&self) -> OsResult<AirPressure> {
        let raw = self.read_raw_measurements()?;
        let t_fine = i64::from(self.calc_t_fine(raw.temperature));

        let p1 = i64::from(self.cal.p1);
        let p2 = i64::from(self.cal.p2);
        let p3 = i64::from(self.cal.p3);
        let p4 = i64::from(self.cal.p4);
        let p5 = i64::from(self.cal.p5);
        let p6 = i64::from(self.cal.p6);
        let p7 = i64::from(self.cal.p7);
        let p8 = i64::from(self.cal.p8);
        let p9 = i64::from(self.cal.p9);

        let mut var1 = t_fine - 128_000;
        let mut var2 = var1 * var1 * p6 + ((var1 * p5) << 17) + (p4 << 35);
        var1 = ((var1 * var1 * p3) >> 8) + ((var1 * p2) << 12);
        var1 = (((1 << 47) + var1) * p1) >> 33;

        if var1 == 0 {
            // avoid division by zero
            return Ok(0);
        }

        let mut p = 1_048_576 - i64::from(raw.pressure);
        p = (((p << 31) - var2) * 3125) / var1;
        var1 = (p9 * (p >> 13) * (p >> 13)) >> 25;
        var2 = (p8 * p) >> 19;
        p = ((p + var1 + var2) >> 8) + (p7 << 4);

        // `p` is in Q24.8 Pa, so divide by 256 * 100 to get hPa
        let hpa = (p / 25_600).clamp(0, i64::from(u16::MAX));
        Ok(hpa as AirPressure)
    }

    /// Detect the sensor model by reading the device ID register.
    pub fn model(&self) -> OsResult<Option<&'static str>> {
        let mut buffer = [0u8; 1];
        self.write_read(Command::ReadIdRegister, &mut buffer)?;

        match buffer[0] {
            0x60 => Ok(Some("BME280")),
            _ => Ok(None),
        }
    }

    /// Read the factory calibration data from the sensor.
    fn read_calibration_data(&mut self) -> OsResult<()> {
        let mut buf1 = [0u8; 26]; // 0x88 to 0xA1
        self.write_read(Command::ReadCalibrationPart1, &mut buf1)?;

        let mut buf2 = [0u8; 7]; // 0xE1 to 0xE7
        self.write_read(Command::ReadCalibrationPart2, &mut buf2)?;

        let u16_at = |buf: &[u8], i: usize| u16::from_le_bytes([buf[i], buf[i + 1]]);
        let i16_at = |buf: &[u8], i: usize| i16::from_le_bytes([buf[i], buf[i + 1]]);

        self.cal = CalibrationData {
            t1: u16_at(&buf1, 0),
            t2: i16_at(&buf1, 2),
            t3: i16_at(&buf1, 4),

            p1: u16_at(&buf1, 6),
            p2: i16_at(&buf1, 8),
            p3: i16_at(&buf1, 10),
            p4: i16_at(&buf1, 12),
            p5: i16_at(&buf1, 14),
            p6: i16_at(&buf1, 16),
            p7: i16_at(&buf1, 18),
            p8: i16_at(&buf1, 20),
            p9: i16_at(&buf1, 22),

            h1: buf1[25],
            h2: i16_at(&buf2, 0),
            h3: buf2[2],
            h4: sign_extend_12((i16::from(buf2[3]) << 4) | (i16::from(buf2[4]) & 0x0F)),
            h5: sign_extend_12((i16::from(buf2[5]) << 4) | (i16::from(buf2[4]) >> 4)),
            h6: buf2[6].cast_signed(),
        };

        Ok(())
    }

    /// Read the raw measurements from the sensor.
    ///
    /// These are not final and need to be compensated using the calibration data to get accurate readings.
    fn read_raw_measurements(&self) -> OsResult<RawMeasurements> {
        let mut buf = [0; 8];
        self.write_read(Command::ReadMeasurementsStartRegister, &mut buf)?;

        // pressure and temperature are 20-bit, humidity is 16-bit, so all fit into an `i32`
        let raw_20bit =
            |b: &[u8]| i32::from(b[0]) << 12 | i32::from(b[1]) << 4 | i32::from(b[2]) >> 4;

        Ok(RawMeasurements {
            pressure: raw_20bit(&buf[0..3]),
            temperature: raw_20bit(&buf[3..6]),
            humidity: i32::from(buf[6]) << 8 | i32::from(buf[7]),
        })
    }

    /// Shuts down the sensor.
    ///
    /// This method should only ever be used internally, specifically in the implementation of [`Drop`].
    /// This is also the reason why it's private.
    ///
    /// Due to the use in [`Drop::drop()`], this method needs to take a mutable reference instead of taking
    /// ownership of the instance.
    ///
    /// # Caveats
    /// - Any operation after this will likely fail.
    /// - All oversampling and other settings set by [`new_with_driver`](Self::new_with_driver) **will be erased**.
    ///
    fn enter_sleep_mode(&self) -> OsResult<()> {
        self.write(Command::SetMeasurementCtlRegister(0))
    }

    /// Calculate the `t_fine` value from the raw temperature measurement, which is used for compensation of all measurements.
    ///
    /// Uses the 32-bit integer compensation formula from the datasheet.
    fn calc_t_fine(&self, raw_t: i32) -> i32 {
        let t1 = i32::from(self.cal.t1);
        let t2 = i32::from(self.cal.t2);
        let t3 = i32::from(self.cal.t3);

        let var1 = (((raw_t >> 3) - (t1 << 1)) * t2) >> 11;
        let var2 = (((((raw_t >> 4) - t1) * ((raw_t >> 4) - t1)) >> 12) * t3) >> 14;
        var1 + var2
    }

    /// Send a non-returning command to the sensor.
    ///
    /// If the command returns data, use [`write_read()`](Self::write_read) instead.
    fn write(&self, command: Command) -> OsResult<()> {
        self.wait_until_ready()?;
        let (cmd, len) = command.serialize();
        self.i2c.write(self.addr, &cmd[..len], Self::BUS_TIMEOUT)
    }

    /// Send a command to the sensor and read the response into the provided buffer.
    ///
    /// If the command does not return data, use [`write()`](Self::write) instead.
    fn write_read(&self, command: Command, buffer: &mut [u8]) -> OsResult<()> {
        self.wait_until_ready()?;
        let (cmd, len) = command.serialize();
        self.i2c
            .write_read(self.addr, &cmd[..len], buffer, Self::BUS_TIMEOUT)
    }

    /// Blocks the caller until the sensor is ready.
    fn wait_until_ready(&self) -> OsResult<()> {
        // the lock is held while sleeping, so concurrent callers also wait
        let mut expected_init_time = self.expected_init_time.lock()?;

        let Some(deadline) = expected_init_time.take() else {
            return Ok(());
        };

        if let Some(remaining) = deadline.checked_duration_since(Instant::now()) {
            log::warn!("Sensor is not ready yet, waiting...");
            std::thread::sleep(remaining);
        }

        Ok(())
    }
}

impl Drop for BoschME280 {
    fn drop(&mut self) {
        log::debug!("Attempting to enter sleep mode");

        // Note: if this fails, the next boot will likely fail
        self.enter_sleep_mode().report("Failed to enter sleep mode");
    }
}

impl Driver for BoschME280 {}

impl TemperatureDriver for BoschME280 {
    fn read(&self) -> OsResult<Temperature> {
        self.read_temperature()
    }

    fn is_internal(&self) -> bool {
        false
    }
}

impl HumidityDriver for BoschME280 {
    fn read(&self) -> OsResult<u8> {
        self.read_humidity()
    }
}

impl AirPressureDriver for BoschME280 {
    fn read(&self) -> OsResult<u16> {
        self.read_air_pressure()
    }
}

impl Command {
    /// Serialize the command into raw bytes and a length.
    const fn serialize(self) -> ([u8; 2], usize) {
        match self {
            Self::ReadCalibrationPart1 => ([0x88, 0], 1),
            Self::ReadCalibrationPart2 => ([0xE1, 0], 1),
            Self::SetHumidityCtlRegister(d) => ([0xF2, d], 2),
            Self::SetMeasurementCtlRegister(d) => ([0xF4, d], 2),
            Self::ReadIdRegister => ([0xD0, 0], 1),
            Self::ReadMeasurementsStartRegister => ([0xF7, 0], 1),
        }
    }
}

/// Converts a 12-bit signed integer into a normal [i16].
const fn sign_extend_12(x: i16) -> i16 {
    (x << 4) >> 4
}
