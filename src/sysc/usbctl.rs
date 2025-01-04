use esp_idf_svc::sys::usb_serial_jtag_is_connected;

/// Returns whether the device is connected **to a computer** over the USB port.
///
/// ### Warning
/// **This does not check if the serial port is open.**
///
/// ### Caveats
/// This will return `false` if the device is connected to a power-only USB device, e.g. a power-bank.
pub fn is_connected() -> bool {
    unsafe { usb_serial_jtag_is_connected() }
}
