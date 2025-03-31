//! System peripherals.

#![allow(clippy::wildcard_imports)]

use crate::os_debug;
use esp_idf_svc::{
    eventloop::EspSystemEventLoop,
    hal::{adc::*, gpio::*, i2c::*, modem::Modem, prelude::Peripherals},
};

pub struct SystemPeripherals<I2C, SclPin, SdaPin, ADC, ADCPin, LedPin> {
    pub i2c: I2cPeripherals<I2C, SclPin, SdaPin>,
    pub battery: BatteryPeripherals<ADC, ADCPin>,
    pub onboard_led: OnboardLedPeripherals<LedPin>,
    pub wifi: WifiPeripherals,
}

pub struct I2cPeripherals<I2C, SclPin, SdaPin> {
    pub i2c: I2C,
    pub scl: SclPin,
    pub sda: SdaPin,
}

pub struct BatteryPeripherals<ADC, Pin> {
    pub adc: ADC,
    pub pin: Pin,
}

pub struct OnboardLedPeripherals<LedPin> {
    pub pin: LedPin,
    pub invert: bool,
}

pub struct WifiPeripherals {
    pub modem: Modem,
    pub sys_loop: EspSystemEventLoop,
}

#[cfg(any(
    feature = "lilygo-t7s3",
    /* default to this implementation if no board-specific configuration is selected */
    not(any(feature = "lilygo-t7s3", feature = "xiao-s3", feature = "arduino-nano-esp32"))
))]
impl SystemPeripherals<I2C1, Gpio8, Gpio5, ADC1, Gpio2, Gpio17> {
    pub fn take() -> Self {
        let (peripherals, sys_loop) = initialize_base_parts();

        Self {
            i2c: I2cPeripherals {
                i2c: peripherals.i2c1,
                scl: peripherals.pins.gpio8,
                sda: peripherals.pins.gpio5,
            },
            battery: BatteryPeripherals {
                adc: peripherals.adc1,
                pin: peripherals.pins.gpio2,
            },
            onboard_led: OnboardLedPeripherals {
                pin: peripherals.pins.gpio17,
                invert: false,
            },
            wifi: WifiPeripherals {
                modem: peripherals.modem,
                sys_loop,
            },
        }
    }
}

#[cfg(feature = "xiao-s3")]
impl SystemPeripherals<I2C1, Gpio8, Gpio5, ADC1, Gpio2, Gpio21> {
    pub fn take() -> Self {
        let (peripherals, sys_loop) = initialize_base_parts();

        Self {
            i2c: I2cPeripherals {
                i2c: peripherals.i2c1,
                scl: peripherals.pins.gpio8,
                sda: peripherals.pins.gpio5,
            },
            battery: BatteryPeripherals {
                adc: peripherals.adc1,
                pin: peripherals.pins.gpio2,
            },
            onboard_led: OnboardLedPeripherals {
                pin: peripherals.pins.gpio21,
                invert: true,
            },
            wifi: WifiPeripherals {
                modem: peripherals.modem,
                sys_loop,
            },
        }
    }
}

#[cfg(feature = "arduino-nano-esp32")]
impl SystemPeripherals<I2C1, Gpio8, Gpio5, ADC1, Gpio2, Gpio48> {
    pub fn take() -> Self {
        let (peripherals, sys_loop) = initialize_base_parts();

        Self {
            i2c: I2cPeripherals {
                i2c: peripherals.i2c1,
                scl: peripherals.pins.gpio8,
                sda: peripherals.pins.gpio5,
            },
            battery: BatteryPeripherals {
                adc: peripherals.adc1,
                pin: peripherals.pins.gpio2,
            },
            onboard_led: OnboardLedPeripherals {
                pin: peripherals.pins.gpio48,
                invert: false,
            },
            wifi: WifiPeripherals {
                modem: peripherals.modem,
                sys_loop,
            },
        }
    }
}

fn initialize_base_parts() -> (Peripherals, EspSystemEventLoop) {
    os_debug!("Initializing base peripherals");
    let peripherals = Peripherals::take().expect("Failed to initialize peripherals");
    os_debug!("Initializing System Event Loop");
    let sys_loop = EspSystemEventLoop::take().expect("SEL init error");

    (peripherals, sys_loop /* add more when needed */)
}
