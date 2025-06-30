//! System peripherals.

use crate::os_debug;
use esp_idf_svc::{
    eventloop::EspSystemEventLoop,
    hal::{modem::Modem, prelude::Peripherals},
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

fn initialize_base_parts() -> (Peripherals, EspSystemEventLoop) {
    os_debug!("Initializing base peripherals");
    let peripherals = Peripherals::take().expect("Failed to initialize peripherals");
    os_debug!("Initializing System Event Loop");
    let sys_loop = EspSystemEventLoop::take().expect("SEL init error");

    (peripherals, sys_loop /* add more when needed */)
}

#[cfg(any(
    feature = "lilygo-t7s3",
    /* default to this implementation if no board-specific configuration is selected */
    not(any(feature = "lilygo-t7s3", feature = "xiao-s3", feature = "arduino-nano-esp32"))
))]
mod lilygo_t7s3;

#[cfg(feature = "xiao-s3")]
mod xiao_s3;

#[cfg(feature = "arduino-nano-esp32")]
mod arduino_nano_esp32;
