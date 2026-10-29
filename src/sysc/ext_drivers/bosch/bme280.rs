use super::sign_extend_12;
use crate::sysc::{drivers::shared_i2c::SharedI2cDriver, OsResult, ReportableError};
use pwmp_client::pwmp_msg::aliases::{AirPressure, Humidity, Temperature};

const REG_CALIB_DATA_1: u8 = 0x88;
const REG_CALIB_DATA_2: u8 = 0xE1;
const REG_MEASUREMENTS: u8 = 0xF7;
const REG_MEASUREMENT_CTL: u8 = 0xF4;
const REG_HUMIDITY_CTL: u8 = 0xF2;

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

pub struct Driver {
    /// Factory calibration data read from the sensor.
    cal: CalibrationData,

    /// I2C driver handle for communication with the sensor.
    i2c: SharedI2cDriver,

    /// I2C address of the sensor.
    addr: u8,
}

impl Driver {
    pub fn new(addr: u8, i2c: SharedI2cDriver) -> Self {
        Self {
            cal: CalibrationData::default(),
            i2c,
            addr,
        }
    }

    pub fn initialize(&mut self) -> OsResult<()> {
        self.read_calibration_data()?;

        // set temperature and humidity oversampling to 16x
        self.i2c.write(self.addr, [REG_HUMIDITY_CTL, 0b0000_0101])?;

        // also set the reading mode to normal
        self.i2c
            .write(self.addr, [REG_MEASUREMENT_CTL, 0b1011_0111])?;

        Ok(())
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

    /// Read the factory calibration data from the sensor.
    fn read_calibration_data(&mut self) -> OsResult<()> {
        let mut buf1 = [0u8; 26];
        self.i2c
            .write_read(self.addr, [REG_CALIB_DATA_1], &mut buf1)?;

        let mut buf2 = [0u8; 7];
        self.i2c
            .write_read(self.addr, [REG_CALIB_DATA_2], &mut buf2)?;

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
        self.i2c.write(self.addr, [REG_MEASUREMENT_CTL, 0])
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

    /// Read the raw measurements from the sensor.
    ///
    /// These are not final and need to be compensated using the calibration data to get accurate readings.
    fn read_raw_measurements(&self) -> OsResult<RawMeasurements> {
        let mut buf = [0; 8];
        self.i2c
            .write_read(self.addr, [REG_MEASUREMENTS], &mut buf)?;

        // pressure and temperature are 20-bit, humidity is 16-bit, so all fit into an `i32`
        let raw_20bit =
            |b: &[u8]| i32::from(b[0]) << 12 | i32::from(b[1]) << 4 | i32::from(b[2]) >> 4;

        Ok(RawMeasurements {
            pressure: raw_20bit(&buf[0..3]),
            temperature: raw_20bit(&buf[3..6]),
            humidity: i32::from(buf[6]) << 8 | i32::from(buf[7]),
        })
    }
}

impl Drop for Driver {
    fn drop(&mut self) {
        log::debug!("Attempting to enter sleep mode");

        // Note: if this fails, the next boot will likely fail
        self.enter_sleep_mode().report("Failed to enter sleep mode");
    }
}
