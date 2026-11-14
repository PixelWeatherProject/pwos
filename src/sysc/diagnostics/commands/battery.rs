use crate::sysc::{battery::Battery, usbctl};

pub fn main(battery: &mut Battery) {
    if usbctl::is_connected() {
        println!("Warning: External USB power present!");
    }

    match battery.read() {
        Ok(mv) => {
            println!("Battery voltage: {mv}mV");
        }
        Err(why) => {
            eprintln!("Failed to read battery voltage: {why:?}");
        }
    }
}
