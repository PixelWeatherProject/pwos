use super::OsResult;
use esp_idf_svc::{
    hal::{
        adc::{
            attenuation::DB_2_5,
            oneshot::{config::AdcChannelConfig, AdcChannelDriver, AdcDriver},
            Resolution, ADC1,
        },
        gpio::Gpio2,
    },
    sys::adc_atten_t,
};
use pwmp_client::pwmp_msg::{aliases::BatteryVoltage, dec, Decimal};
use std::{thread::sleep, time::Duration};

const ATTEN: adc_atten_t = DB_2_5;
const VMAX: f32 = 1250.0; // For attenuation 2.5dB
const DIVIDER_R1: f32 = 20_000.0; // 20kOhm
const DIVIDER_R2: f32 = 6800.0; // 6.8kOhm
type BatteryGpio = Gpio2;
type BatteryAdc = ADC1;
type BatteryDriver = AdcDriver<'static, BatteryAdc>;
type BatteryChDriver = AdcChannelDriver<'static, BatteryGpio, BatteryDriver>;

pub const CRITICAL_VOLTAGE: Decimal = dec!(2.70);
pub const ADC_CONFIG: AdcChannelConfig = AdcChannelConfig {
    attenuation: ATTEN,
    calibration: true,
    resolution: Resolution::Resolution12Bit,
};

pub struct Battery(BatteryChDriver);

impl Battery {
    pub fn new(adc: BatteryAdc, gpio: BatteryGpio) -> OsResult<Self> {
        let driver = BatteryDriver::new(adc)?;
        let ch_driver = BatteryChDriver::new(driver, gpio, &ADC_CONFIG)?;

        Ok(Self(ch_driver))
    }

    pub fn read_voltage(&mut self, samples: u16) -> OsResult<BatteryVoltage> {
        let div_out = self.read_raw_voltage(samples)?;
        // Vout = Vin * (R2 / (R1 + R2)) => Vin = Vout * (R1 + R2) / R2
        let vin = div_out * (DIVIDER_R1 + DIVIDER_R2) / DIVIDER_R2;
        let voltage = vin.clamp(0.0, 4.2);

        let mut decimal = Decimal::from_f32_retain(voltage).unwrap();
        decimal.rescale(2);

        Ok(decimal)
    }

    pub fn read_raw_voltage(&mut self, samples: u16) -> OsResult<f32> {
        Ok(Self::raw_to_voltage(self.read_raw(samples)?))
    }

    fn raw_to_voltage(raw: u16) -> f32 {
        (VMAX / 4096.0) * (raw as f32)
    }

    fn read_raw(&mut self, samples: u16) -> OsResult<u16> {
        let mut avg = 0.0f32;

        for _ in 0..samples {
            avg += self.0.read()? as f32;
            sleep(Duration::from_millis(20));
        }

        avg /= samples as f32;
        Ok(avg as u16)
    }
}
