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

type BatteryGpio = Gpio2;
type BatteryAdc = ADC1;
type BatteryAdcDriver = AdcDriver<'static, BatteryAdc>;
type BatteryAdcChannelDriver = AdcChannelDriver<'static, BatteryGpio, Rc<BatteryAdcDriver>>;

const ATTEN: adc_atten_t = DB_11;
const R1: Decimal = dec!(20_000); // 20kOhm
const R2: Decimal = dec!(6_800); // 6.8kOhm
const CONFIG: AdcChannelConfig = AdcChannelConfig {
    attenuation: ATTEN,
    calibration: Calibration::Curve,
    resolution: Resolution::Resolution12Bit,
};
const MAX_VOLTAGE: Decimal = dec!(5.00);

pub const CRITICAL_VOLTAGE: Decimal = dec!(2.70);

pub struct Battery {
    adc: Rc<BatteryAdcDriver>,
    ch: BatteryAdcChannelDriver,
}

impl Battery {
    pub fn new(adc: BatteryAdc, gpio: BatteryGpio) -> OsResult<Self> {
        let adc = Rc::new(BatteryAdcDriver::new(adc)?);
        let ch = BatteryAdcChannelDriver::new(Rc::clone(&adc), gpio, &CONFIG)?;

        Ok(Self { adc, ch })
    }

    pub fn read(&mut self, samples: u8) -> OsResult<Decimal> {
        let raw = self.read_raw(samples)?;
        let volts = Decimal::from(self.adc.raw_to_mv(&self.ch, raw)?) / dec!(1000);
        let mut result = (volts * (R1 + R2)) / (R2);

        if result > MAX_VOLTAGE {
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
