use super::OsResult;
use crate::{os_debug, os_info, os_warn};
use esp_idf_svc::{
    ota::{EspOta, SlotState},
    sys::esp,
};

#[link_section = ".rtc_noinit"]
static mut FAILIURES: u8 = 1;

const MAX_FAILIURES: u8 = 3;

pub fn check_for_rollback() -> OsResult<()> {
    os_debug!("Checking rollback");
    if needs_verification()? {
        os_warn!("Running unverified firmware");

        if failiure_limit_exceeded() {
            rollback()?;
        }
    }

    if needs_verification()? && failiure_limit_exceeded() {
        rollback()?;
    }

    Ok(())
}

pub fn increment_failiures_if_needed() -> OsResult<()> {
    if needs_verification()? {
        increment_failiures();
    }

    Ok(())
}

pub fn mark_verified_if_needed() -> OsResult<bool> {
    if needs_verification()? {
        mark_verified()?;
        return Ok(true);
    }

    Ok(false)
}

pub fn reset_failiures() {
    unsafe { FAILIURES = 1 };
}

fn needs_verification() -> OsResult<bool> {
    Ok(EspOta::new()?.get_running_slot()?.state != SlotState::Valid)
}

fn increment_failiures() {
    os_warn!(
        "This firmware has failed {}/{} times",
        unsafe { FAILIURES },
        MAX_FAILIURES
    );

    unsafe { FAILIURES += 1 };
}

fn failiure_limit_exceeded() -> bool {
    (unsafe { FAILIURES }) == MAX_FAILIURES
}

fn mark_verified() -> OsResult<()> {
    os_info!("Firmware validated successfully");

    unsafe { FAILIURES = 1 };

    Ok(EspOta::new()?.mark_running_slot_valid()?)
}

fn rollback() -> OsResult<()> {
    os_info!("Rolling back firmware");

    Ok(esp!(EspOta::new()?
        .mark_running_slot_invalid_and_reboot()
        .code())?)
}
