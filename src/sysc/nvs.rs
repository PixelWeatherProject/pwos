use super::{OsError, OsResult};
use crate::re_esp;
use esp_idf_svc::nvs::{EspDefaultNvsPartition, EspNvs, NvsDefault};

const NAMESPACE: &str = "pixelweather";
const LAST_OS_ERROR_KEY: &str = "last_error";

pub struct NonVolatileStorage(EspNvs<NvsDefault>);

impl NonVolatileStorage {
    pub fn new() -> OsResult<Self> {
        let nvs_partition = re_esp!(EspDefaultNvsPartition::take(), NvsInit)?;
        let nvs = re_esp!(EspNvs::new(nvs_partition, NAMESPACE, true), NvsInit)?;

        Ok(Self(nvs))
    }

    pub fn store_last_os_error(&mut self, err: &OsError) -> OsResult<()> {
        re_esp!(
            self.0.set_str(LAST_OS_ERROR_KEY, err.to_string().as_str()),
            NvsWrite
        )
    }

    pub fn get_last_os_error(&mut self) -> OsResult<Option<String>> {
        self.get_key(LAST_OS_ERROR_KEY, false)
    }

    pub fn clear_last_os_error(&mut self) -> OsResult<()> {
        self.delete_key(LAST_OS_ERROR_KEY)
    }

    fn delete_key(&mut self, key: &str) -> OsResult<()> {
        if !re_esp!(self.0.remove(key), NvsWrite)? {
            return Err(OsError::InvalidNvsKey);
        }
        Ok(())
    }

    fn get_key(&mut self, key: &str, delete_if_exists: bool) -> OsResult<Option<String>> {
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
