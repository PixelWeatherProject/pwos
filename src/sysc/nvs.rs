use super::{OsError, OsResult};
use crate::re_esp;
use esp_idf_svc::nvs::{EspDefaultNvsPartition, EspNvs, NvsDefault};

/// Namespace used for the firmware's keys.
const NAMESPACE: &str = "pixelweather";
/// Key name for the last system error.
const LAST_OS_ERROR_KEY: &str = "last_error";

/// A high-level wrapper/driver for the Non-volatile storage driver.
///
/// Provides a simpler API for getting and storing data in the
/// NVS partition.
pub struct NonVolatileStorage(EspNvs<NvsDefault>);

impl NonVolatileStorage {
    /// Initialize the driver on the default NVS partition.
    pub fn new() -> OsResult<Self> {
        let nvs_partition = re_esp!(EspDefaultNvsPartition::take(), NvsInit)?;
        let nvs = re_esp!(EspNvs::new(nvs_partition, NAMESPACE, true), NvsInit)?;

        Ok(Self(nvs))
    }

    /// Stores the last [`OsError`] that caused the firmware to fail.
    ///
    /// # Errors
    /// Returns an error if the underlying NVS driver fails ([`EspNvs::set_str`]).
    pub fn store_last_os_error(&self, err: &OsError) -> OsResult<()> {
        re_esp!(
            self.0.set_str(LAST_OS_ERROR_KEY, err.to_string().as_str()),
            NvsWrite
        )
    }

    /// Gets the last system error ([`OsError`]) that caused the
    /// firmware to fail stored using [`store_last_os_error()`](Self::store_last_os_error).
    ///
    /// Returns [`Option::None`] if no error was stored previously.
    ///
    /// # Errors
    /// Returns an error if the underlying NVS driver fails.
    pub fn get_last_os_error(&self) -> OsResult<Option<String>> {
        self.get_key(LAST_OS_ERROR_KEY, false)
    }

    /// Deletes the last system error ([`OsError`]) stored using
    /// [`store_last_os_error()`](Self::store_last_os_error) from the NVS.
    ///
    /// # Errors
    /// Returns an error if the underlying NVS driver fails.
    pub fn clear_last_os_error(&self) -> OsResult<()> {
        self.delete_key(LAST_OS_ERROR_KEY)
    }

    /// Deletes a value by it's key from the NVS.
    ///
    /// # Errors
    /// Returns an error if the underlying NVS driver fails, or the specified key
    /// does not exist.
    fn delete_key(&self, key: &str) -> OsResult<()> {
        if !re_esp!(self.0.remove(key), NvsWrite)? {
            return Err(OsError::InvalidNvsKey);
        }
        Ok(())
    }

    /// Get a value by it's key from the NVS.
    ///
    /// If `delete_if_exists` is `true`, the value will be immediately deleted from the NVS
    /// after reading.
    ///
    /// Returns [`Option::None`] if the key does not exist.
    ///
    /// # Errors
    /// Returns an error if the underlying NVS driver fails ([`EspNvs::str_len`], [`EspNvs::get_str`],
    /// [`EspNvs::remove`]).
    fn get_key(&self, key: &str, delete_if_exists: bool) -> OsResult<Option<String>> {
        let Some(length) = re_esp!(self.0.str_len(key), NvsRead)? else {
            return Ok(None);
        };

        let mut buffer = vec![0u8; length];
        re_esp!(self.0.get_str(key, &mut buffer), NvsRead)?;
        buffer.pop(); // Buffer contains a NULL-terminated string

        let err_string = String::from_utf8(buffer)?;

        if delete_if_exists {
            re_esp!(self.0.remove(key), NvsWrite)?;
        }

        Ok(Some(err_string))
    }
}
