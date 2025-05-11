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
        let Some(length) = self.0.str_len(LAST_OS_ERROR_KEY)? else {
            return Ok(None);
        };

        let mut buffer = vec![0u8; length];
        self.0.get_str(LAST_OS_ERROR_KEY, &mut buffer)?;
        let err_string = String::from_utf8(buffer).expect("Last OS error in NVS is not UTF-8");

        self.0.remove(LAST_OS_ERROR_KEY)?;
        Ok(Some(err_string))
    }
}
