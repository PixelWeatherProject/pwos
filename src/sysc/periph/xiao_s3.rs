use super::{
    initialize_base_parts, BatteryPeripherals, I2cPeripherals, OnboardLedPeripherals,
    SystemPeripherals, WifiPeripherals,
};
use esp_idf_svc::hal::{
    adc::ADC1,
    gpio::{Gpio21, Gpio3, Gpio5, Gpio8},
    i2c::I2C1,
};

impl SystemPeripherals<I2C1, Gpio8, Gpio5, ADC1, Gpio3, Gpio21> {
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
                pin: peripherals.pins.gpio3,
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
