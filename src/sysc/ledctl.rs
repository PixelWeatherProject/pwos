use super::OsResult;
use esp_idf_svc::hal::gpio::{AnyIOPin, Output, PinDriver};

type LedDriver = PinDriver<'static, AnyIOPin, Output>;

pub struct BoardLed(LedDriver, bool);

impl BoardLed {
    pub fn new(pin: AnyIOPin, invert: bool) -> OsResult<Self> {
        let mut i = Self(PinDriver::output(pin)?, invert);
        i.on();

        Ok(i)
    }

    // On/Off operations are usually not failable, but errors are not fatal either.
    // They can be safely ignored. This also reduces the size of the firmware.

    pub fn on(&mut self) {
        let _ = self.0.set_level((!self.1).into());
    }

    pub fn off(&mut self) {
        let _ = self.0.set_level(self.1.into());
    }
}
