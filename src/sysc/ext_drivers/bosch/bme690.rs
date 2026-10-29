use std::time::Duration;

use super::sign_extend_12;
use crate::sysc::{drivers::shared_i2c::SharedI2cDriver, OsError, OsResult, ReportableError};
use pwmp_client::pwmp_msg::aliases::{AirPressure, Humidity, Temperature};

const REG_CALIB_DATA_1: u8 = 0x8A;
const REG_CALIB_DATA_2: u8 = 0xE1;
const REG_MEASUREMENT_CTL: u8 = 0x74;
const REG_HUMIDITY_CTL: u8 = 0x72;
const REG_MEAS_STATUS_0: u8 = 0x1D;

/// `ctrl_meas` value with temperature and pressure oversampling set to 16x, in sleep mode.
const CTRL_MEAS_OSRS_16X: u8 = 0b1011_0100;

/// `mode<1:0>` value for forced mode.
const MODE_FORCED: u8 = 0b01;

/// `meas_status_0` bit set when new data is available.
const STATUS_NEW_DATA: u8 = 1 << 7;

/// `meas_status_0` bit set while a measurement is in progress.
const STATUS_MEASURING: u8 = 1 << 5;

// with 16x oversampling on all channels, a measurement takes roughly 100ms
const EXPECTED_DURATION: Duration = Duration::from_millis(100);
const POLL_INTERVAL: Duration = Duration::from_millis(5);
const MAX_POLLS: u32 = 20;

/// Sensor calibration data.
///
/// Since every sensor has a slightly different accuracy, a calibration step is performed at the factory.
/// The calibration data is them stored in the sensor's memory after calibration.
///
/// The values are used to compensate the raw measurements read from the sensor, which results in more accurate readings.
/// The compensation formulas are provided in the BME690 datasheet.
#[derive(Default)]
struct CalibrationData {
    // Temperature
    t1: u16,
    t2: u16,
    t3: i8,

    // Pressure
    p1: u16,
    p2: u16,
    p3: i8,
    p4: i8,
    p5: i16,
    p6: i16,
    p7: i8,
    p8: i8,
    p9: i16,
    p10: i8,
    p11: i8,

    // Humidity
    h1: i16,
    h2: i8,
    h3: u8,
    h4: i8,
    h5: i16,
    h6: u8,
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

        // humidity oversampling 16x, then temperature and pressure oversampling 16x (staying in sleep mode)
        // the datasheet recommends setting `osrs_h` first, followed by `osrs_t`/`osrs_p`, in one transaction
        self.i2c.write(
            self.addr,
            [
                REG_HUMIDITY_CTL,
                0b0000_0101,
                REG_MEASUREMENT_CTL,
                CTRL_MEAS_OSRS_16X,
            ],
        )?;

        Ok(())
    }

    /// Read the temperature in decidegrees Celsius.
    #[allow(clippy::cast_possible_truncation)]
    pub fn read_temperature(&self) -> OsResult<Temperature> {
        let raw = self.read_raw_measurements()?;

        // `t_lin` is T[°C] in Q16, so T[d°C] = t_lin * 10 / 2^16, rounded to nearest
        let t_lin = self.calc_t_lin(raw.temperature);
        let temperature_dc = (t_lin * 10 + (1 << 15)) >> 16;

        // the sensor's range is -40..85°C, so this always fits
        Ok(temperature_dc as Temperature)
    }

    /// Read the relative humidity in percent.
    ///
    /// Uses the integer compensation formula from the datasheet, evaluated in 64 bits.
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    pub fn read_humidity(&self) -> OsResult<Humidity> {
        let raw = self.read_raw_measurements()?;

        // the humidity formula is the BME280 one, which expects `t_fine = T[°C] * 5120`
        let t_lin = i64::from(self.calc_t_lin(raw.temperature));
        let t_fine = (t_lin * 5) >> 6;

        let h1 = i64::from(self.cal.h1);
        let h2 = i64::from(self.cal.h2);
        let h3 = i64::from(self.cal.h3);
        let h4 = i64::from(self.cal.h4);
        let h5 = i64::from(self.cal.h5);
        let h6 = i64::from(self.cal.h6);

        let x = t_fine - 76800;
        let offset = ((i64::from(raw.humidity) << 14) - (h1 << 20) - (h2 * x) + 16384) >> 15;
        let scale =
            (((((x * h4) >> 10) * (((x * h3) >> 11) + 32768)) >> 10) + 2_097_152) * h5 + 8192;
        let mut h = offset * (scale >> 14);
        h -= ((((h >> 15) * (h >> 15)) >> 7) * h6) >> 4;

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
        let t_lin = i64::from(self.calc_t_lin(raw.temperature));

        // the datasheet formula expects a 24-bit ADC value
        let adc = i64::from(raw.pressure) << 4;

        let p1 = i64::from(self.cal.p1);
        let p2 = i64::from(self.cal.p2);
        let p3 = i64::from(self.cal.p3);
        let p4 = i64::from(self.cal.p4);
        let p5 = i64::from(self.cal.p5);
        let p6 = i64::from(self.cal.p6);
        let p7 = i64::from(self.cal.p7);
        let p8 = i64::from(self.cal.p8);
        let p9 = i64::from(self.cal.p9);
        let p10 = i64::from(self.cal.p10);
        let p11 = i64::from(self.cal.p11);

        let t_lin2 = t_lin * t_lin;
        let t_lin3 = ((t_lin2 >> 6) * t_lin) >> 8;

        let offset =
            (p1 << 47) + ((p4 * t_lin3) >> 5) + ((p3 * t_lin2) << 4) + ((p2 * t_lin) << 22);
        let sensitivity = ((p5 - 16384) << 46)
            + ((p8 * t_lin3) >> 5)
            + ((p7 * t_lin2) << 2)
            + (((p6 - 16384) * t_lin) << 21);

        let var1 = (sensitivity >> 24) * adc;
        let var2 = (((p10 * t_lin) + (p9 << 16)) * adc) >> 13;
        // divide by 10 before multiplying to avoid overflowing
        let var3 = ((adc * (var2 / 10)) >> 9) * 10;
        let var4 = (((p11 * adc * adc) >> 16) * adc) >> 7;

        let p = (offset >> 2) + var1 + var3 + var4;

        // `p` is in units of 2^-42 Pa, so divide by 2^42 * 100 to get hPa
        let hpa = ((p >> 40) / 400).clamp(0, i64::from(u16::MAX));
        Ok(hpa as AirPressure)
    }

    /// Read the factory calibration data from the sensor.
    fn read_calibration_data(&mut self) -> OsResult<()> {
        let mut buf1 = [0u8; 22]; // 0x8A to 0x9F
        self.i2c
            .write_read(self.addr, [REG_CALIB_DATA_1], &mut buf1)?;

        let mut buf2 = [0u8; 10]; // 0xE1 to 0xEA
        self.i2c
            .write_read(self.addr, [REG_CALIB_DATA_2], &mut buf2)?;

        let u16_at = |buf: &[u8], i: usize| u16::from_le_bytes([buf[i], buf[i + 1]]);
        let i16_at = |buf: &[u8], i: usize| i16::from_le_bytes([buf[i], buf[i + 1]]);

        self.cal = CalibrationData {
            t1: u16_at(&buf2, 8),
            t2: u16_at(&buf1, 0),
            t3: buf1[2].cast_signed(),

            p1: u16_at(&buf1, 10),
            p2: u16_at(&buf1, 12),
            p3: buf1[14].cast_signed(),
            p4: buf1[15].cast_signed(),
            p5: i16_at(&buf1, 4),
            p6: i16_at(&buf1, 6),
            p7: buf1[8].cast_signed(),
            p8: buf1[9].cast_signed(),
            p9: i16_at(&buf1, 18),
            p10: buf1[20].cast_signed(),
            p11: buf1[21].cast_signed(),

            h1: sign_extend_12((i16::from(buf2[2]) << 4) | (i16::from(buf2[1]) & 0x0F)),
            h2: buf2[3].cast_signed(),
            h3: buf2[5],
            h4: buf2[4].cast_signed(),
            h5: sign_extend_12((i16::from(buf2[0]) << 4) | (i16::from(buf2[1]) >> 4)),
            h6: buf2[6],
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
    /// - All oversampling and other settings.
    ///
    fn enter_sleep_mode(&self) -> OsResult<()> {
        self.i2c.write(self.addr, [REG_MEASUREMENT_CTL, 0])
    }

    /// Calculate the `t_lin` value from the raw temperature measurement, which is used for compensation of all measurements.
    ///
    /// The result is the temperature in degrees Celsius, in Q16 fixed point (`t_lin = T[°C] * 2^16`).
    /// Uses the 64-bit integer compensation formula from the datasheet.
    #[allow(clippy::cast_possible_truncation)]
    fn calc_t_lin(&self, raw_t: i32) -> i32 {
        let t1 = i64::from(self.cal.t1);
        let t2 = i64::from(self.cal.t2);
        let t3 = i64::from(self.cal.t3);

        // the datasheet formula expects a 24-bit ADC value
        let adc = i64::from(raw_t) << 4;

        let var1 = adc - (t1 << 8);
        let var2 = var1 * t2;
        let var3 = var1 * var1 * t3;

        // the sensor's range is -40..85°C, so the result always fits into an `i32`
        (((var2 << 18) + var3) / (1 << 32)) as i32
    }

    /// Read the raw measurements from the sensor.
    ///
    /// This triggers a single forced mode measurement and blocks until it's finished.
    /// The results are not final and need to be compensated using the calibration data to get accurate readings.
    fn read_raw_measurements(&self) -> OsResult<RawMeasurements> {
        // trigger a single measurement, the sensor returns to sleep mode afterwards
        self.i2c.write(
            self.addr,
            [REG_MEASUREMENT_CTL, CTRL_MEAS_OSRS_16X | MODE_FORCED],
        )?;

        log::debug!("Waiting for device to complete measurement");
        std::thread::sleep(EXPECTED_DURATION);

        // field 0: status (0x1D), sub-measurement index (0x1E), pressure (0x1F..=0x21),
        // temperature (0x22..=0x24), humidity (0x25..=0x26)
        let mut buf = [0u8; 10];

        for i in 0..MAX_POLLS {
            self.i2c
                .write_read(self.addr, [REG_MEAS_STATUS_0], &mut buf)?;

            let status = buf[0];
            if status & STATUS_NEW_DATA != 0 && status & STATUS_MEASURING == 0 {
                // pressure and temperature are 20-bit, humidity is 16-bit, so all fit into an `i32`
                let raw_20bit =
                    |b: &[u8]| i32::from(b[0]) << 12 | i32::from(b[1]) << 4 | i32::from(b[2]) >> 4;

                return Ok(RawMeasurements {
                    pressure: raw_20bit(&buf[2..5]),
                    temperature: raw_20bit(&buf[5..8]),
                    humidity: i32::from(buf[8]) << 8 | i32::from(buf[9]),
                });
            }

            log::debug!("Measurement not ready, polling ({i}/{MAX_POLLS})");
            std::thread::sleep(POLL_INTERVAL);
        }

        Err(OsError::BmeMeasurementTimeout)
    }
}

impl Drop for Driver {
    fn drop(&mut self) {
        log::debug!("Attempting to enter sleep mode");

        // Note: if this fails, the next boot will likely fail
        self.enter_sleep_mode().report("Failed to enter sleep mode");
    }
}
