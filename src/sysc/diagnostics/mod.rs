use crate::sysc::{battery::Battery, ledctl::BoardLed, nvs::NonVolatileStorage, ota::Ota};
use esp_idf_svc::{
    eventloop::EspSystemEventLoop,
    hal::{delay::BLOCK, modem::Modem},
    sys::{
        esp, usb_serial_jtag_driver_config_t, usb_serial_jtag_driver_install,
        usb_serial_jtag_driver_uninstall, usb_serial_jtag_read_bytes, usb_serial_jtag_write_bytes,
        EspError,
    },
};
use pwmp_client::pwmp_msg::settings::NodeSettings;
use std::io::{stdout, Write};

pub fn diagnostics_main(
    mut battery: Battery,
    modem: Modem<'static>,
    sys_loop: EspSystemEventLoop,
    mut led: BoardLed,
    nvs: &NonVolatileStorage,
    ota: &Ota,
    cfg: &NodeSettings,
) {
    println!("\nPixelWeatherOS Diagnostics Console\n");

    if let Err(why) = init_console_input() {
        log::error!("Failed to set up console input: {why}");
        return;
    }

    let mut input_buffer = String::with_capacity(64);

    loop {
        read_command(&mut input_buffer);

        match input_buffer.as_str() {
            "" => (),
            "test" => {
                println!("Hello, world!");
            }
            "exit" => {
                unsafe { usb_serial_jtag_driver_uninstall() };
                return;
            }
            _ => eprintln!("unknown command: {input_buffer}"),
        }

        input_buffer.clear();
    }
}

/// Installs the USB-Serial-JTAG driver so input can be read from it.
///
/// `CONFIG_VFS_SUPPORT_IO` is disabled, so `stdin` has no backend and always
/// reports EOF. Input is therefore read directly from the driver instead.
fn init_console_input() -> Result<(), EspError> {
    let mut cfg = usb_serial_jtag_driver_config_t {
        tx_buffer_size: 256,
        rx_buffer_size: 256,
    };

    esp!(unsafe { usb_serial_jtag_driver_install(&raw mut cfg) })
}

/// Writes directly through the USB-Serial-JTAG driver.
///
/// The ROM console path used by `print!` (VFS is disabled) only pushes the TX
/// FIFO to the host on `\n`, so partial lines like the prompt or echoed
/// characters would never appear. The driver flushes after every write.
fn console_write(data: &[u8]) {
    // Make sure anything already buffered in Rust's stdout goes out first.
    let _ = stdout().flush();
    unsafe { usb_serial_jtag_write_bytes(data.as_ptr().cast(), data.len(), BLOCK) };
}

/// Blocks until a full line is entered. Echoes input and handles backspace.
fn read_command(buffer: &mut String) {
    console_write(b"$ ");

    loop {
        let mut byte = 0u8;
        let read = unsafe { usb_serial_jtag_read_bytes((&raw mut byte).cast(), 1, BLOCK) };
        if read <= 0 {
            continue;
        }

        match byte {
            b'\r' | b'\n' => {
                console_write(b"\r\n");
                break;
            }
            // Backspace / DEL
            0x08 | 0x7f => {
                if buffer.pop().is_some() {
                    console_write(b"\x08 \x08");
                }
            }
            0x20..=0x7e => {
                buffer.push(byte as char);
                console_write(&[byte]);
            }
            _ => (), // ignore other control chars / escape sequences
        }
    }
}
