use super::OsResult;
use crate::re_esp;
use esp_idf_svc::hal::gpio::{AnyOutputPin, Output, PinDriver};

type LedDriver = PinDriver<'static, Output>;

/// A simple LED driver for the onboard LED.
pub struct BoardLed(LedDriver, bool);

impl BoardLed {
    /// Initialize the driver using the specified GPIO pin and invert mode.
    ///
    /// If the LED is driven using inverted logic (`true` - off, `false` - on), `invert`
    /// should be set to `true`.
    ///
    /// # Errors
    /// Returns an error if [`PinDriver::output`] fails.
    pub fn new(pin: AnyOutputPin<'static>, invert: bool) -> OsResult<Self> {
        let mut i = Self(re_esp!(PinDriver::output(pin), GpioInit)?, invert);
        i.on();

        Ok(i)
    }

    // On/Off operations are usually not failable, but errors are not fatal either.
    // They can be safely ignored. This also reduces the size of the firmware.

    /// Turns on the onboard LED.
    pub fn on(&mut self) {
        let _ = self.0.set_level((!self.1).into());
    }

    /// Turns off the onboard LED.
    pub fn off(&mut self) {
        let _ = self.0.set_level(self.1.into());
    }
}
