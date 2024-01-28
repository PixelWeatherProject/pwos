use log::{Level, Metadata, Record};
use std::io::{stdout, Write};

#[cfg(not(debug_assertions))]
const MAX_LEVEL: Level = Level::Info;
#[cfg(debug_assertions)]
const MAX_LEVEL: Level = Level::Debug;

pub struct PwosLogger;

impl PwosLogger {
    pub fn init() {
        log::set_logger(&Self)
            .map(|()| log::set_max_level(MAX_LEVEL.to_level_filter()))
            .unwrap();
    }
}

impl log::Log for PwosLogger {
    fn enabled(&self, _: &Metadata) -> bool {
        true
    }

    fn flush(&self) {
        let _ = stdout().flush();
    }

    fn log(&self, record: &Record) {
        let mut stdout = stdout();
        let mut text = String::with_capacity(8);

        let level_name = match record.level() {
            Level::Info => "INFO",
            Level::Warn => "WARN",
            Level::Error => "ERROR",
            Level::Debug => "DEBUG",
            Level::Trace => "TRACE",
        };
        let component = record.module_path().unwrap_or("unknown");

        if !component.starts_with("pwos") {
            return;
        }

        let message = record.args().to_string();

        ufmt::uwriteln!(text, "{} {}: {}", level_name, component, message).unwrap();
        stdout.write_all(text.as_bytes()).unwrap();
    }
}
