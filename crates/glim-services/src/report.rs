//! Stream generated read-only text to a temporary file, then page it in the UI.
use anyhow::Result;
use std::{
    fmt,
    io::{BufWriter, Write},
    path::Path,
};

pub struct Report {
    writer: BufWriter<tempfile::NamedTempFile>,
    error: Option<std::io::Error>,
}
impl Report {
    pub fn new() -> Result<Self> {
        Ok(Self {
            writer: BufWriter::new(tempfile::NamedTempFile::new()?),
            error: None,
        })
    }
    pub fn push_str(&mut self, text: &str) {
        if self.error.is_none()
            && let Err(error) = self.writer.write_all(text.as_bytes())
        {
            self.error = Some(error);
        }
    }
    pub fn push(&mut self, ch: char) {
        self.push_str(ch.encode_utf8(&mut [0; 4]));
    }
    pub fn finish(mut self, original: &Path) -> Result<crate::paged::TextPage> {
        if let Some(error) = self.error {
            return Err(error.into());
        }
        self.writer.flush()?;
        let file = self
            .writer
            .into_inner()
            .map_err(|error| error.into_error())?;
        crate::paged::generated(original, file)
    }
}
impl fmt::Write for Report {
    fn write_str(&mut self, text: &str) -> fmt::Result {
        self.push_str(text);
        if self.error.is_some() {
            Err(fmt::Error)
        } else {
            Ok(())
        }
    }
}
