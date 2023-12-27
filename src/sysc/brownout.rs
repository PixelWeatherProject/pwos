use esp_idf_svc::sys::RTC_CNTL_BROWN_OUT_REG;
use std::ptr;

pub fn disable_brownout_detector() {
    unsafe {
        ptr::write(RTC_CNTL_BROWN_OUT_REG as *mut i32, 0);
    }
}
