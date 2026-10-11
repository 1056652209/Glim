//! First-pass container previews. Never extract files or execute document code.
use crate::binary::BytePreview;
use anyhow::{Context, Result, ensure};
use std::{
    fmt::Write as _,
    io::{Cursor, Read},
    path::Path,
};
const LIMIT: u64 = 32 * 1024 * 1024;
fn bounded(reader: impl Read) -> Result<Vec<u8>> {
    let mut bytes = Vec::new();
    reader.take(LIMIT + 1).read_to_end(&mut bytes)?;
    ensure!(bytes.len() as u64 <= LIMIT, "Preview exceeds 32 MiB limit");
    Ok(bytes)
}
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
pub fn read_preview(path: &Path) -> Result<BytePreview> {
    let bytes = bounded(std::fs::File::open(path)?)?;
    let ext = extension(path);
    let mut text = format!("{} · 只读预览\n\n", ext.to_uppercase());
    match ext.as_str() {
        "zip" | "epub" | "odt" | "ods" | "odp" => {
            let mut archive = zip::ZipArchive::new(Cursor::new(bytes))
                .context("Invalid or truncated ZIP container")?;
            ensure!(archive.len() <= 10_000, "Archive exceeds 10,000 entries");
            let mut total = 0;
            for i in 0..archive.len() {
                ensure!(text.len() as u64 <= LIMIT, "Archive listing exceeds 32 MiB");
                let entry = archive.by_index(i)?;
                let safe = entry.enclosed_name().is_some();
                writeln!(
                    text,
                    "{} · {} bytes{}",
                    entry.name(),
                    entry.size(),
                    if safe {
                        ""
                    } else {
                        " · 路径越界，禁止提取"
                    }
                )?;
                let document =
                    matches!(ext.as_str(), "odt" | "ods" | "odp") && entry.name() == "content.xml";
                let chapter = ext == "epub" && entry.name().ends_with(".xhtml");
                if document || chapter {
                    let content = bounded(entry)?;
                    total += content.len();
                    ensure!(total as u64 <= LIMIT, "Document content exceeds 32 MiB");
                    text.push_str(&xml_text(&content)?);
                    text.push('\n');
                }
            }
        }
        "tar" => {
            for (i, entry) in tar::Archive::new(Cursor::new(bytes)).entries()?.enumerate() {
                ensure!(i < 10_000, "Archive exceeds 10,000 entries");
                ensure!(text.len() as u64 <= LIMIT, "Archive listing exceeds 32 MiB");
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
                        ""
                    } else {
                        " · 路径越界，禁止提取"
                    }
                )?;
            }
        }
        "gz" | "bz2" | "xz" => {
            let cursor = Cursor::new(bytes);
            let content = match ext.as_str() {
                "gz" => bounded(flate2::read::MultiGzDecoder::new(cursor))?,
                "bz2" => bounded(bzip2::read::MultiBzDecoder::new(cursor))?,
                _ => bounded(xz2::read::XzDecoder::new_multi_decoder(cursor))?,
            };
            text.push_str(
                std::str::from_utf8(&content).context("Decompressed content is not UTF-8")?,
            );
        }
        "npy" => text.push_str(&numpy(&bytes)?),
        _ => anyhow::bail!("Unsupported container"),
    }
    ensure!(
        text.len() as u64 <= LIMIT,
        "Rendered preview exceeds 32 MiB"
    );
    Ok(BytePreview {
        path: std::path::absolute(path)?,
        text,
    })
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
fn numpy(bytes: &[u8]) -> Result<String> {
    ensure!(bytes.starts_with(b"\x93NUMPY"), "Invalid NumPy signature");
    let version = *bytes.get(6).context("Truncated NumPy header")?;
    let (start, length) = match version {
        1 => (
            10,
            u16::from_le_bytes(bytes.get(8..10).context("Truncated header")?.try_into()?) as usize,
        ),
        2 | 3 => (
            12,
            u32::from_le_bytes(bytes.get(8..12).context("Truncated header")?.try_into()?) as usize,
        ),
        _ => anyhow::bail!("Unsupported NumPy version"),
    };
    ensure!(length <= 65536, "NumPy header exceeds 64 KiB");
    let header = std::str::from_utf8(
        bytes
            .get(start..start + length)
            .context("Truncated NumPy header")?,
    )?;
    ensure!(
        !header.contains("'O") && !header.contains("\"O"),
        "Object arrays are unsupported; pickle is never executed"
    );
    let mut text = format!("NumPy v{version}\n{header}\n");
    let data = &bytes[start + length..];
    let descriptor = ["<f8", "<f4", "<i8", "<i4", "|u1"]
        .into_iter()
        .find(|s| header.contains(s));
    let size = match descriptor {
        Some("<f8" | "<i8") => 8,
        Some("<f4" | "<i4") => 4,
        Some("|u1") => 1,
        _ => 0,
    };
    if size == 0 {
        writeln!(text, "数据类型暂无数值预览 · {} bytes", data.len())?;
    } else {
        let shape = header
            .split("shape")
            .nth(1)
            .context("Missing NumPy shape")?;
        let shape = shape.split('(').nth(1).context("Invalid NumPy shape")?;
        let shape = shape.split(')').next().context("Invalid NumPy shape")?;
        let mut count = 1usize;
        for dimension in shape.split(',').map(str::trim).filter(|v| !v.is_empty()) {
            count = count
                .checked_mul(dimension.parse::<usize>()?)
                .context("NumPy dimensions overflow")?;
        }
        ensure!(
            count.checked_mul(size) == Some(data.len()),
            "NumPy data length does not match shape"
        );
        for (index, chunk) in data.chunks_exact(size).take(10_000).enumerate() {
            let value = match descriptor {
                Some("<f8") => f64::from_le_bytes(chunk.try_into()?).to_string(),
                Some("<f4") => f32::from_le_bytes(chunk.try_into()?).to_string(),
                Some("<i8") => i64::from_le_bytes(chunk.try_into()?).to_string(),
                Some("<i4") => i32::from_le_bytes(chunk.try_into()?).to_string(),
                _ => chunk[0].to_string(),
            };
            writeln!(text, "[{index}] {value}")?;
        }
        if data.len() / size > 10_000 {
            text.push_str("仅显示前 10,000 个值\n");
        }
    }
    Ok(text)
}
