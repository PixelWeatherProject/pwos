use esp_idf_svc::sys::{esp_deep_sleep, esp_reset_reason, esp_reset_reason_t, esp_restart};
use std::time::Duration;

const INFINITE_SLEEP_TIME: Duration = Duration::from_micros(2_629_746_000_000); /* 1 month */

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResetReason {
    ///Reset reason can not be determined
    Unknown,
    ///Reset due to power-on event
    PowerOn,
    ///Reset by external pin (not applicable for ESP32)
    External,
    ///Software reset via `esp_restart()`
    Software,
    ///Software reset due to exception/panic
    Panic,
    ///Reset (software or hardware) due to interrupt watchdog
    WatchdogInterrupt,
    ///Reset due to task watchdog
    WatchdogTask,
    ///Reset due to other watchdogs
    WatchdogOther,
    ///Reset after exiting deep sleep mode
    DeepsleepWakeup,
    ///Brownout reset (software or hardware)
    Brownout,
    ///Reset over SDIO
    Sdio,
    ///Reset by USB peripheral
    Usb,
    ///Reset by JTAG
    Jtag,
    ///Reset due to efuse error
    Efuse,
    ///Reset due to power glitch detected
    PowerGlitch,
    ///Reset due to CPU lock up (double exception)
    CpuLockup,
}

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

impl ResetReason {
    pub const fn is_abnormal(self) -> bool {
        !matches!(
            self,
            Self::PowerOn | Self::Software | Self::DeepsleepWakeup | Self::Usb | Self::Jtag
        )
    }
}

impl From<esp_reset_reason_t> for ResetReason {
    fn from(value: esp_reset_reason_t) -> Self {
        match value {
            0 => Self::Unknown,
            1 => Self::PowerOn,
            2 => Self::External,
            3 => Self::Software,
            4 => Self::Panic,
            5 => Self::WatchdogInterrupt,
            6 => Self::WatchdogTask,
            7 => Self::WatchdogOther,
            8 => Self::DeepsleepWakeup,
            9 => Self::Brownout,
            10 => Self::Sdio,
            11 => Self::Usb,
            12 => Self::Jtag,
            13 => Self::Efuse,
            14 => Self::PowerGlitch,
            15 => Self::CpuLockup,
            _ => unreachable!(),
        }
    }
}
