//! A driver for reading the battery supply voltage using the node's ADC.

use super::{OsError, OsResult};
use crate::{os_debug, os_error};
use esp_idf_svc::{
    hal::{
        adc::{
            attenuation::DB_11,
            oneshot::{
                config::{AdcChannelConfig, Calibration},
                AdcChannelDriver, AdcDriver,
            },
            Resolution, ADC1,
        },
        gpio::Gpio2,
    },
    sys::adc_atten_t,
};
use pwmp_client::pwmp_msg::{dec, Decimal};
use std::{rc::Rc, thread::sleep, time::Duration};

/// GPIO pin where the output of the voltage divider is connected
type BatteryGpio = Gpio2;
/// The ADC hardware
type BatteryAdc = ADC1;
/// Alias for the ADC driver
type BatteryAdcDriver = AdcDriver<'static, BatteryAdc>;
/// Alias for the ADC channel driver
type BatteryAdcChannelDriver = AdcChannelDriver<'static, BatteryGpio, Rc<BatteryAdcDriver>>;

/// Input signal attenuation level
const ATTEN: adc_atten_t = DB_11;
/// Value of the first resistor of the voltage divider
const R1: Decimal = dec!(20_000); // 20kOhm
/// Value of the second resistor of the voltage divider
const R2: Decimal = dec!(6_800); // 6.8kOhm
/// ADC channel configuration
const CONFIG: AdcChannelConfig = AdcChannelConfig {
    attenuation: ATTEN,              /* refer to the attenuation value above  */
    calibration: Calibration::Curve, /* ADC auto-calibration type */
    resolution: Resolution::Resolution12Bit, /* ADC resolution */
};
/// Maximum voltage we expect
const MAX_VOLTAGE: Decimal = dec!(5.00);
/// Minimum voltage we expect
const MIN_VOLTAGE: Decimal = dec!(2.80);
/// Critical voltage value that's still higher than the minimum supply voltage for the ESP32
pub const CRITICAL_VOLTAGE: Decimal = dec!(2.70);

/// Battery voltage measurement driver.
pub struct Battery {
    /// ADC driver handle
    adc: Rc<BatteryAdcDriver>,

    /// ADC channel driver handle
    ch: BatteryAdcChannelDriver,
}

impl Battery {
    /// Initiliaze a new instance of this driver using the given peripheral handles.
    pub fn new(adc: BatteryAdc, gpio: BatteryGpio) -> OsResult<Self> {
        let adc = Rc::new(BatteryAdcDriver::new(adc)?);
        let ch = BatteryAdcChannelDriver::new(Rc::clone(&adc), gpio, &CONFIG)?;

        Ok(Self { adc, ch })
    }

    /// Read the ADC value and calculate the voltage.
    pub fn read(&mut self, samples: u8) -> OsResult<Decimal> {
        let raw = self.read_raw(samples)?;
        let volts = Decimal::from(self.adc.raw_to_mv(&self.ch, raw)?) / dec!(1000);
        let mut result = (volts * (R1 + R2)) / (R2);

        if result > MAX_VOLTAGE || result < MIN_VOLTAGE {
            os_debug!("Abnormal battery voltage result, attempting fix");

            // swap R1 and R2
            result = (volts * (R2 + R1)) / (R1);

            if result > MAX_VOLTAGE {
                os_error!("Abnormal battery voltage");
                return Err(OsError::IllegalBatteryVoltage);
            }

            os_debug!("Detected swapped R1/R2 values, fix successful");
        }

        Ok(result.trunc_with_scale(2))
    }

    /// Read the raw ADC value.
    fn read_raw(&mut self, samples: u8) -> OsResult<u16> {
        let mut avg = dec!(0);

        for _ in 0..samples {
            avg += Decimal::from(self.adc.read_raw(&mut self.ch)?);
            sleep(Duration::from_millis(10));
        }

        avg /= Decimal::from(samples);
        u16::try_from(avg).map_err(|_| OsError::DecimalConversion)
    }
}
