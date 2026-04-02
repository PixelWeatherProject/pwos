use esp_idf_svc::sys::const_format::concatcp;
use log::{Level, LevelFilter, Log};
use std::{
    io::{stdout, Write},
    sync::atomic::{AtomicBool, Ordering},
};

/// Modules whose logs should be ignored.
const BLACKLISTED_MODULES: [&str; 1] = ["esp_idf_svc"];
/// Color code for an info message.
const COLOR_INFO: &str = "\x1b[1;94m";
/// Color code for a warning message.
const COLOR_WARN: &str = "\x1b[1;33m";
/// Color code for an error message.
const COLOR_ERROR: &str = "\x1b[1;91m";
/// Color code for a debug message.
const COLOR_DEBUG: &str = "\x1b[1;95m";
/// Code for resetting a previously set color.
const RESET_COLOR: &str = "\x1b[0m";

// We can pre-define these as they don't change during the entire firmware, and this
// way we can avoid runtime formatting.
const INFO_HEADER: &str = concatcp!(COLOR_INFO, "INFO", RESET_COLOR, "  [");
const WARN_HEADER: &str = concatcp!(COLOR_WARN, "WARN", RESET_COLOR, "  [");
const ERROR_HEADER: &str = concatcp!(COLOR_ERROR, "ERROR", RESET_COLOR, " [");
const DEBUG_HEADER: &str = concatcp!(COLOR_DEBUG, "DEBUG", RESET_COLOR, " [");
const TRACE_HEADER: &str = "TRACE [";

/// The global instance of the logger.
pub static LOGGER: OsLogger = OsLogger::new();

/// The firmware-wide logging backend.
///
/// It integrates with the [`log`] crate.
pub struct OsLogger {
    /// Whether the logger is enabled.
    enabled: AtomicBool,
}

impl OsLogger {
    /// Create the logger.
    pub const fn new() -> Self {
        Self {
            enabled: AtomicBool::new(true),
        }
    }

    /// Disable the global logger.
    ///
    /// When disabled, it will not print any messages *regardless of their level*.
    pub fn disable() {
        LOGGER.enabled.store(false, Ordering::SeqCst);
    }

    /// Initialize the global logger.
    ///
    /// Initialization consists of two steps:
    /// - Setting maximum log level.
    ///     - [`LevelFilter::Debug`] for debug builds, [`LevelFilter::Info`] for release builds
    /// - Setting the global logger by calling [`log::set_boxed_logger`].
    ///
    /// # Panics
    /// This will panic if [`log::set_boxed_logger`] returns an error. This should never happen if
    /// this method was never called before.
    pub fn init() {
        #[cfg(debug_assertions)]
        log::set_max_level(LevelFilter::Debug);

        #[cfg(not(debug_assertions))]
        log::set_max_level(LevelFilter::Info);

        log::set_logger(&LOGGER).expect("Failed to initialize logger");
    }
}

impl Log for OsLogger {
    fn enabled(&self, _: &log::Metadata) -> bool {
        false
    }

    fn flush(&self) {
        stdout().lock().flush().expect("Failed to flush stdout");
    }

    fn log(&self, record: &log::Record) {
        if !self.enabled.load(Ordering::Relaxed) {
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
