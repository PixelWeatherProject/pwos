use esp_idf_svc::hal::gpio::{Gpio17, Output, PinDriver};

type LedGpio = Gpio17;
type LedDriver = PinDriver<'static, LedGpio, Output>;

pub struct BoardLed(LedDriver);

impl BoardLed {
    pub fn new(pin: LedGpio) -> Self {
        let mut i = Self(unsafe { PinDriver::output(pin).unwrap_unchecked() });
        i.on();

        i
    }

    // On/Off operations are usually not failable, but errors are not fatal either.
    // They can be safely ignored. This also reduces the size of the firmware.

    pub fn on(&mut self) {
        let _ = self.0.set_high();
    }

    pub fn off(&mut self) {
        let _ = self.0.set_low();
    }
}
