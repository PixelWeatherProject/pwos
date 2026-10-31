use std::io::Write;

/// A fixed-capacity string that can be stored in RTC memory.
#[derive(Clone, Copy)]
pub struct RtcString<const N: usize>([u8; N]);

/// String length was too large to fit into [`RtcString`].
pub struct SourceTooLong;

impl<const N: usize> RtcString<N> {
    pub const fn new() -> Self {
        Self([0; N])
    }

    pub fn len(&self) -> usize {
        for (i, b) in self.0.iter().enumerate() {
            if *b == 0 {
                return i;
            }
        }

        unreachable!()
    }

    pub fn as_bytes(&self) -> &[u8] {
        &self.0[..self.len()]
    }

    pub fn as_str(&self) -> &str {
        // SAFETY: RtcString can only store valid UTF-8 encoded strings.
        unsafe { std::str::from_utf8_unchecked(self.as_bytes()) }
    }

    pub const fn capacity() -> usize {
        N - 1
    }
}

impl<const N: usize> TryFrom<&str> for RtcString<N> {
    type Error = SourceTooLong;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        let src_len = value.len();

        if src_len > usize::from(u16::MAX) || src_len > Self::capacity() {
            return Err(SourceTooLong);
        }

        let mut buffer = [0u8; N];
        let _ = (&mut buffer[..]).write_all(value.as_bytes());

        Ok(Self(buffer))
    }
}

impl<const N: usize> From<RtcString<N>> for Box<str> {
    fn from(value: RtcString<N>) -> Self {
        value.as_str().into()
    }
}
