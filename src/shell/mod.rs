use crate::{os_debug, os_error, sysc::OsResult};
use esp_idf_svc::sys::{esp, esp_vfs_dev_uart_use_driver, uart_driver_install, vTaskDelay};
use std::{
    io::{stdin, stdout, Write},
    ptr,
};

pub fn shell_start() -> ! {
    os_debug!("Setting up shell");
    setup_stdin().unwrap();

    println!("\nWelcome PWOS-Shell!");
    let mut buf = String::new();
    let mut read = false;

    loop {
        if read {
            display_prompt();
        }

        match stdin().read_line(&mut buf) {
            Ok(_) => (),
            Err(why) => match why.kind() {
                std::io::ErrorKind::WouldBlock
                | std::io::ErrorKind::TimedOut
                | std::io::ErrorKind::Interrupted => {
                    unsafe { vTaskDelay(10) };
                    continue;
                }
                _ => {
                    os_error!("Error: {why}\r\n");
                    continue;
                }
            },
        }

        println!("You said: {buf}");
        buf.clear();
        read = false;
    }
}

fn display_prompt() {
    print!("> ");
    stdout().flush().unwrap();
}

fn setup_stdin() -> OsResult<()> {
    unsafe {
        esp!(uart_driver_install(0, 512, 512, 10, ptr::null_mut(), 0))?;
        esp_vfs_dev_uart_use_driver(0);
    };

    Ok(())
}
