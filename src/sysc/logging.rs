use log::{Level, LevelFilter, Log};

const BLACKLISTED_MODULES: [&str; 1] = ["esp_idf_svc"];
const COLOR_INFO: &str = "\x1b[1;94m";
const COLOR_WARN: &str = "\x1b[1;33m";
const COLOR_ERROR: &str = "\x1b[1;91m";
const COLOR_DEBUG: &str = "\x1b[1;95m";
const RESET_COLOR: &str = "\x1b[0m";

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

    fn flush(&self) {}

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

        match record.level() {
            Level::Info => print!("{COLOR_INFO}INFO{RESET_COLOR}  ["),
            Level::Warn => print!("{COLOR_WARN}WARN{RESET_COLOR}  ["),
            Level::Error => print!("{COLOR_ERROR}ERROR{RESET_COLOR} ["),
            Level::Debug => print!("{COLOR_DEBUG}DEBUG{RESET_COLOR} ["),
            Level::Trace => print!("TRACE ["),
        }

        println!("{module}] {}", record.args());
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
