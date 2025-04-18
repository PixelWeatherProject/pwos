//! A driver for reading the battery supply voltage using the node's ADC.

use super::{OsError, OsResult};
use esp_idf_svc::{
    hal::{
        adc::{
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
/// See the attenuation table [here](https://docs.espressif.com/projects/esp-idf/en/v4.4/esp32s3/api-reference/peripherals/adc.html#adc-attenuation).
// With the resistor configuration below, the maximum ADC input
// at 4.2V should be 970mV, so 0 attenuation is almost the correct
// choice. Due to the high resistor values, even if this higher voltage enters
// the ADC, the current should be very limited, i.e. no damage should be done.
const ATTEN: adc_atten_t = 0;
/// Value of the first resistor of the voltage divider
const R1: Decimal = dec!(1_000_000); // 1MOhm
/// Value of the second resistor of the voltage divider
const R2: Decimal = dec!(300_000); // 300kOhm
/// ADC channel configuration
const CONFIG: AdcChannelConfig = AdcChannelConfig {
    attenuation: ATTEN,              /* refer to the attenuation value above  */
    calibration: Calibration::Curve, /* ADC auto-calibration type */
    resolution: Resolution::Resolution12Bit, /* ADC resolution */
};
/// Critical voltage value that's still higher than the minimum supply voltage for the ESP32
pub const CRITICAL_VOLTAGE: Decimal = dec!(3.22);

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
        let result = (volts * (R1 + R2)) / R2;

        Ok(result.trunc_with_scale(2))
    }

    /// Read the raw ADC value.
    fn read_raw(&mut self, samples: u8) -> OsResult<u16> {
        let mut avg = dec!(0);

        for _ in 0..samples {
            avg += Decimal::from(self.adc.read_raw(&mut self.ch)?);
            sleep(Duration::from_millis(1));
        }

        avg /= Decimal::from(samples);
        u16::try_from(avg).map_err(|_| OsError::DecimalConversion)
    }
}
