use super::periphctl::PinManager;
use crate::os_warn;
use esp_idf_svc::hal::gpio::{AnyIOPin, IOPin};

macro_rules! create_mapping {
    ($n:expr, $peripherals:ident, [$($num:literal => $gpio:ident),* $(,)?]) => {
        match $n {
            $(
                $num => $peripherals.$gpio().map(|pin| pin.downgrade()).ok(),
            )*
            _ => {
                os_warn!("Requested handle for invalid GPIO pin #{}", $n);
                None
            }
        }
    };
}

pub fn number_to_io_pin(n: u8, peripherals: &mut PinManager) -> Option<AnyIOPin> {
    create_mapping! {
        n,
        peripherals,
        [
            0 => take_gpio0,
            1 => take_gpio1,
            2 => take_gpio2,
            3 => take_gpio3,
            4 => take_gpio4,
            5 => take_gpio5,
            6 => take_gpio6,
            7 => take_gpio7,
            8 => take_gpio8,
            9 => take_gpio9,
            10 => take_gpio10,
            11 => take_gpio11,
            12 => take_gpio12,
            13 => take_gpio13,
            14 => take_gpio14,
            15 => take_gpio15,
            16 => take_gpio16,
            17 => take_gpio17,
            18 => take_gpio18,
            19 => take_gpio19,
            20 => take_gpio20,
            21 => take_gpio21,
            26 => take_gpio26,
            27 => take_gpio27,
            28 => take_gpio28,
            29 => take_gpio29,
            30 => take_gpio30,
            31 => take_gpio31,
            32 => take_gpio32,
            33 => take_gpio33,
            34 => take_gpio34,
            35 => take_gpio35,
            36 => take_gpio36,
            37 => take_gpio37,
            38 => take_gpio38,
            39 => take_gpio39,
            40 => take_gpio40,
            41 => take_gpio41,
            42 => take_gpio42,
            43 => take_gpio43,
            44 => take_gpio44,
            45 => take_gpio45,
            46 => take_gpio46,
            47 => take_gpio47,
            48 => take_gpio48,
        ]
    }
}
