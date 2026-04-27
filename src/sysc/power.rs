pub use esp_idf_svc::hal::reset::ResetReason;
use esp_idf_svc::{
    hal::reset::restart,
    sys::{esp_deep_sleep, esp_reset_reason},
};
use std::time::Duration;

const INFINITE_SLEEP_TIME: Duration = Duration::from_secs(2_629_746); /* 1 month */

/// Puts the node into sleep mode, while automatically selecting the proper
/// sleep type (*deep*/*fake*) depending on whether the node is powered trough USB
/// or battery.
pub fn mcu_sleep(time: Option<Duration>) -> ! {
    if super::usbctl::is_connected() {
        log::debug!("Using fake sleep instead of deep sleep");
        fake_sleep(time)
    } else {
        deep_sleep(time)
    }
}

/// Puts the device into deep sleep for the specified amount of time, or indefinetly, if the duration is [None].
fn deep_sleep(time: Option<Duration>) -> ! {
    let us = u64::try_from(time.unwrap_or(INFINITE_SLEEP_TIME).as_micros())
        .expect("Deep sleep duration is too long");

    unsafe {
        esp_deep_sleep(us);
    }
}

/// Simulates a sleep using [`std::thread::sleep`] for the specified amount of time, or
/// indefinetly, if the duration is [None] and then performs a software reset.
///
/// Useful for when debugging over USB where deep sleep causes the USB controller
/// to shut down and therefore disconnect from the computer.
fn fake_sleep(time: Option<Duration>) -> ! {
    std::thread::sleep(time.unwrap_or(INFINITE_SLEEP_TIME));
    restart();
}

/// Returns the reset reason.
pub fn get_reset_reason() -> ResetReason {
    // SAFETY: Calling a safe C function.
    unsafe { esp_reset_reason() }.into()
}

/// A trait for extending [`ResetReason`].
pub trait ResetReasonExt {
    /// Returns whether the reset reason is abnormal (caused by a crash/error).
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
