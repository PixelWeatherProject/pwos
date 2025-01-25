use crate::config::LED_BUILTIN_INVERT;
use esp_idf_svc::hal::gpio::{AnyIOPin, Output, PinDriver};

type LedDriver = PinDriver<'static, AnyIOPin, Output>;

pub struct BoardLed(LedDriver);

impl BoardLed {
    pub fn new(pin: AnyIOPin) -> Self {
        let mut i = Self(unsafe { PinDriver::output(pin).unwrap_unchecked() });
        i.on();

        i
    }

    // On/Off operations are usually not failable, but errors are not fatal either.
    // They can be safely ignored. This also reduces the size of the firmware.

    pub fn on(&mut self) {
        let _ = if LED_BUILTIN_INVERT {
            self.0.set_low()
        } else {
            self.0.set_high()
        };
    }

    pub fn off(&mut self) {
        let _ = if LED_BUILTIN_INVERT {
            self.0.set_high()
        } else {
            self.0.set_low()
        };
    }
}
