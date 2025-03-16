use super::OsResult;
use crate::{os_debug, os_error, os_info, os_warn, sysc::OsError};
use esp_idf_svc::ota::{EspOta, EspOtaUpdate, FirmwareInfo, SlotState};
use pwmp_client::pwmp_msg::version::Version;
use std::{
    ops::{Deref, DerefMut},
    sync::atomic::{AtomicBool, AtomicU8, Ordering},
};

const MAX_FAILIURES: u8 = 3;

/// Number of times the current firmware has failed.
#[link_section = ".rtc.data"]
static FAILIURES: AtomicU8 = AtomicU8::new(0);

/// Whether the last update has been reported back to the PWMP server.
#[link_section = ".rtc.data"]
static REPORTED: AtomicBool = AtomicBool::new(false);

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
        REPORTED.store(true, Ordering::SeqCst);
    }

    pub fn report_needed(&self) -> OsResult<bool> {
        // The current firmware might be verified, but it could be a previous version.
        if self.current_verified()? && !self.rollback_detected()? {
            os_debug!("Skipping report check on verified firmware");
            return Ok(false);
        }

        Ok(!REPORTED.load(Ordering::SeqCst))
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

        if FAILIURES.load(Ordering::SeqCst) >= MAX_FAILIURES {
            os_info!("Rolling back to previous version");
            self.0.mark_running_slot_invalid_and_reboot();
        }

        Ok(())
    }

    pub fn inc_failiures(&self) -> OsResult<()> {
        if self.current_verified()? {
            return Ok(());
        }

        let counter = FAILIURES.fetch_add(1, Ordering::SeqCst) /* returns old value */ + 1;
        os_warn!("Firmware has failed {}/{} times", counter, MAX_FAILIURES);

        Ok(())
    }

    #[cfg(debug_assertions)]
    pub fn current_version(&self) -> OsResult<Option<Version>> {
        let slot = self.0.get_running_slot()?;

        let Some(info) = slot.firmware else {
            return Err(OsError::MissingPartitionMetadata);
        };

        let Some(version) = Self::parse_info_version(&info) else {
            os_error!("Current firmware has an invalid version string");
            return Err(OsError::IllegalFirmwareVersion);
        };

        Ok(Some(version))
    }

    pub fn previous_version(&self) -> OsResult<Option<Version>> {
        let Some(slot) = self.0.get_last_invalid_slot()? else {
            return Ok(None);
        };

        let Some(info) = slot.firmware else {
            return Err(OsError::MissingPartitionMetadata);
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

        FAILIURES.store(0, Ordering::SeqCst);
        REPORTED.store(false, Ordering::SeqCst);

        // Null-safety of `self.0`:
        // The handle can never be used after this drop.
        // Therefore, no calls to `unwrap_unchecked()` can be made in the Deref implementations, and no UB can occur.
    }
}
