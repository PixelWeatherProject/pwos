use colored::Color;
use log::{Level, LevelFilter, Log};
use std::io::{self, Write};

const BLACKLISTED_MODULES: [&str; 1] = ["esp_idf_svc"];

pub struct OsLogger {
    level: LevelFilter,
}

impl OsLogger {
    pub const fn new() -> Self {
        Self {
            level: if cfg!(debug_assertions) {
                LevelFilter::Debug
            } else {
                LevelFilter::Error
            },
        }
    }

    pub fn init(self) {
        log::set_max_level(self.level);
        log::set_boxed_logger(Box::new(self)).expect("Failed to set logger");
    }

    pub fn disable(&mut self) {
        self.level = LevelFilter::Off;
    }

    fn check_blacklist(&self, module: Option<&str>) -> bool {
        let Some(module) = module else { return true };

        BLACKLISTED_MODULES
            .iter()
            .any(|candidate| module.starts_with(candidate))
    }
}

impl Log for OsLogger {
    fn enabled(&self, _: &log::Metadata) -> bool {
        false
    }

    fn flush(&self) {
        io::stdout().flush().expect("Failed to flush STDOUT");
    }

    #[allow(clippy::unused_io_amount)]
    fn log(&self, record: &log::Record) {
        if self.check_blacklist(record.module_path_static()) {
            return;
        }

        /*
         * This code should prevent heap allocations as much as possible.
         */

        let mut stdout_handle = io::stdout();

        send_color_code(level_to_color(&record.level()), &mut stdout_handle)
            .and_then(|_| stdout_handle.write(record.level().as_str().as_bytes()))
            .and_then(|_| reset_color(&mut stdout_handle))
            .and_then(|_| stdout_handle.write(b" ["))
            .and_then(|_| {
                stdout_handle.write(record.module_path_static().unwrap_or("?").as_bytes())
            })
            .and_then(|_| stdout_handle.write(b"] "))
            .expect("Failed to write to STDOUT");

        // If the message has no arguments to be formatted at runtime, just print it.
        if let Some(text) = record.args().as_str() {
            stdout_handle.write(text.as_bytes())
        } else {
            stdout_handle.write(record.args().to_string().as_bytes())
        }
        .expect("Failed to write to STDOUT");

        stdout_handle
            .write(b"\n")
            .expect("Failed to write to STDOUT");

        // `stdout_handle` does not have a Drop implementation

        self.flush();
    }
}

const fn level_to_color(level: &Level) -> Color {
    match level {
        Level::Info => Color::Blue,
        Level::Warn => Color::Yellow,
        Level::Error => Color::Red,
        Level::Debug => Color::Magenta,
        Level::Trace => Color::Green,
    }
}

fn send_color_code(color: Color, stdout: &mut io::Stdout) -> io::Result<usize> {
    stdout
        .write(b"\x1b[")
        .and_then(|_| stdout.write(color.to_fg_str().as_bytes()))
        .and_then(|_| stdout.write(b"m"))
}

fn reset_color(stdout: &mut io::Stdout) -> io::Result<usize> {
    stdout.write(b"\x1b[0m")
}
