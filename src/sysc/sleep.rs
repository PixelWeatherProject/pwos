use esp_idf_svc::sys::esp_deep_sleep;
use std::time::Duration;

const INFINITE_SLEEP_TIME: Duration = Duration::from_micros(2_629_746_000_000); /* 1 month */

pub fn deep_sleep(time: Option<Duration>) {
    unsafe {
        esp_deep_sleep(time.unwrap_or(INFINITE_SLEEP_TIME).as_micros() as u64);
    }
}
