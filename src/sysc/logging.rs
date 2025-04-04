use esp_idf_svc::sys::const_format::concatcp;
use log::{Level, LevelFilter, Log};
use std::io::{stdout, Write};

const BLACKLISTED_MODULES: [&str; 1] = ["esp_idf_svc"];
const COLOR_INFO: &str = "\x1b[1;94m";
const COLOR_WARN: &str = "\x1b[1;33m";
const COLOR_ERROR: &str = "\x1b[1;91m";
const COLOR_DEBUG: &str = "\x1b[1;95m";
const RESET_COLOR: &str = "\x1b[0m";

// We can pre-define these as they don't change during the entire firmware, and this
// way we can avoid runtime formatting.
const INFO_HEADER: &str = concatcp!(COLOR_INFO, "INFO", RESET_COLOR, "  [");
const WARN_HEADER: &str = concatcp!(COLOR_WARN, "WARN", RESET_COLOR, "  [");
const ERROR_HEADER: &str = concatcp!(COLOR_ERROR, "ERROR", RESET_COLOR, " [");
const DEBUG_HEADER: &str = concatcp!(COLOR_DEBUG, "DEBUG", RESET_COLOR, " [");
const TRACE_HEADER: &str = "TRACE [";

pub struct OsLogger {
    enabled: bool,
}

impl OsLogger {
    pub const fn new() -> Self {
        Self { enabled: true }
    }

    pub fn disable(&mut self) {
        self.enabled = false;
    }

    pub fn init(self) {
        #[cfg(debug_assertions)]
        log::set_max_level(LevelFilter::Debug);

        #[cfg(not(debug_assertions))]
        log::set_max_level(LevelFilter::Info);

        log::set_boxed_logger(Box::new(self)).expect("Failed to initialize logger");
    }
}

impl Log for OsLogger {
    fn enabled(&self, _: &log::Metadata) -> bool {
        false
    }

    fn flush(&self) {
        stdout().lock().flush().expect("Failed to flush stdout")
    }

    fn log(&self, record: &log::Record) {
        if !self.enabled {
            return;
        }

        let module = record.module_path_static().unwrap_or("?");

        // Filter out blacklisted modules
        if BLACKLISTED_MODULES
            .iter()
            .any(|candidate| module.starts_with(candidate))
        {
            return;
        }

        // Get a lock to stdout
        let mut lock = stdout().lock();

        // Print the level first
        match record.level() {
            Level::Info => lock.write_all(INFO_HEADER.as_bytes()),
            Level::Warn => lock.write_all(WARN_HEADER.as_bytes()),
            Level::Error => lock.write_all(ERROR_HEADER.as_bytes()),
            Level::Debug => lock.write_all(DEBUG_HEADER.as_bytes()),
            Level::Trace => lock.write_all(TRACE_HEADER.as_bytes()),
        }
        .expect("stdout-write failed");

        // Print the module level next
        lock.write_all(module.as_bytes())
            .and_then(|()| lock.write_all(b"] "))
            .expect("stdout-write-2 failed");

        // Print the actual message, but also avoid runtime formatting when possible
        match record.args().as_str() {
            Some(stat_str) => lock.write_all(stat_str.as_bytes()),
            None => lock.write_all(record.args().to_string().as_bytes()),
        }
        .and_then(|()| lock.write_all(b"\n"))
        .expect("stdout-write-3 failed");
    }
}

#[macro_export]
macro_rules! os_info {
    ($($arg:tt)+) => {
        log::info!($($arg)+)
    };
}

#[macro_export]
macro_rules! os_warn {
    ($($arg:tt)+) => {
        log::warn!($($arg)+)
    };
}

#[macro_export]
macro_rules! os_error {
    ($($arg:tt)+) => {
        log::error!($($arg)+)
    };
}

#[macro_export]
macro_rules! os_debug {
    ($($arg:tt)+) => {
        #[cfg(debug_assertions)]
        log::debug!($($arg)+)
    };
}
