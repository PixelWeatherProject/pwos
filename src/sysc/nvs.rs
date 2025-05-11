use super::{OsError, OsResult};
use esp_idf_svc::nvs::{EspDefaultNvsPartition, EspNvs, NvsDefault};

const NAMESPACE: &str = "pixelweather";
const LAST_OS_ERROR_KEY: &str = "last_error";

pub struct NonVolatileStorage(EspNvs<NvsDefault>);

impl NonVolatileStorage {
    pub fn new() -> OsResult<Self> {
        let nvs_partition = EspDefaultNvsPartition::take()?;
        let nvs = EspNvs::new(nvs_partition, NAMESPACE, true)?;

        Ok(Self(nvs))
    }

    pub fn store_last_os_error(&mut self, err: &OsError) -> OsResult<()> {
        self.0
            .set_str(LAST_OS_ERROR_KEY, err.to_string().as_str())?;
        Ok(())
    }

    pub fn get_last_os_error(&mut self) -> OsResult<Option<String>> {
        self.get_key(LAST_OS_ERROR_KEY, false)
    }

    pub fn clear_last_os_error(&mut self) -> OsResult<()> {
        self.delete_key(LAST_OS_ERROR_KEY)
    }

    fn delete_key(&mut self, key: &str) -> OsResult<()> {
        if !self.0.remove(key)? {
            return Err(OsError::InvalidNvsKey);
        }
        Ok(())
    }

    fn get_key(&mut self, key: &str, delete_if_exists: bool) -> OsResult<Option<String>> {
        let Some(length) = self.0.str_len(key)? else {
            return Ok(None);
        };

        let mut buffer = vec![0u8; length];
        self.0.get_str(key, &mut buffer)?;
        buffer.pop(); // Buffer contains a NULL-terminated string

        let err_string = String::from_utf8(buffer).map_err(|_| OsError::InvalidUtf8)?;

        if delete_if_exists {
            self.0.remove(key)?;
        }

        Ok(Some(err_string))
    }
}
