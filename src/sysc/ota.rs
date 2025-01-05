use super::OsResult;
use crate::{os_debug, os_info, os_warn};
use esp_idf_svc::ota::{EspOta, EspOtaUpdate, SlotState};
use std::{
    mem::MaybeUninit,
    ops::{AddAssign, Deref, DerefMut},
};

const MAX_FAILIURES: u8 = 3;

#[link_section = ".rtc_noinit"]
static mut FAILIURES: MaybeUninit<u8> = MaybeUninit::uninit();

#[link_section = ".rtc_noinit"]
static mut REPORTED: MaybeUninit<bool> = MaybeUninit::uninit();

pub struct Ota(EspOta);

pub struct OtaHandle<'h>(Option<EspOtaUpdate<'h>>);

impl Ota {
    pub fn new() -> OsResult<Self> {
        Ok(Self(EspOta::new()?))
    }

    pub fn current_verified(&self) -> OsResult<bool> {
        Ok(self.0.get_running_slot()?.state == SlotState::Valid)
    }

    pub fn mark_reported(&mut self) {
        unsafe { REPORTED.write(true) };
    }

    pub fn report_needed(&mut self) -> OsResult<bool> {
        if self.current_verified()? && !self.rollback_detected()? {
            os_debug!("Skipping report check on verified firmware");
            return Ok(false);
        }

        Ok(unsafe { !REPORTED.assume_init() })
    }

    pub fn rollback_detected(&mut self) -> OsResult<bool> {
        Ok(self.0.get_last_invalid_slot()?.is_some())
    }

    pub fn begin_update(&mut self) -> OsResult<OtaHandle<'_>> {
        os_debug!("Initializing update");
        Ok(OtaHandle(Some(self.0.initiate_update()?)))
    }

    pub fn rollback_if_needed(&mut self) -> OsResult<()> {
        if self.current_verified()? {
            return Ok(());
        }

        let fails = unsafe { FAILIURES.assume_init() };

        if fails >= MAX_FAILIURES {
            os_info!("Rolling back to previous version");
            self.0.mark_running_slot_invalid_and_reboot();
        }

        Ok(())
    }

    pub fn inc_failiures(&self) -> OsResult<()> {
        if self.current_verified()? {
            return Ok(());
        }

        let fail_count = unsafe { FAILIURES.assume_init_mut() };
        fail_count.add_assign(1);

        os_warn!("Firmware has failed {}/{} times", fail_count, MAX_FAILIURES);
        Ok(())
    }
}

impl<'h> Deref for OtaHandle<'h> {
    type Target = EspOtaUpdate<'h>;

    fn deref(&self) -> &Self::Target {
        // SAFETY: Handle is always Some() at this point
        unsafe { self.0.as_ref().unwrap_unchecked() }
    }
}

impl<'h> DerefMut for OtaHandle<'h> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        // SAFETY: Handle is always Some() at this point
        unsafe { self.0.as_mut().unwrap_unchecked() }
    }
}

impl<'h> Drop for OtaHandle<'h> {
    fn drop(&mut self) {
        os_debug!("Finalizing update");

        // SAFETY: Handle is always Some() at this point
        let mut handle = unsafe { self.0.take().unwrap_unchecked() };

        handle.flush().expect("Failed to flush OTA write");
        handle.complete().expect("Failed to complete update");

        unsafe {
            FAILIURES.write(0);
            REPORTED.write(false);
        }
    }
}
