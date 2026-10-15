//! Streaming static SVG preparation for the system thumbnail provider.
use crate::{limits, work::Cancellation};
use anyhow::{Result, ensure};
use quick_xml::{
    Reader, Writer,
    events::{BytesStart, Event},
};
use std::{
    fs::File,
    io::{BufReader, BufWriter, Write},
    path::Path,
};

fn local_css(value: &str) -> bool {
    let lower = value.to_ascii_lowercase();
    if lower.contains(['\\', '@'])
        || lower.contains("expression")
        || lower.contains("image-set")
        || lower.contains("/*")
    {
        return false;
    }
    let mut rest = lower.as_str();
    while let Some(start) = rest.find("url") {
        rest = rest[start + 3..].trim_start();
        let Some(argument) = rest.strip_prefix('(') else {
            return false;
        };
        let Some(end) = argument.find(')') else {
            return false;
        };
        let id = argument[..end].trim().trim_matches(['\'', '"']);
        if !id.starts_with('#')
            || !id[1..]
                .chars()
                .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-' | ':' | '.'))
        {
            return false;
        }
        rest = &argument[end + 1..];
    }
    true
}
fn embedded(value: &str, cancel: &Cancellation) -> Result<Option<String>> {
    use base64::Engine;
    let Some((header, bytes)) = value.split_once(",") else {
        return Ok(None);
    };
    if !header.ends_with(";base64") || !header.starts_with("data:image/") {
        return Ok(None);
    }
    let bytes = base64::engine::general_purpose::STANDARD.decode(bytes)?;
    if image::guess_format(&bytes).is_err() {
        return Ok(None);
    }
    let mut file = tempfile::NamedTempFile::new()?;
    file.write_all(&bytes)?;
    drop(bytes);
    let png = crate::inline_images::thumbnail_png_cancellable(
        file.path(),
        limits::INLINE_IMAGE_PREVIEW_EDGE.min(
            ((limits::REMOTE_IMAGE_SVG_EMBEDDED_BYTES / 8) as f64)
                .sqrt()
                .max(1.) as u32,
        ),
        cancel,
    )?;
    Ok(Some(format!(
        "data:image/png;base64,{}",
        base64::engine::general_purpose::STANDARD.encode(png)
    )))
}
pub(crate) fn prepare(path: &Path, cancel: &Cancellation) -> Result<tempfile::NamedTempFile> {
    let input = crate::work::RecordReader::new(File::open(path)?, limits::SVG_EVENT_BYTES, cancel);
    let mut reader = Reader::from_reader(BufReader::with_capacity(limits::IO_BUFFER_BYTES, input));
    let mut output = tempfile::Builder::new().suffix(".svg").tempfile()?;
    let mut writer = Writer::new(BufWriter::new(output.as_file_mut()));
    let mut buffer = Vec::new();
    let mut skipped = 0usize;
    let mut root = false;
    let mut style = false;
    loop {
        cancel.check()?;
        let event = reader.read_event_into(&mut buffer)?;
        reader.get_mut().get_mut().reset(limits::SVG_EVENT_BYTES);
        match event {
            event @ (Event::Start(_) | Event::Empty(_)) => {
                let empty = matches!(event, Event::Empty(_));
                let start = match event {
                    Event::Start(start) | Event::Empty(start) => start,
                    _ => unreachable!(),
                };
                let name = String::from_utf8(start.name().as_ref().to_vec())?;
                let local = name
                    .rsplit(':')
                    .next()
                    .unwrap_or(&name)
                    .to_ascii_lowercase();
                if !root {
                    ensure!(local == "svg", "Not an SVG image");
                    root = true;
                }
                if skipped > 0
                    || matches!(
                        local.as_str(),
                        "script"
                            | "foreignobject"
                            | "iframe"
                            | "object"
                            | "embed"
                            | "animate"
                            | "animatemotion"
                            | "animatetransform"
                            | "set"
                            | "discard"
                    )
                {
                    if !empty {
                        skipped += 1;
                    }
                } else {
                    let mut cleaned = BytesStart::new(name.as_str());
                    for attribute in start.attributes() {
                        let attribute = attribute?;
                        let key = std::str::from_utf8(attribute.key.as_ref())?;
                        let local_key = key.rsplit(':').next().unwrap_or(key).to_ascii_lowercase();
                        let value = attribute
                            .decode_and_unescape_value(reader.decoder())?
                            .into_owned();
                        if local_key.starts_with("on")
                            || matches!(local_key.as_str(), "src" | "base")
                        {
                            continue;
                        }
                        if local_key == "href" {
                            if value.starts_with('#') {
                                cleaned.push_attribute((key, value.as_str()));
                            } else if let Some(value) = embedded(&value, cancel).ok().flatten() {
                                cleaned.push_attribute((key, value.as_str()));
                            }
                        } else if local_key == "style" || value.to_ascii_lowercase().contains("url")
                        {
                            if local_css(&value) {
                                cleaned.push_attribute((key, value.as_str()));
                            }
                        } else {
                            cleaned.push_attribute((key, value.as_str()));
                        }
                    }
                    style = local == "style" && !empty;
                    writer.write_event(if empty {
                        Event::Empty(cleaned)
                    } else {
                        Event::Start(cleaned)
                    })?;
                }
            }
            Event::End(end) => {
                if skipped > 0 {
                    skipped -= 1;
                } else {
                    style = false;
                    writer.write_event(Event::End(end))?;
                }
            }
            Event::Text(text) if skipped == 0 => {
                if !style || local_css(&text.decode()?) {
                    writer.write_event(Event::Text(text))?;
                }
            }
            Event::CData(text) if skipped == 0 => {
                if !style || local_css(&text.decode()?) {
                    writer.write_event(Event::CData(text))?;
                }
            }
            Event::GeneralRef(reference) if skipped == 0 && !style => {
                writer.write_event(Event::GeneralRef(reference))?;
            }
            Event::DocType(_) => {
                anyhow::bail!("SVG external entities and document types are unsupported")
            }
            Event::Eof => break,
            _ => {}
        }
        buffer.clear();
    }
    ensure!(root && skipped == 0, "Incomplete SVG document");
    writer.into_inner().flush()?;
    Ok(output)
}
