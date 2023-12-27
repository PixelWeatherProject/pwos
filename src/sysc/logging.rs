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
