pub use esp_idf_svc::hal::reset::ResetReason;
use esp_idf_svc::sys::{esp_deep_sleep, esp_reset_reason, esp_restart};
use std::time::Duration;

const INFINITE_SLEEP_TIME: Duration = Duration::from_micros(2_629_746_000_000); /* 1 month */

pub fn deep_sleep(time: Option<Duration>) -> ! {
    let us = u64::try_from(time.unwrap_or(INFINITE_SLEEP_TIME).as_micros())
        .expect("Deep sleep duration is too long");

    unsafe {
        esp_deep_sleep(us);
    }
}

pub fn fake_sleep(time: Option<Duration>) -> ! {
    std::thread::sleep(time.unwrap_or(INFINITE_SLEEP_TIME));
    unsafe { esp_restart() };
}

pub fn get_reset_reason() -> ResetReason {
    unsafe { esp_reset_reason() }.into()
}

pub trait ResetReasonExt {
    fn is_abnormal(&self) -> bool;
}

impl ResetReasonExt for ResetReason {
    fn is_abnormal(&self) -> bool {
        !matches!(
            self,
            Self::PowerOn | Self::Software | Self::DeepSleep | Self::USBPeripheral | Self::JTAG
        )
    }
}
