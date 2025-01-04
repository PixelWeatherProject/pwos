use super::OsResult;
use esp_idf_svc::sys::{
    esp, esp_deep_sleep, esp_light_sleep_start, esp_restart, esp_sleep_enable_timer_wakeup,
};
use std::time::Duration;

const INFINITE_SLEEP_TIME: Duration = Duration::from_micros(2_629_746_000_000); /* 1 month */

pub fn deep_sleep(time: Option<Duration>) -> ! {
    let us = time.unwrap_or(INFINITE_SLEEP_TIME).as_micros() as u64;

    unsafe {
        esp_deep_sleep(us);
    }
}

pub fn light_sleep(time: Option<Duration>) -> OsResult<()> {
    let us = time.unwrap_or(INFINITE_SLEEP_TIME).as_micros() as u64;

    // Wake up after the specified time (if needed)
    esp!(unsafe { esp_sleep_enable_timer_wakeup(us) })?;

    // Begin sleep
    esp!(unsafe { esp_light_sleep_start() })?;

    unsafe { esp_restart() };
}
