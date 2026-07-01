use std::io::Write;

pub struct JoinedWriter<F: Write, S: Write> {
    first: F,
    second: Option<S>,
}

impl<F: Write, S: Write> JoinedWriter<F, S> {
    pub const fn new(first: F, second: S) -> Self {
        Self {
            first,
            second: Some(second),
        }
    }

    pub fn disable_second(&mut self) {
        self.second = None;
    }
}

impl<F: Write, S: Write> Write for JoinedWriter<F, S> {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let mut written = self.first.write(buf)?;

        if let Some(second) = &mut self.second {
            written += second.write(buf)?;
        }

        Ok(written)
    }

    fn write_all(&mut self, buf: &[u8]) -> std::io::Result<()> {
        self.first.write_all(buf)?;

        if let Some(second) = &mut self.second {
            second.write_all(buf)?;
        }

        Ok(())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.first.flush()?;

        if let Some(second) = &mut self.second {
            second.flush()?;
        }

        Ok(())
    }
}
