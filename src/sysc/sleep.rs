use esp_idf_svc::sys::{esp_deep_sleep, esp_restart};
use std::time::Duration;

const INFINITE_SLEEP_TIME: Duration = Duration::from_micros(2_629_746_000_000); /* 1 month */

pub fn deep_sleep(time: Option<Duration>) -> ! {
    let us = time.unwrap_or(INFINITE_SLEEP_TIME).as_micros() as u64;

    unsafe {
        esp_deep_sleep(us);
    }
}

pub fn fake_sleep(time: Option<Duration>) -> ! {
    std::thread::sleep(time.unwrap_or(INFINITE_SLEEP_TIME));
    unsafe { esp_restart() };
}
