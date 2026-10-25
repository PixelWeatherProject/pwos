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
    p1: f32,
    p2: f32,
    p3: f32,
    p4: f32,
    p5: f32,
    p6: f32,
    p7: f32,
    p8: f32,
    p9: f32,

    // Humidity
    h1: f32,
    h2: f32,
    h3: f32,
    h4: f32,
    h5: f32,
    h6: f32,
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
        let raw_t = self.read_raw_measurements()?.0;

        // temperature compensation
        // T[°C] = t_fine / 5120, so T[d°C] = t_fine / 512, rounded to nearest
        let t_fine = self.calc_t_fine(raw_t);
        let temperature_dc = (t_fine + 256) >> 9;

        // the sensor's range is -40..85°C, so this always fits
        Ok(temperature_dc as Temperature)
    }

    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        clippy::cast_precision_loss
    )]
    pub fn read_humidity(&self) -> OsResult<Humidity> {
        let (raw_t, raw_h, _) = self.read_raw_measurements()?;

        // humidity compensation
        let t_fine = self.calc_t_fine(raw_t) as f32;
        let mut h = t_fine - 76800.0;

        h = (raw_h - self.cal.h4.mul_add(64.0, self.cal.h5 / 16384.0 * h))
            * (self.cal.h2 / 65536.0
                * (self.cal.h6 / 67_108_864.0 * h)
                    .mul_add((self.cal.h3 / 67_108_864.0).mul_add(h, 1.0), 1.0));

        h = h * (1.0 - self.cal.h1 * h / 524_288.0);
        h = h.floor().clamp(0., 100.);

        Ok(h as u8)
    }

    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        clippy::cast_precision_loss
    )]
    pub fn read_air_pressure(&self) -> OsResult<AirPressure> {
        let (raw_t, _, raw_p) = self.read_raw_measurements()?;

        // pressure compensation
        let t_fine = self.calc_t_fine(raw_t) as f32;

        let mut p_var1 = (t_fine / 2.0) - 64000.0;
        let mut p_var2 = p_var1 * p_var1 * self.cal.p6 / 32768.0;
        p_var2 = (p_var1 * self.cal.p5).mul_add(2.0, p_var2);
        p_var2 = self.cal.p4.mul_add(65536.0, p_var2 / 4.0);
        p_var1 = self
            .cal
            .p2
            .mul_add(p_var1, self.cal.p3 * p_var1 * p_var1 / 524_288.0)
            / 524_288.0;
        p_var1 = (1.0 + p_var1 / 32768.0) * self.cal.p1;

        let pressure_pa = if p_var1 > 0.0 {
            let mut p = 1_048_576.0 - raw_p;
            p = (p - (p_var2 / 4096.0)) * 6250.0 / p_var1;
            p_var1 = self.cal.p9 * p * p / 2_147_483_648.0;
            p_var2 = p * self.cal.p8 / 32768.0;
            p + (p_var1 + p_var2 + self.cal.p7) / 16.0
        } else {
            0.0
        };

        let hpa = pressure_pa / 100.0;
        let hpa = hpa.floor().clamp(0., 65535.0) as u16;

        Ok(hpa)
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
        let mut calib_buf_1 = [0u8; 26]; // 0x88 to 0xA1
        self.write_read(Command::ReadCalibrationPart1, &mut calib_buf_1)?;

        let mut calib_buf_2 = [0u8; 7]; // 0xE1 to 0xE7
        self.write_read(Command::ReadCalibrationPart2, &mut calib_buf_2)?;

        self.cal = CalibrationData {
            t1: u16::from_le_bytes([calib_buf_1[0], calib_buf_1[1]]),
            t2: i16::from_le_bytes([calib_buf_1[2], calib_buf_1[3]]),
            t3: i16::from_le_bytes([calib_buf_1[4], calib_buf_1[5]]),

            p1: f32::from(u16::from_le_bytes([calib_buf_1[6], calib_buf_1[7]])),
            p2: f32::from(i16::from_le_bytes([calib_buf_1[8], calib_buf_1[9]])),
            p3: f32::from(i16::from_le_bytes([calib_buf_1[10], calib_buf_1[11]])),
            p4: f32::from(i16::from_le_bytes([calib_buf_1[12], calib_buf_1[13]])),
            p5: f32::from(i16::from_le_bytes([calib_buf_1[14], calib_buf_1[15]])),
            p6: f32::from(i16::from_le_bytes([calib_buf_1[16], calib_buf_1[17]])),
            p7: f32::from(i16::from_le_bytes([calib_buf_1[18], calib_buf_1[19]])),
            p8: f32::from(i16::from_le_bytes([calib_buf_1[20], calib_buf_1[21]])),
            p9: f32::from(i16::from_le_bytes([calib_buf_1[22], calib_buf_1[23]])),

            h1: f32::from(calib_buf_1[25]),
            h2: f32::from(i16::from_le_bytes([calib_buf_2[0], calib_buf_2[1]])),
            h3: f32::from(calib_buf_2[2]),
            h4: f32::from(sign_extend_12(
                (i16::from(calib_buf_2[3]) << 4) | (i16::from(calib_buf_2[4]) & 0x0F),
            )),
            h5: f32::from(sign_extend_12(
                (i16::from(calib_buf_2[5]) << 4) | (i16::from(calib_buf_2[4]) >> 4),
            )),
            h6: f32::from(calib_buf_2[6].cast_signed()),
        };

        Ok(())
    }

    /// Read the raw measurements from the sensor.
    ///
    /// These are not final and need to be compensated using the calibration data to get accurate readings.
    #[allow(clippy::cast_precision_loss)]
    fn read_raw_measurements(&self) -> OsResult<(i32, f32, f32)> {
        let mut buffer = [0; 8];
        self.write_read(Command::ReadMeasurementsStartRegister, &mut buffer)?;

        let raw_p = (u32::from(buffer[0]) << 12
            | u32::from(buffer[1]) << 4
            | u32::from(buffer[2]) >> 4) as f32;

        // 20-bit value, always fits into an `i32`
        let raw_t =
            i32::from(buffer[3]) << 12 | i32::from(buffer[4]) << 4 | i32::from(buffer[5]) >> 4;

        let raw_h = (u32::from(buffer[6]) << 8 | u32::from(buffer[7])) as f32;

        Ok((raw_t, raw_h, raw_p))
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
        let mut maybe_expected_init_time = self.expected_init_time.lock()?;

        if maybe_expected_init_time.is_none() {
            return Ok(());
        }

        // SAFETY: The above check for `Option::None` guarantees that this only runs of the option has a value.
        let expected_init_time = unsafe { maybe_expected_init_time.unwrap_unchecked() };

        if Instant::now() >= expected_init_time {
            return Ok(());
        }

        log::warn!("Sensor is not ready yet, waiting...");

        let min_wait_time = expected_init_time - Instant::now();
        std::thread::sleep(min_wait_time);

        *maybe_expected_init_time = None;
        drop(maybe_expected_init_time);
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
    fn read(&self) -> OsResult<f32> {
        self.read_temperature().map(|t| f32::from(t) / 10.0)
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
