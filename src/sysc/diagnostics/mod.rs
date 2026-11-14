use crate::sysc::{battery::Battery, jtag_serial::JtagSerialDriver, nvs::NonVolatileStorage};

mod commands;

pub fn diagnostics_main(mut battery: Battery, nvs: &NonVolatileStorage) {
    println!("\nPixelWeatherOS Diagnostics Console\n");

    let jtag_serial_driver = match JtagSerialDriver::new() {
        Ok(d) => d,
        Err(why) => {
            log::error!("Failed to set up console input: {why}");
            return;
        }
    };

    let mut input_buffer = String::with_capacity(64);

    loop {
        read_command(&jtag_serial_driver, &mut input_buffer);

        match input_buffer.as_str() {
            "" => (),
            "battery" => {
                commands::battery::main(&mut battery);
            }
            "nvsdump" => {
                commands::nvsdump::main(nvs);
            }
            "help" => {
                println!("Diagnostics commands:\n");
                println!("- battery:      Reads the battery voltage level");
                println!("- nvsdump:      Dumps the OS variables stored in NVS");
                println!("- nvsreset:     Deletes all stored OS variables from NVS");
                println!("- settingsdump: Dumps all node settings");
                println!("- otadump:      Dumps OTA status");
                println!("- rtcdump:      Dumps RTC variables");
                println!("- ddlsdump:     Dumps the current status of the DDL subsystem");
                println!("- logsdump:     Dumps the system logs from the previous run");
                println!("- reboot:       Restart the system");
                println!();
            }
            "exit" => {
                drop(jtag_serial_driver);
                return;
            }
            _ => eprintln!("unknown command: {input_buffer}"),
        }

        input_buffer.clear();
    }
}

fn read_command(driver: &JtagSerialDriver, buffer: &mut String) {
    driver.print("$ ");

    loop {
        let byte = driver.read_byte();

        match byte {
            b'\r' | b'\n' => {
                driver.write_bytes(b"\r\n");
                break;
            }
            // Backspace / DEL
            0x08 | 0x7f => {
                if buffer.pop().is_some() {
                    driver.write_bytes(b"\x08 \x08");
                }
            }
            0x20..=0x7e => {
                buffer.push(byte as char);
                driver.write_bytes(&[byte]);
            }
            _ => (), // ignore other control chars / escape sequences
        }
    }
}
