//! A driver for reading the battery supply voltage using the node's ADC.

use super::OsResult;
use crate::re_esp;
use esp_idf_svc::{
    hal::{
        adc::{
            oneshot::{
                config::{AdcChannelConfig, Calibration},
                AdcChannelDriver, AdcDriver,
            },
            Resolution,
        },
        gpio::ADCPin,
        peripheral::Peripheral,
    },
    sys::adc_atten_t,
};
use std::rc::Rc;

/// Alias for the ADC driver
type BatteryAdcDriver<Gpio: ADCPin> = Rc<AdcDriver<'static, Gpio::Adc>>;
/// Alias for the ADC channel driver
type BatteryAdcChannelDriver<Gpio: ADCPin> =
    AdcChannelDriver<'static, Gpio, BatteryAdcDriver<Gpio>>;

/// Input signal attenuation level
/// See the attenuation table [here](https://docs.espressif.com/projects/esp-idf/en/v4.4/esp32s3/api-reference/peripherals/adc.html#adc-attenuation).
// With the resistor configuration below, the maximum ADC input
// at 4.2V should be 970mV, so 0 attenuation is almost the correct
// choice. Due to the high resistor values, even if this higher voltage enters
// the ADC, the current should be very limited, i.e. no damage should be done.
const ATTEN: adc_atten_t = 0;
/// Value of the first resistor of the voltage divider
const R1: f32 = 1_000_000.; // 1MOhm
/// Value of the second resistor of the voltage divider
const R2: f32 = 300_000.; // 300kOhm
/// ADC channel configuration
const CONFIG: AdcChannelConfig = AdcChannelConfig {
    attenuation: ATTEN,              /* refer to the attenuation value above  */
    calibration: Calibration::Curve, /* ADC auto-calibration type */
    resolution: Resolution::Resolution12Bit, /* ADC resolution */
};
/// Critical voltage value that's still higher than the minimum supply voltage for the ESP32
pub const CRITICAL_VOLTAGE: f32 = 3.22;

/// Battery voltage measurement driver.
pub struct Battery<Gpio: ADCPin> {
    /// ADC driver handle
    adc: BatteryAdcDriver<Gpio>,

    /// ADC channel driver handle
    ch: BatteryAdcChannelDriver<Gpio>,
}

impl<Gpio: ADCPin> Battery<Gpio> {
    /// Initiliaze a new instance of this driver using the given peripheral handles.
    pub fn new(adc: Gpio::Adc, gpio: Gpio) -> OsResult<Self>
    where
        Gpio::Adc: Peripheral<P = Gpio::Adc>,
    {
        let adc_driver = Rc::new(re_esp!(AdcDriver::new(adc), AdcInit)?);
        let ch = re_esp!(
            AdcChannelDriver::<Gpio, Rc<AdcDriver<'static, Gpio::Adc>>>::new(
                Rc::clone(&adc_driver),
                gpio,
                &CONFIG,
            ),
            AdcInit
        )?;

        Ok(Self {
            adc: adc_driver,
            ch,
        })
    }

    /// Read the ADC value and calculate the voltage.
    pub fn read(&mut self, samples: u8) -> OsResult<f32> {
        let raw = self.read_raw_avg(samples)?;
        let volts = f32::from(self.raw_to_mv(raw)?) / 1000.;
        let result = (volts * (R1 + R2)) / R2;

        Ok(result)
    }

    /// Read the raw ADC value.
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    fn read_raw_avg(&mut self, samples: u8) -> OsResult<u16> {
        let mut avg = 0.;

        for _ in 0..samples {
            avg += f32::from(self.read_raw()?);
        }

        avg /= f32::from(samples);
        avg = avg.clamp(0., 4095.);
        Ok(avg as u16)
    }

    fn raw_to_mv(&self, raw: u16) -> OsResult<u16> {
        re_esp!(self.adc.raw_to_mv(&self.ch, raw), AdcRead)
    }

    fn read_raw(&mut self) -> OsResult<u16> {
        re_esp!(self.adc.read_raw(&mut self.ch), AdcRead)
    }
}
