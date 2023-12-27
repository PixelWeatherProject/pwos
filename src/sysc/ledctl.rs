use esp_idf_svc::hal::gpio::{Gpio19, Output, PinDriver};

type LedGpio = Gpio19;
type LedDriver = PinDriver<'static, LedGpio, Output>;

pub struct BoardLed(LedDriver);

impl BoardLed {
    pub fn new(pin: LedGpio) -> Self {
        let mut i = Self(unsafe { PinDriver::output(pin).unwrap_unchecked() });
        i.on();

        i
    }

    pub fn on(&mut self) {
        unsafe { self.0.set_high().unwrap_unchecked() }
    }

    pub fn off(&mut self) {
        unsafe { self.0.set_low().unwrap_unchecked() }
    }
}
