use super::OsResult;
use crate::{os_debug, os_error, os_info, os_warn, sysc::OsError};
use esp_idf_svc::ota::{EspOta, EspOtaUpdate, FirmwareInfo, SlotState};
use pwmp_client::pwmp_msg::version::Version;
use std::{
    mem::MaybeUninit,
    ops::{AddAssign, Deref, DerefMut},
};

const MAX_FAILIURES: u8 = 3;

/// Number of times the current firmware has failed.
#[link_section = ".rtc_noinit"]
static mut FAILIURES: MaybeUninit<u8> = MaybeUninit::uninit();

/// Whether the last update has been reported back to the PWMP server.
#[link_section = ".rtc_noinit"]
static mut REPORTED: MaybeUninit<bool> = MaybeUninit::uninit();

/// Over-the-Air updates handler.
pub struct Ota(EspOta);

/// A handle for a pending update.
pub struct OtaHandle<'h>(Option<EspOtaUpdate<'h>>);

#[allow(static_mut_refs)]
impl Ota {
    pub fn new() -> OsResult<Self> {
        Ok(Self(EspOta::new()?))
    }

    pub fn current_verified(&self) -> OsResult<bool> {
        Ok(self.0.get_running_slot()?.state == SlotState::Valid)
    }

    #[allow(clippy::unused_self)]
    pub fn mark_reported(&self) {
        // SAFETY: This method is only called after an update has been performed. So `REPORTED` is initialized to `false`.
        unsafe { REPORTED.write(true) };
    }

    pub fn report_needed(&self) -> OsResult<bool> {
        // The current firmware might be verified, but it could be a previous version.
        if self.current_verified()? && !self.rollback_detected()? {
            os_debug!("Skipping report check on verified firmware");
            return Ok(false);
        }

        // SAFETY: This method is only called after an update has been performed. So `REPORTED` is initialized.
        Ok(unsafe { !REPORTED.assume_init() })
    }

    pub fn rollback_detected(&self) -> OsResult<bool> {
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

        // SAFETY: This part can only be reached after an update has been performed. So `REPORTED` is initialized.
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

        // SAFETY: This part can only be reached after an update has been performed. So `REPORTED` is initialized.
        let fail_count = unsafe { FAILIURES.assume_init_mut() };
        fail_count.add_assign(1);

        os_warn!("Firmware has failed {}/{} times", fail_count, MAX_FAILIURES);
        Ok(())
    }

    #[cfg(debug_assertions)]
    pub fn current_version(&self) -> OsResult<Option<Version>> {
        let slot = self.0.get_running_slot()?;

        let Some(info) = slot.firmware else {
            return Ok(None);
        };

        let Some(version) = Self::parse_info_version(&info) else {
            os_warn!("Current firmware has an invalid version string");
            return Ok(None);
        };

        Ok(Some(version))
    }

    pub fn previous_version(&self) -> OsResult<Option<Version>> {
        let Some(slot) = self.0.get_last_invalid_slot()? else {
            return Ok(None);
        };

        let Some(info) = slot.firmware else {
            return Ok(None);
        };

        let Some(version) = Self::parse_info_version(&info) else {
            os_error!("Previous firmware has an invalid version string");
            return Err(OsError::IllegalFirmwareVersion);
        };

        Ok(Some(version))
    }

    fn parse_info_version(info: &FirmwareInfo) -> Option<Version> {
        /*
         * If ESP-IDF uses `git describe` to get a version string, it will
         * look like this: `v2.0.0-rc3-8-g1a1ba69`.
         *
         * This method assumes the above format.
         */

        // Index of the first `-`
        let dash_index = info.version.find('-')?;

        // Cut the version string
        let slice = &info.version[1..dash_index];

        Version::parse(slice)
    }
}

impl OtaHandle<'_> {
    pub fn cancel(mut self) -> OsResult<()> {
        let inner = self.0.take().expect("Handle was NULL");
        inner.abort()?;

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

impl DerefMut for OtaHandle<'_> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        // SAFETY: Handle is always Some() at this point
        unsafe { self.0.as_mut().unwrap_unchecked() }
    }
}

#[allow(static_mut_refs)]
impl Drop for OtaHandle<'_> {
    fn drop(&mut self) {
        if self.0.is_none() {
            return;
        }

        os_debug!("Finalizing update");

        // SAFETY: Handle is always Some() at this point
        let mut handle = unsafe { self.0.take().unwrap_unchecked() };

        handle.flush().expect("Failed to flush OTA write");
        handle.complete().expect("Failed to complete update");

        unsafe {
            FAILIURES.write(0);
            REPORTED.write(false);
        }

        // Null-safety of `self.0`:
        // The handle can never be used after this drop.
        // Therefore, no calls to `unwrap_unchecked()` can be made in the Deref implementations, and no UB can occur.
    }
}
