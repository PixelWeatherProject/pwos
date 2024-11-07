use super::{OsError, OsResult};
use esp_idf_svc::sys::{
    esp, esp_ota_begin, esp_ota_end, esp_ota_get_next_update_partition, esp_ota_handle_t,
    esp_ota_set_boot_partition, esp_ota_write, esp_partition_t, EspError, ESP_OK, OTA_SIZE_UNKNOWN,
};
use std::{
    io::Error,
    marker::PhantomData,
    mem::{self, MaybeUninit},
    num::NonZero,
    ptr,
};

pub struct Ota<'p> {
    ota_partition: &'p esp_partition_t,
}

pub struct OtaUpdateHandle<'o> {
    partition_handle: &'o esp_partition_t,
    update_handle: NonZero<esp_ota_handle_t>,
}

impl<'p> Ota<'p> {
    pub fn new() -> OsResult<Self> {
        let Some(partition) = (unsafe { esp_ota_get_next_update_partition(ptr::null()).as_ref() })
        else {
            return Err(OsError::OtaUnsupported);
        };

        Ok(Self {
            ota_partition: dbg!(partition),
        })
    }

    pub fn update(&self, image_size: Option<usize>) -> OsResult<OtaUpdateHandle<'_>> {
        let mut handle: esp_ota_handle_t = Default::default();

        esp!(unsafe {
            esp_ota_begin(
                dbg!(self.ota_partition),
                image_size.unwrap_or(OTA_SIZE_UNKNOWN as usize),
                &mut handle,
            )
        })?;

        Ok(OtaUpdateHandle {
            partition_handle: dbg!(self.ota_partition),
            update_handle: handle.try_into().unwrap(),
        })
    }
}

impl<'o> OtaUpdateHandle<'o> {
    pub fn finish(self) -> OsResult<()> {
        dbg!(self.partition_handle);
        dbg!(self.update_handle);

        esp!(unsafe { esp_ota_end(self.update_handle.get()) })?;
        esp!(unsafe { esp_ota_set_boot_partition(self.partition_handle) })?;

        Ok(())
    }

    pub fn abort(self) {}
}

impl<'o> std::io::Write for OtaUpdateHandle<'o> {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        if buf.is_empty() {
            return Ok(0);
        }

        dbg!(self.update_handle.get());
        dbg!(self.partition_handle);

        esp!(unsafe { esp_ota_write(self.update_handle.get(), buf.as_ptr().cast(), buf.len()) })
            .map_err(Error::other)?;

        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}
