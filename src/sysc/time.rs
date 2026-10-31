//! Utilities for managing the system's wall-clock time.
//!
//! The ESP32S3 has no battery-backed real-time clock, so the time starts at the
//! Unix epoch (1970) after a cold boot. It must be set from an external source,
//! such as the PWMP server, before it can be relied upon. The RTC keeps counting
//! during deep sleep, so the time is preserved between wake-ups.

use crate::sysc::{rtcvar::RtcValue, OsError, OsResult};
use esp_idf_svc::sys::{settimeofday, time as raw_time, tm};

/// Maximum time between two time syncs, in seconds.
const RESYNC_INTERVAL: u64 = 7 * 24 * 60 * 60; // 7 days

/// Minimum accepted year for out-of-sync checks.
const MIN_YEAR: i32 = 2026;

#[link_section = ".rtc_noinit"]
static SYNC_EPOCH: RtcValue<u64> = RtcValue::new();

/// Sets the system time to the given Unix timestamp.
///
/// The `epoch` is the number of seconds elapsed since `1970-01-01 00:00:00 UTC`.
/// Sub-second precision is not supported, so the microseconds part is set to `0`.
///
/// After a successful call, [`std::time::SystemTime::now()`] will reflect the new time.
///
/// # Errors
/// Returns [`OsError::UnexpectedNull`] if `epoch` does not fit into an [`i64`], or
/// [`OsError::SetTime`] if the underlying [`settimeofday()`] call fails.
pub fn set_system_time(epoch: u64) -> OsResult<()> {
    let tv = esp_idf_svc::sys::timeval {
        tv_sec: i64::try_from(epoch).map_err(|_| OsError::UnexpectedNull)?,
        tv_usec: 0,
    };

    // SAFETY: `tv` is a valid, initialized timeval, timezone arg may be NULL.
    let ret = unsafe { settimeofday(&raw const tv, std::ptr::null()) };
    if ret != 0 {
        return Err(OsError::SetTime(std::io::Error::last_os_error()));
    }

    SYNC_EPOCH.set(epoch);
    Ok(())
}

/// Checks whether the system time should be synchronized again.
///
/// A resync is recommended if any of the following is true:
/// - The current year is earlier than [`MIN_YEAR`], meaning the time was likely never set
///   (e.g. after a cold boot, the clock starts at 1970).
/// - The current time is earlier than the last sync, meaning the clock was set backwards.
/// - More than [`RESYNC_INTERVAL`] seconds have elapsed since the last sync.
///
/// The time of the last sync is stored in RTC memory by [`set_system_time()`], so it
/// survives deep sleep, but not a power loss.
///
/// # Errors
/// Returns [`OsError::UnexpectedNull`] if the current time could not be read or converted.
pub fn resync_recommended() -> OsResult<bool> {
    let tm = get_raw_system_time()?;
    // `tm_year` counts years since 1900.
    if tm.tm_year + 1900 < MIN_YEAR {
        return Ok(true);
    }

    // SAFETY: NULL is a valid argument, the time is returned instead.
    let now = unsafe { raw_time(std::ptr::null_mut()) };
    let now = u64::try_from(now).map_err(|_| OsError::UnexpectedNull)?;

    // time of last sync
    let last = SYNC_EPOCH.get();

    // check if the clock was set backwards
    if now < last {
        return Ok(true);
    }

    if now - last > RESYNC_INTERVAL {
        return Ok(true);
    }

    Ok(false)
}

/// Returns the current system time broken down into its components, in UTC.
///
/// Note that the fields of [`tm`] follow the C conventions:
/// - `tm_year` is the number of years since 1900
/// - `tm_mon` is in the range `0..=11`
/// - `tm_mday` is in the range `1..=31`
///
/// # Errors
/// Returns [`OsError::UnexpectedNull`] if the underlying `gmtime_r()` call fails.
fn get_raw_system_time() -> OsResult<tm> {
    // SAFETY: NULL is a valid argument, the time is returned instead.
    let now = unsafe { raw_time(std::ptr::null_mut()) };

    // SAFETY: `tm` is plain old data, an all-zero value is valid.
    let mut tm: tm = unsafe { std::mem::zeroed() };

    // SAFETY: both pointers are valid for the duration of the call.
    let ret = unsafe { esp_idf_svc::sys::gmtime_r(&raw const now, &raw mut tm) };
    if ret.is_null() {
        return Err(OsError::UnexpectedNull);
    }

    Ok(tm)
}
