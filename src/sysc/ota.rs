use super::OsResult;
use crate::{null_check, re_esp, sysc::OsError};
use esp_idf_svc::ota::{EspOta, EspOtaUpdate, FirmwareInfo, SlotState};
use pwmp_client::pwmp_msg::version::Version;
use std::{
    ops::{Deref, DerefMut},
    sync::atomic::{AtomicBool, AtomicU8, Ordering},
};

/// Maximum number of times the firmware can fail.
const MAX_FAILIURES: u8 = 3;

/// Number of times the current firmware has failed.
#[link_section = ".rtc.data"]
static FAILIURES: AtomicU8 = AtomicU8::new(0);

/// Whether the last update has been reported back to the PWMP server.
#[link_section = ".rtc.data"]
static REPORTED: AtomicBool = AtomicBool::new(false);

/// A high-level Over-the-Air updates driver/wrapper.
///
/// Provides a simpler API for dealing with firmware updates.
pub struct Ota(EspOta);

/// A handle for a pending update.
pub struct OtaHandle<'h>(Option<EspOtaUpdate<'h>>);

#[allow(static_mut_refs)]
impl Ota {
    /// Initialize a new driver.
    ///
    /// # Errors
    /// Returns an error if the underlying OTA driver fails ([`EspOta::new`]).
    pub fn new() -> OsResult<Self> {
        Ok(Self(re_esp!(EspOta::new(), OtaInit)?))
    }

    /// Returns whether the currently running firmware is marked as [`Valid`](SlotState::Valid).
    ///
    /// # Errors
    /// Returns an error if the underlying OTA driver fails ([`EspOta::get_running_slot`]).
    pub fn current_verified(&self) -> OsResult<bool> {
        Ok(re_esp!(self.0.get_running_slot(), OtaSlot)?.state == SlotState::Valid)
    }

    /// Mark the current firmware as "reported", signaling that the PWMP server
    /// was told to mark the firmware update as successfull or not.
    #[allow(clippy::unused_self)]
    pub fn mark_reported(&self) {
        REPORTED.store(true, Ordering::SeqCst);
    }

    /// Returns whether the last firmware update needs reporting to the PWMP server.
    ///
    /// # Errors
    /// Returns an error if the underlying OTA driver fails.
    pub fn report_needed(&self) -> OsResult<bool> {
        // The current firmware might be verified, but it could be a previous version.
        if self.current_verified()? && !self.rollback_detected()? {
            log::debug!("Skipping report check on verified firmware");
            return Ok(false);
        }

        Ok(!REPORTED.load(Ordering::SeqCst))
    }

    /// Returns whether a firmware rollback has been detected.
    ///
    /// # Errors
    /// Returns an error if the underlying OTA driver fails ([`EspOta::get_last_invalid_slot`]).
    pub fn rollback_detected(&self) -> OsResult<bool> {
        Ok(re_esp!(self.0.get_last_invalid_slot(), OtaSlot)?.is_some())
    }

    /// Initiates a new firmware update and returns a handle for it.
    ///
    /// The returned handle can then be used to write the new firmware
    /// to the flash memory, or to abort the update.
    ///
    /// # Errors
    /// Returns an error if the underlying OTA driver fails ([`EspOta::initiate_update`]).
    pub fn begin_update(&mut self) -> OsResult<OtaHandle<'_>> {
        log::debug!("Initializing update");
        Ok(OtaHandle(Some(re_esp!(self.0.initiate_update(), OtaInit)?)))
    }

    /// Returns whether a firmware rollback is needed.
    ///
    /// If the currently running firmware failed more than [`MAX_FAILIURES`]
    /// times, this will return `true`
    ///
    /// # Errors
    /// Returns an error if the underlying OTA driver fails.
    pub fn rollback_if_needed(&mut self) -> OsResult<()> {
        if self.current_verified()? {
            return Ok(());
        }

        if FAILIURES.load(Ordering::SeqCst) >= MAX_FAILIURES {
            log::info!("Rolling back to previous version");
            self.0.mark_running_slot_invalid_and_reboot();
        }

        Ok(())
    }

    /// Increment the number of failiures for this firmware.
    ///
    /// This should be called before the system goes to sleep. It's safe to call
    /// even if the current firmware is marked as [`Valid`](SlotState::Valid), in which case
    /// nothing will be done.
    ///
    /// # Errors
    /// Fails if [`current_verified`](Self::current_verified) returns an error.
    pub fn inc_failiures(&self) -> OsResult<()> {
        // if the current firmware is verified, we don't need to increment anything
        if self.current_verified()? {
            return Ok(());
        }

        let counter = FAILIURES.fetch_add(1, Ordering::SeqCst) /* returns old value */ + 1;
        log::warn!("Firmware has failed {counter}/{MAX_FAILIURES} times");

        Ok(())
    }

    /// Returns the version of the currently running firmware.
    ///
    /// *This method should only be used in debug builds.*
    ///
    /// If the version number is not available, [`Option::None`] is returned.
    ///
    /// # Errors
    /// Returns an error if the underlying OTA driver fails.
    pub fn current_version(&self) -> OsResult<Option<Version>> {
        let slot = crate::re_esp!(self.0.get_running_slot(), OtaSlot)?;

        let Some(info) = slot.firmware else {
            return Err(OsError::MissingPartitionMetadata);
        };

        let Some(version) = Self::parse_info_version(&info) else {
            log::error!("Current firmware has an invalid version string");
            return Err(OsError::IllegalFirmwareVersion);
        };

        Ok(Some(version))
    }

    /// Returns the version of the previous firmware from the flash.
    ///
    /// If the version number is not available, [`Option::None`] is returned.
    ///
    /// # Errors
    /// Returns an error if the underlying OTA driver fails.
    pub fn previous_version(&self) -> OsResult<Option<Version>> {
        let Some(slot) = re_esp!(self.0.get_last_invalid_slot(), OtaSlot)? else {
            return Ok(None);
        };

        let Some(info) = slot.firmware else {
            return Err(OsError::MissingPartitionMetadata);
        };

        let Some(version) = Self::parse_info_version(&info) else {
            log::error!("Previous firmware has an invalid version string");
            return Err(OsError::IllegalFirmwareVersion);
        };

        Ok(Some(version))
    }

    /// Parses the raw version string of a firmware on flash.
    ///
    /// If the parsing fails, [`Option::None`] is returned.
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
    /// Aborts the firmware update.
    ///
    /// # Errors
    /// Returns an error if the underlying OTA driver fails.
    pub fn cancel(mut self) -> OsResult<()> {
        let inner = null_check!(self.0.take());
        re_esp!(inner.abort(), OtaAbort)?;

        Ok(())
    }
}

impl<'h> Deref for OtaHandle<'h> {
    type Target = EspOtaUpdate<'h>;

    fn deref(&self) -> &Self::Target {
        self.0
            .as_ref()
            .ok_or(OsError::UnexpectedNull)
            .expect("Unexpected NULL")
    }
}

impl DerefMut for OtaHandle<'_> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.0
            .as_mut()
            .ok_or(OsError::UnexpectedNull)
            .expect("Unexpected NULL")
    }
}

#[allow(static_mut_refs)]
impl Drop for OtaHandle<'_> {
    fn drop(&mut self) {
        let Some(mut handle) = self.0.take() else {
            return;
        };

        log::error!("Finalizing update");

        handle.flush().expect("Failed to flush OTA write");
        handle.complete().expect("Failed to complete update");

        FAILIURES.store(0, Ordering::SeqCst);
        REPORTED.store(false, Ordering::SeqCst);

        // Null-safety of `self.0`:
        // The handle can never be used after this drop.
        // Therefore, no calls to `unwrap_unchecked()` can be made in the Deref implementations, and no UB can occur.
    }
}
