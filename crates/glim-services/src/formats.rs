//! Disk-backed container reports, with bounded page display and cancellable streaming.
use crate::{
    limits as reader_limits,
    paged::{self, TextPage},
    work::Cancellation,
};
use anyhow::{Context, Result, ensure};
use std::{
    fs::File,
    io::{BufReader, BufWriter, Read, Seek, SeekFrom, Write},
    path::Path,
};
pub fn supports(path: &Path) -> bool {
    matches!(
        extension(path).as_str(),
        "zip" | "tar" | "gz" | "bz2" | "xz" | "epub" | "odt" | "ods" | "odp" | "npy"
    )
}
fn extension(path: &Path) -> String {
    path.extension()
        .and_then(|v| v.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase()
}

pub fn read_preview(path: &Path) -> Result<TextPage> {
    read_preview_cancellable(path, &Cancellation::default())
}
pub fn read_preview_cancellable(path: &Path, cancel: &Cancellation) -> Result<TextPage> {
    let ext = extension(path);
    if let Some(compression) = match ext.as_str() {
        "gz" => Some(crate::streamed::Compression::Gzip),
        "bz2" => Some(crate::streamed::Compression::Bzip2),
        "xz" => Some(crate::streamed::Compression::Xz),
        _ => None,
    } {
        return paged::compressed(path, compression, cancel);
    }
    let mut output = tempfile::NamedTempFile::new()?;
    {
        let mut text = BufWriter::new(output.as_file_mut());
        writeln!(
            text,
            "{} · {}\n",
            ext.to_uppercase(),
            crate::locale::tr("Read-only preview")
        )?;
        match ext.as_str() {
            "zip" | "epub" | "odt" | "ods" | "odp" => {
                let mut archive = zip::ZipArchive::new(File::open(path)?)
                    .context("Invalid or truncated ZIP container")?;
                for index in 0..archive.len() {
                    cancel.check()?;
                    let mut entry = archive.by_index(index)?;
                    writeln!(
                        text,
                        "{} · {} bytes{}",
                        entry.name(),
                        entry.size(),
                        if entry.enclosed_name().is_some() {
                            String::new()
                        } else {
                            crate::locale::tr(" · Unsafe extraction path")
                        }
                    )?;
                    if (matches!(ext.as_str(), "odt" | "ods" | "odp")
                        && entry.name() == "content.xml")
                        || (ext == "epub" && entry.name().ends_with(".xhtml"))
                    {
                        // Preserve large XML completely in source form rather than rejecting it.
                        let mut prefix = Vec::new();
                        (&mut entry)
                            .take(reader_limits::ARCHIVE_BYTES + 1)
                            .read_to_end(&mut prefix)?;
                        if prefix.len() as u64 <= reader_limits::ARCHIVE_BYTES {
                            text.write_all(xml_text(&prefix)?.as_bytes())?;
                        } else {
                            text.write_all(&prefix)?;
                            cancel.copy(entry, &mut text)?;
                        }
                        writeln!(text)?;
                    }
                }
            }
            "tar" => {
                let mut archive = tar::Archive::new(BufReader::new(File::open(path)?));
                for entry in archive.entries()? {
                    cancel.check()?;
                    let entry = entry?;
                    let name = entry.path()?;
                    let safe = !name.is_absolute()
                        && !name
                            .components()
                            .any(|c| matches!(c, std::path::Component::ParentDir));
                    writeln!(
                        text,
                        "{} · {} bytes{}",
                        name.display(),
                        entry.size(),
                        if safe {
                            String::new()
                        } else {
                            crate::locale::tr(" · Unsafe extraction path")
                        }
                    )?;
                }
            }
            "npy" => numpy_stream(path, &mut text, cancel)?,
            _ => anyhow::bail!("Unsupported container"),
        }
        text.flush()?;
    }
    paged::generated(path, output)
}
fn xml_text(bytes: &[u8]) -> Result<String> {
    use quick_xml::events::Event;
    let mut reader = quick_xml::Reader::from_reader(bytes);
    let mut text = String::new();
    loop {
        match reader.read_event()? {
            Event::Text(value) => {
                text.push_str(&quick_xml::escape::unescape(&value.decode()?)?);
            }
            Event::End(_) => text.push('\n'),
            Event::DocType(_) => anyhow::bail!("Document type declarations are unsupported"),
            Event::Eof => break,
            _ => {}
        }
    }
    Ok(text)
}

fn numpy_stream(path: &Path, output: &mut impl Write, cancel: &Cancellation) -> Result<()> {
    let mut input = BufReader::new(File::open(path)?);
    let total = input.get_ref().metadata()?.len();
    let mut signature = [0; 8];
    input.read_exact(&mut signature)?;
    ensure!(
        signature.starts_with(b"\x93NUMPY"),
        "Invalid NumPy signature"
    );
    let version = signature[6];
    let length = match version {
        1 => {
            let mut n = [0; 2];
            input.read_exact(&mut n)?;
            u64::from(u16::from_le_bytes(n))
        }
        2 | 3 => {
            let mut n = [0; 4];
            input.read_exact(&mut n)?;
            u64::from(u32::from_le_bytes(n))
        }
        _ => anyhow::bail!("Unsupported NumPy version"),
    };
    let offset = input.stream_position()?;
    let data_offset = offset
        .checked_add(length)
        .filter(|end| *end <= total)
        .context("Truncated NumPy header")?;
    let mut prefix = Vec::new();
    (&mut input)
        .take(length.min(reader_limits::NUMPY_HEADER_BYTES as u64))
        .read_to_end(&mut prefix)?;
    writeln!(output, "NumPy v{version}")?;
    output.write_all(&prefix)?;
    cancel.copy(
        (&mut input).take(length - prefix.len() as u64),
        &mut *output,
    )?;
    writeln!(output)?;
    let header = std::str::from_utf8(&prefix).context("Invalid NumPy header")?;
    ensure!(
        !header.contains("'O") && !header.contains("\"O"),
        "Object arrays are unsupported; pickle is never executed"
    );
    let descriptor = ["<f8", "<f4", "<i8", "<i4", "|u1"]
        .into_iter()
        .find(|d| header.contains(d));
    let size = match descriptor {
        Some("<f8" | "<i8") => 8,
        Some("<f4" | "<i4") => 4,
        Some("|u1") => 1,
        _ => 0,
    };
    if size == 0 {
        writeln!(
            output,
            "{} · {} bytes",
            crate::locale::tr("Numeric preview unavailable for this data type"),
            total - data_offset
        )?;
        return Ok(());
    }
    let shape = header
        .split("shape")
        .nth(1)
        .and_then(|s| s.split('(').nth(1))
        .and_then(|s| s.split(')').next())
        .context("NumPy shape is unavailable")?;
    let mut count = 1u64;
    for dim in shape.split(',').map(str::trim).filter(|s| !s.is_empty()) {
        count = count
            .checked_mul(dim.parse()?)
            .context("NumPy dimensions overflow")?;
    }
    ensure!(
        count.checked_mul(size as u64) == Some(total - data_offset),
        "NumPy data length does not match shape"
    );
    input.seek(SeekFrom::Start(data_offset))?;
    let mut buffer = [0; 8];
    for index in 0..count {
        cancel.check()?;
        input.read_exact(&mut buffer[..size])?;
        let value = match descriptor {
            Some("<f8") => f64::from_le_bytes(buffer).to_string(),
            Some("<i8") => i64::from_le_bytes(buffer).to_string(),
            Some("<f4") => f32::from_le_bytes(buffer[..4].try_into()?).to_string(),
            Some("<i4") => i32::from_le_bytes(buffer[..4].try_into()?).to_string(),
            _ => buffer[0].to_string(),
        };
        writeln!(output, "[{index}] {value}")?;
    }
    Ok(())
}
