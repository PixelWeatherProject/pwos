use crate::{re_esp, sysc::OsResult};
use esp_idf_svc::{
    hal::delay::BLOCK,
    sys::{
        esp, usb_serial_jtag_driver_config_t, usb_serial_jtag_driver_install,
        usb_serial_jtag_driver_uninstall, usb_serial_jtag_read_bytes, usb_serial_jtag_write_bytes,
    },
};

pub const TX_BUFFER_SIZE: u32 = 256;
pub const RX_BUFFER_SIZE: u32 = 256;

pub struct JtagSerialDriver;

impl JtagSerialDriver {
    pub fn new() -> OsResult<Self> {
        let mut cfg = usb_serial_jtag_driver_config_t {
            tx_buffer_size: TX_BUFFER_SIZE,
            rx_buffer_size: RX_BUFFER_SIZE,
        };

        re_esp!(
            esp!(unsafe { usb_serial_jtag_driver_install(&raw mut cfg) }),
            JtagSerialInit
        )?;

        Ok(Self)
    }

    pub fn print<S: AsRef<str>>(&self, text: S) {
        self.write_bytes(text.as_ref().as_bytes());
    }

    #[allow(clippy::unused_self)]
    pub fn read_byte(&self) -> u8 {
        let mut byte = 0;

        // TODO: handle 0 read bytes
        unsafe { usb_serial_jtag_read_bytes((&raw mut byte).cast(), 1, BLOCK) };

        byte
    }

    #[allow(clippy::unused_self)]
    pub fn write_bytes(&self, bytes: &[u8]) {
        // this does not return an error, but the number of bytes pushed
        unsafe { usb_serial_jtag_write_bytes(bytes.as_ptr().cast(), bytes.len(), BLOCK) };
    }
}

impl Drop for JtagSerialDriver {
    fn drop(&mut self) {
        if let Err(why) = esp!(unsafe { usb_serial_jtag_driver_uninstall() }) {
            log::error!("Failed to shutdown USB JTAG serial driver: {why}");
        }
    }
}
