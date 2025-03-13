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
    let us = time.unwrap_or(INFINITE_SLEEP_TIME).as_micros() as u64;

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
            ResetReason::PowerOn
                | ResetReason::Software
                | ResetReason::DeepsleepWakeup
                | ResetReason::Usb
                | ResetReason::Jtag
        )
    }
}

impl From<esp_reset_reason_t> for ResetReason {
    fn from(value: esp_reset_reason_t) -> Self {
        match value {
            0 => ResetReason::Unknown,
            1 => ResetReason::PowerOn,
            2 => ResetReason::External,
            3 => ResetReason::Software,
            4 => ResetReason::Panic,
            5 => ResetReason::WatchdogInterrupt,
            6 => ResetReason::WatchdogTask,
            7 => ResetReason::WatchdogOther,
            8 => ResetReason::DeepsleepWakeup,
            9 => ResetReason::Brownout,
            10 => ResetReason::Sdio,
            11 => ResetReason::Usb,
            12 => ResetReason::Jtag,
            13 => ResetReason::Efuse,
            14 => ResetReason::PowerGlitch,
            15 => ResetReason::CpuLockup,
            _ => unreachable!(),
        }
    }
}
