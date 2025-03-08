use super::{OsError, OsResult};
use esp_idf_svc::hal::{
    adc::ADC1, gpio::*, i2c::I2C1, modem::Modem, prelude::Peripherals, uart::UART1,
};

macro_rules! impl_take {
    ($fn_name:ident, $field:ident, $return_type:ty) => {
        pub fn $fn_name(&mut self) -> OsResult<$return_type> {
            self.$field.take().ok_or(OsError::PeriphUnavailable)
        }
    };
}

pub struct PeripheralManager {
    // Add more as necessary
    pub pins: PinManager,
    uart1: Option<UART1>,
    i2c1: Option<I2C1>,
    adc1: Option<ADC1>,
    modem: Option<Modem>,
}

pub struct PinManager {
    pub gpio0: Option<Gpio0>,
    pub gpio1: Option<Gpio1>,
    pub gpio2: Option<Gpio2>,
    pub gpio3: Option<Gpio3>,
    pub gpio4: Option<Gpio4>,
    pub gpio5: Option<Gpio5>,
    pub gpio6: Option<Gpio6>,
    pub gpio7: Option<Gpio7>,
    pub gpio8: Option<Gpio8>,
    pub gpio9: Option<Gpio9>,
    pub gpio10: Option<Gpio10>,
    pub gpio11: Option<Gpio11>,
    pub gpio12: Option<Gpio12>,
    pub gpio13: Option<Gpio13>,
    pub gpio14: Option<Gpio14>,
    pub gpio15: Option<Gpio15>,
    pub gpio16: Option<Gpio16>,
    pub gpio17: Option<Gpio17>,
    pub gpio18: Option<Gpio18>,
    pub gpio19: Option<Gpio19>,
    pub gpio20: Option<Gpio20>,
    pub gpio21: Option<Gpio21>,
    pub gpio26: Option<Gpio26>,
    pub gpio27: Option<Gpio27>,
    pub gpio28: Option<Gpio28>,
    pub gpio29: Option<Gpio29>,
    pub gpio30: Option<Gpio30>,
    pub gpio31: Option<Gpio31>,
    pub gpio32: Option<Gpio32>,
    pub gpio33: Option<Gpio33>,
    pub gpio34: Option<Gpio34>,
    pub gpio35: Option<Gpio35>,
    pub gpio36: Option<Gpio36>,
    pub gpio37: Option<Gpio37>,
    pub gpio38: Option<Gpio38>,
    pub gpio39: Option<Gpio39>,
    pub gpio40: Option<Gpio40>,
    pub gpio41: Option<Gpio41>,
    pub gpio42: Option<Gpio42>,
    pub gpio43: Option<Gpio43>,
    pub gpio44: Option<Gpio44>,
    pub gpio45: Option<Gpio45>,
    pub gpio46: Option<Gpio46>,
    pub gpio47: Option<Gpio47>,
    pub gpio48: Option<Gpio48>,
}

impl PeripheralManager {
    impl_take!(take_uart1, uart1, UART1);
    impl_take!(take_i2c1, i2c1, I2C1);
    impl_take!(take_adc1, adc1, ADC1);
    impl_take!(take_modem, modem, Modem);
}

impl PinManager {
    impl_take!(take_gpio0, gpio0, Gpio0);
    impl_take!(take_gpio1, gpio1, Gpio1);
    impl_take!(take_gpio2, gpio2, Gpio2);
    impl_take!(take_gpio3, gpio3, Gpio3);
    impl_take!(take_gpio4, gpio4, Gpio4);
    impl_take!(take_gpio5, gpio5, Gpio5);
    impl_take!(take_gpio6, gpio6, Gpio6);
    impl_take!(take_gpio7, gpio7, Gpio7);
    impl_take!(take_gpio8, gpio8, Gpio8);
    impl_take!(take_gpio9, gpio9, Gpio9);
    impl_take!(take_gpio10, gpio10, Gpio10);
    impl_take!(take_gpio11, gpio11, Gpio11);
    impl_take!(take_gpio12, gpio12, Gpio12);
    impl_take!(take_gpio13, gpio13, Gpio13);
    impl_take!(take_gpio14, gpio14, Gpio14);
    impl_take!(take_gpio15, gpio15, Gpio15);
    impl_take!(take_gpio16, gpio16, Gpio16);
    impl_take!(take_gpio17, gpio17, Gpio17);
    impl_take!(take_gpio18, gpio18, Gpio18);
    impl_take!(take_gpio19, gpio19, Gpio19);
    impl_take!(take_gpio20, gpio20, Gpio20);
    impl_take!(take_gpio21, gpio21, Gpio21);
    impl_take!(take_gpio26, gpio26, Gpio26);
    impl_take!(take_gpio27, gpio27, Gpio27);
    impl_take!(take_gpio28, gpio28, Gpio28);
    impl_take!(take_gpio29, gpio29, Gpio29);
    impl_take!(take_gpio30, gpio30, Gpio30);
    impl_take!(take_gpio31, gpio31, Gpio31);
    impl_take!(take_gpio32, gpio32, Gpio32);
    impl_take!(take_gpio33, gpio33, Gpio33);
    impl_take!(take_gpio34, gpio34, Gpio34);
    impl_take!(take_gpio35, gpio35, Gpio35);
    impl_take!(take_gpio36, gpio36, Gpio36);
    impl_take!(take_gpio37, gpio37, Gpio37);
    impl_take!(take_gpio38, gpio38, Gpio38);
    impl_take!(take_gpio39, gpio39, Gpio39);
    impl_take!(take_gpio40, gpio40, Gpio40);
    impl_take!(take_gpio41, gpio41, Gpio41);
    impl_take!(take_gpio42, gpio42, Gpio42);
    impl_take!(take_gpio43, gpio43, Gpio43);
    impl_take!(take_gpio44, gpio44, Gpio44);
    impl_take!(take_gpio45, gpio45, Gpio45);
    impl_take!(take_gpio46, gpio46, Gpio46);
    impl_take!(take_gpio47, gpio47, Gpio47);
    impl_take!(take_gpio48, gpio48, Gpio48);
}

impl From<Pins> for PinManager {
    fn from(value: Pins) -> Self {
        Self {
            gpio0: Some(value.gpio0),
            gpio1: Some(value.gpio1),
            gpio2: Some(value.gpio2),
            gpio3: Some(value.gpio3),
            gpio4: Some(value.gpio4),
            gpio5: Some(value.gpio5),
            gpio6: Some(value.gpio6),
            gpio7: Some(value.gpio7),
            gpio8: Some(value.gpio8),
            gpio9: Some(value.gpio9),
            gpio10: Some(value.gpio10),
            gpio11: Some(value.gpio11),
            gpio12: Some(value.gpio12),
            gpio13: Some(value.gpio13),
            gpio14: Some(value.gpio14),
            gpio15: Some(value.gpio15),
            gpio16: Some(value.gpio16),
            gpio17: Some(value.gpio17),
            gpio18: Some(value.gpio18),
            gpio19: Some(value.gpio19),
            gpio20: Some(value.gpio20),
            gpio21: Some(value.gpio21),
            gpio26: Some(value.gpio26),
            gpio27: Some(value.gpio27),
            gpio28: Some(value.gpio28),
            gpio29: Some(value.gpio29),
            gpio30: Some(value.gpio30),
            gpio31: Some(value.gpio31),
            gpio32: Some(value.gpio32),
            gpio33: Some(value.gpio33),
            gpio34: Some(value.gpio34),
            gpio35: Some(value.gpio35),
            gpio36: Some(value.gpio36),
            gpio37: Some(value.gpio37),
            gpio38: Some(value.gpio38),
            gpio39: Some(value.gpio39),
            gpio40: Some(value.gpio40),
            gpio41: Some(value.gpio41),
            gpio42: Some(value.gpio42),
            gpio43: Some(value.gpio43),
            gpio44: Some(value.gpio44),
            gpio45: Some(value.gpio45),
            gpio46: Some(value.gpio46),
            gpio47: Some(value.gpio47),
            gpio48: Some(value.gpio48),
        }
    }
}

impl From<Peripherals> for PeripheralManager {
    fn from(value: Peripherals) -> Self {
        Self {
            pins: value.pins.into(),
            uart1: Some(value.uart1),
            i2c1: Some(value.i2c1),
            adc1: Some(value.adc1),
            modem: Some(value.modem),
        }
    }
}
