use std::io::Write;

/// A fixed-capacity string that can be stored in RTC memory.
#[derive(Clone, Copy)]
pub struct RtcString<const N: usize> {
    len: u16,
    buf: [u8; N],
}

/// String length was too large to fit into [`RtcString`].
pub struct SourceTooLong;

impl<const N: usize> RtcString<N> {
    pub const fn new() -> Self {
        Self {
            len: 0,
            buf: [0; N],
        }
    }

    pub fn as_bytes(&self) -> &[u8] {
        &self.buf[..usize::from(self.len)]
    }

    pub fn as_str(&self) -> &str {
        // SAFETY: RtcString can only store valid UTF-8 encoded strings.
        unsafe { std::str::from_utf8_unchecked(self.as_bytes()) }
    }

    pub const fn capacity() -> usize {
        N
    }
}

impl<const N: usize> TryFrom<&str> for RtcString<N> {
    type Error = SourceTooLong;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        if value.len() > usize::from(u16::MAX) || value.len() > N {
            return Err(SourceTooLong);
        }

        let mut buffer = [0u8; N];
        let _ = (&mut buffer[..]).write_all(value.as_bytes());

        // SAFETY: The above check ensures that the length of the string fits into a `u16`.
        let len = unsafe { u16::try_from(value.len()).unwrap_unchecked() };

        Ok(Self { len, buf: buffer })
    }
}

impl<const N: usize> From<RtcString<N>> for Box<str> {
    fn from(value: RtcString<N>) -> Self {
        value.as_str().into()
    }
}
