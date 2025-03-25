//! GPIO-related operations and utilities.

use crate::os_warn;
use esp_idf_svc::hal::{
    gpio::{AnyIOPin, IOPin},
    peripheral::Peripheral,
    prelude::Peripherals,
};

/// Map an integer value to the specified struct property.
macro_rules! create_mapping {
    ($n:expr, $peripherals:ident, [$($num:literal => $gpio:ident),* $(,)?]) => {
        match $n {
            $(
                $num => Some(unsafe { $peripherals.pins.$gpio.clone_unchecked().downgrade() }),
            )*
            _ => {
                os_warn!("Requested handle for invalid GPIO pin #{}", $n);
                None
            }
        }
    };
}

/// Return a handle to the specified GPIO pin, identified by it's numeric name.
///
/// If the specified GPIO pin number is invalid, [`None`] is returned.
pub fn number_to_io_pin(n: u8, peripherals: &mut Peripherals) -> Option<AnyIOPin> {
    create_mapping! {
        n,
        peripherals,
        [
            0 => gpio0,
            1 => gpio1,
            2 => gpio2,
            3 => gpio3,
            4 => gpio4,
            5 => gpio5,
            6 => gpio6,
            7 => gpio7,
            8 => gpio8,
            9 => gpio9,
            10 => gpio10,
            11 => gpio11,
            12 => gpio12,
            13 => gpio13,
            14 => gpio14,
            15 => gpio15,
            16 => gpio16,
            17 => gpio17,
            18 => gpio18,
            19 => gpio19,
            20 => gpio20,
            21 => gpio21,
            26 => gpio26,
            27 => gpio27,
            28 => gpio28,
            29 => gpio29,
            30 => gpio30,
            31 => gpio31,
            32 => gpio32,
            33 => gpio33,
            34 => gpio34,
            35 => gpio35,
            36 => gpio36,
            37 => gpio37,
            38 => gpio38,
            39 => gpio39,
            40 => gpio40,
            41 => gpio41,
            42 => gpio42,
            43 => gpio43,
            44 => gpio44,
            45 => gpio45,
            46 => gpio46,
            47 => gpio47,
            48 => gpio48,
        ]
    }
}
