use esp_idf_svc::hal::rom::crc::crc32_le;

/// Computes a CRC-32/ISO-HDLC checksum (poly=0x04c11db7, reflected in/out).
///
/// Backed by the ESP ROM's `crc32_le`. The `!0xffffffff` seed accounts for the
/// ROM function one's-complementing the initial value internally.
pub fn crc32(data: &[u8]) -> u32 {
    crc32_le(!0xffff_ffff, data)
}
