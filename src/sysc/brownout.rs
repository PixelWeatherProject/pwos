//! Brownout management driver.
//!
//! Currently this module only allows disabling the brownout protection feature.

use esp_idf_svc::sys::RTC_CNTL_BROWN_OUT_REG;
use std::ptr;

/// Disable the brownout detector by zeroing the brownout control register ([`RTC_CNTL_BROWN_OUT_REG`](RTC_CNTL_BROWN_OUT_REG).)
pub fn disable_brownout_detector() {
    unsafe {
        ptr::write_volatile(RTC_CNTL_BROWN_OUT_REG as *mut i32, 0);
    }
}
