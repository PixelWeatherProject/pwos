//! Driver for the Bosch BME280 temperature, humidity and air pressure sensor.
//!
//! ## Compatibility
//! **This driver only supports the BME280**. Other models like the BMP280, BME680 are **not** supported.
//!
//! These sensors work over the I2C protocol.

use super::EnvironmentSensor;
use crate::{
    re_esp,
    sysc::{OsError, OsResult},
};
use esp_idf_svc::hal::i2c::I2cDriver;
use pwmp_client::pwmp_msg::aliases::{AirPressure, Humidity, Temperature};

/// Driver handle for Bosch BME280 sensors.
pub struct BoschME280<'s> {
    i2c: I2cDriver<'s>,
    addr: u8,
    calibration: CalibrationData,
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
    t1: f32,
    t2: f32,
    t3: f32,

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

impl<'s> BoschME280<'s> {
    /// Known default address
    pub const DEV_ADDRS: [u8; 2] = [0x76, 0x77];

    const BUS_TIMEOUT: u32 = 1000;

    /// Initialize the driver with the given I2C driver handle.
    pub fn new_with_driver(driver: I2cDriver<'s>, addr: u8) -> Result<Self, OsError> {
        log::debug!("Loading driver");
        let mut dev = Self {
            i2c: driver,
            addr,
            calibration: CalibrationData::default(),
        };

        /*
         * Basic setup
         *
         * The BMx280 sensors require some additional setup to work.
         */

        // read the factory calibration data
        dev.read_calibration_data()?;

        // set temperature and humidity oversampling to 16x
        // also set the reading mode to normal
        dev.write(Command::SetHumidityCtlRegister(0b0000_0101))?;
        dev.write(Command::SetMeasurementCtlRegister(0b1011_0111))?;

        match dev.model()? {
            Some(model) => log::debug!("Detected '{model}'"),
            None => log::warn!("Device model is unknown and may not be supported"),
        }

        Ok(dev)
    }

    /// Detect the sensor model by reading the device ID register.
    fn model(&mut self) -> OsResult<Option<&'static str>> {
        let model = self.write_read_u8(Command::ReadIdRegister)?;

        match model {
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

        self.calibration = CalibrationData {
            t1: f32::from(u16::from_le_bytes([calib_buf_1[0], calib_buf_1[1]])),
            t2: f32::from(i16::from_le_bytes([calib_buf_1[2], calib_buf_1[3]])),
            t3: f32::from(i16::from_le_bytes([calib_buf_1[4], calib_buf_1[5]])),

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
            h4: f32::from(i16::from(calib_buf_2[3]) << 4 | (i16::from(calib_buf_2[4]) & 0x0F)),
            h5: f32::from(i16::from(calib_buf_2[5]) << 4 | (i16::from(calib_buf_2[4]) >> 4)),
            h6: f32::from(calib_buf_2[6].cast_signed()),
        };

        Ok(())
    }

    /// Read the raw measurements from the sensor.
    ///
    /// These are not final and need to be compensated using the calibration data to get accurate readings.
    #[allow(clippy::cast_precision_loss)]
    fn read_raw_measurements(&mut self) -> OsResult<(f32, f32, f32)> {
        let mut buffer = [0; 8];
        self.write_read(Command::ReadMeasurementsStartRegister, &mut buffer)?;

        let raw_p = (u32::from(buffer[0]) << 12
            | u32::from(buffer[1]) << 4
            | u32::from(buffer[2]) >> 4) as f32;

        let raw_t = (u32::from(buffer[3]) << 12
            | u32::from(buffer[4]) << 4
            | u32::from(buffer[5]) >> 4) as f32;

        let raw_h = (u32::from(buffer[6]) << 8 | u32::from(buffer[7])) as f32;

        Ok((raw_t, raw_h, raw_p))
    }

    /// Send a non-returning command to the sensor.
    ///
    /// If the command returns data, use [`write_read()`](Self::write_read) instead.
    fn write(&mut self, command: Command) -> OsResult<()> {
        let (cmd, len) = command.serialize();

        re_esp!(
            self.i2c.write(self.addr, &cmd[..len], Self::BUS_TIMEOUT),
            I2c
        )
    }

    /// Send a command to the sensor and read the response into the provided buffer.
    ///
    /// If the command does not return data, use [`write()`](Self::write) instead.
    fn write_read(&mut self, command: Command, buffer: &mut [u8]) -> OsResult<()> {
        let (cmd, len) = command.serialize();

        re_esp!(
            self.i2c
                .write_read(self.addr, &cmd[..len], buffer, Self::BUS_TIMEOUT,),
            I2c
        )
    }

    /// Send a command to the sensor and read a single byte response.
    ///
    /// This is a shorthand for:
    /// ```rust
    /// let mut buffer = [0; 1];
    /// self.write_read(command, &mut buffer)?;
    /// let raw = u8::from_be_bytes(buffer);
    /// ```
    fn write_read_u8(&mut self, command: Command) -> OsResult<u8> {
        let mut buffer = [0; 1];
        self.write_read(command, &mut buffer)?;

        let raw = u8::from_be_bytes(buffer);
        Ok(raw)
    }
}

impl EnvironmentSensor for BoschME280<'_> {
    fn read_temperature(&mut self) -> OsResult<Temperature> {
        let raw_t = self.read_raw_measurements()?.0;

        // temperature compensation
        let var1 = (raw_t / 16384.0 - self.calibration.t1 / 1024.0) * self.calibration.t2;
        let var2 = ((raw_t / 131_072.0 - self.calibration.t1 / 8192.0)
            * (raw_t / 131_072.0 - self.calibration.t1 / 8192.0))
            * self.calibration.t3;
        let t_fine = var1 + var2;
        let temperature_c = t_fine / 5120.0;

        Ok(temperature_c)
    }

    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    fn read_humidity(&mut self) -> OsResult<Humidity> {
        let (raw_t, raw_h, _) = self.read_raw_measurements()?;

        // humidity compensation
        let var1 = (raw_t / 16384.0 - self.calibration.t1 / 1024.0) * self.calibration.t2;
        let var2 = ((raw_t / 131_072.0 - self.calibration.t1 / 8192.0)
            * (raw_t / 131_072.0 - self.calibration.t1 / 8192.0))
            * self.calibration.t3;
        let t_fine = var1 + var2;

        let mut h = t_fine - 76800.0;

        h = (raw_h
            - self
                .calibration
                .h4
                .mul_add(64.0, self.calibration.h5 / 16384.0 * h))
            * (self.calibration.h2 / 65536.0
                * (self.calibration.h6 / 67_108_864.0 * h)
                    .mul_add((self.calibration.h3 / 67_108_864.0).mul_add(h, 1.0), 1.0));

        h = h * (1.0 - self.calibration.h1 * h / 524_288.0);
        h = h.floor().clamp(0., 100.);

        Ok(h as u8)
    }

    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    fn read_air_pressure(&mut self) -> OsResult<Option<AirPressure>> {
        let (raw_t, _, raw_p) = self.read_raw_measurements()?;

        // pressure compensation
        let var1 = (raw_t / 16384.0 - self.calibration.t1 / 1024.0) * self.calibration.t2;
        let var2 = ((raw_t / 131_072.0 - self.calibration.t1 / 8192.0)
            * (raw_t / 131_072.0 - self.calibration.t1 / 8192.0))
            * self.calibration.t3;
        let t_fine = var1 + var2;

        let mut p_var1 = (t_fine / 2.0) - 64000.0;
        let mut p_var2 = p_var1 * p_var1 * self.calibration.p6 / 32768.0;
        p_var2 += p_var1 * self.calibration.p5 * 2.0;
        p_var2 = self.calibration.p4.mul_add(65536.0, p_var2 / 4.0);
        p_var1 = self
            .calibration
            .p2
            .mul_add(p_var1, self.calibration.p3 * p_var1 * p_var1 / 524_288.0)
            / 524_288.0;
        p_var1 = (1.0 + p_var1 / 32768.0) * self.calibration.p1;

        let pressure_pa = if p_var1 > 0.0 {
            let mut p = 1_048_576.0 - raw_p;
            p = (p - (p_var2 / 4096.0)) * 6250.0 / p_var1;
            p_var1 = self.calibration.p9 * p * p / 2_147_483_648.0;
            p_var2 = p * self.calibration.p8 / 32768.0;
            p + (p_var1 + p_var2 + self.calibration.p7) / 16.0
        } else {
            0.0
        };

        let hpa = pressure_pa / 100.0;
        let hpa = hpa.floor().clamp(0., 65535.0) as u16;

        Ok(Some(hpa))
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
