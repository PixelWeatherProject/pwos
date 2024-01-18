#[macro_export]
macro_rules! os_info {
    ($($arg:tt)+) => {
        defmt::info!($($arg)+)
    };
}

#[macro_export]
macro_rules! os_warn {
    ($($arg:tt)+) => {
        defmt::warn!($($arg)+)
    };
}

#[macro_export]
macro_rules! os_error {
    ($($arg:tt)+) => {
        defmt::error!($($arg)+)
    };
}

#[macro_export]
macro_rules! os_debug {
    ($($arg:tt)+) => {
        #[cfg(debug_assertions)]
        defmt::debug!($($arg)+)
    };
}
