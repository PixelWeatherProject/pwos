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
static LOGGER: OsLogger = OsLogger::new();

/// The firmware-wide logging backend.
///
/// It integrates with the [`log`] crate.
struct OsLogger {
    /// Whether the logger is enabled.
    enabled: AtomicBool,
}

/// A global logger that can be used to log messages.
pub struct GlobalOsLogger;

impl OsLogger {
    /// Create the logger.
    pub const fn new() -> Self {
        Self {
            enabled: AtomicBool::new(true),
        }
    }
}

impl GlobalOsLogger {
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

        // Create a buffer
        let mut buffer = heapless::String::<256>::new();

        // Get a lock to stdout
        let mut lock = stdout().lock();

        // Get the header
        let header = match record.level() {
            Level::Info => INFO_HEADER,
            Level::Warn => WARN_HEADER,
            Level::Error => ERROR_HEADER,
            Level::Debug => DEBUG_HEADER,
            Level::Trace => TRACE_HEADER,
        };

        /* Assuming that the buffer is large enough to hold everything, we'll skip error handling. */

        // Print the level first
        let _ = buffer.push_str(header);

        // Print the module name next
        let _ = buffer.push_str(module);
        let _ = buffer.push_str("] ");

        // Print the actual message, but also avoid runtime formatting when possible
        match record.args().as_str() {
            Some(stat_str) => {
                // No string interpolation was used
                let _ = std::fmt::Write::write_str(&mut buffer, stat_str);
            }
            None => {
                // String interpolation was used, so we need to format it at runtime
                let _ = std::fmt::Write::write_fmt(&mut buffer, format_args!("{}", record.args()));
            }
        }

        // End it with a newline
        let _ = buffer.push('\n');

        // Write the buffer to stdout
        lock.write_all(buffer.as_bytes())
            .and_then(|()| lock.flush())
            .expect("Failed to write log message to stdout");
    }
}
