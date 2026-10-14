//! Bounded raster previews for inline document images. Call from a background executor.
use anyhow::{Context, Result, ensure};
use std::{
    io::Write,
    path::Path,
    process::{Command, Stdio},
    sync::Mutex,
};

const MAX_EDGE: u32 = 2048;
// Avoid simultaneous full-size native decode work for a document containing many images.
static PREPARATION: Mutex<()> = Mutex::new(());

pub fn from_file(path: &Path) -> Result<image::Frame> {
    ensure!(
        std::fs::metadata(path)?.len() <= 128 * 1024 * 1024,
        "Inline image exceeds 128 MiB"
    );
    let _guard = PREPARATION
        .lock()
        .map_err(|_| anyhow::anyhow!("Image preparation was interrupted"))?;
    if path
        .extension()
        .is_some_and(|e| e.eq_ignore_ascii_case("svg"))
    {
        let document = crate::media::read_image(path)?;
        return decoded_preview(&document.png);
    }
    native_thumbnail(path)
}

pub fn from_bytes(bytes: &[u8]) -> Result<image::Frame> {
    ensure!(
        bytes.len() <= 20 * 1024 * 1024,
        "Inline image exceeds 20 MiB"
    );
    let mut file = tempfile::NamedTempFile::new()?;
    file.write_all(bytes)?;
    from_file(file.path())
}

fn native_thumbnail(path: &Path) -> Result<image::Frame> {
    if let Ok((width, height)) = image::ImageReader::open(path)?
        .with_guessed_format()?
        .into_dimensions()
    {
        ensure!(
            width > 0 && height > 0 && u64::from(width) * u64::from(height) <= 100_000_000,
            "Inline image exceeds 100 megapixels"
        );
    }
    let directory = tempfile::tempdir()?;
    let output = directory.path().join("preview.png");
    // macOS ImageIO prepares the thumbnail outside the viewer's heap. The source is untouched.
    let result = Command::new("/usr/bin/sips")
        .args([
            "--resampleHeightWidthMax",
            &MAX_EDGE.to_string(),
            "--setProperty",
            "format",
            "png",
            "--out",
        ])
        .arg(&output)
        .arg(std::path::absolute(path)?)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .output()
        .context("Cannot prepare inline image thumbnail")?;
    ensure!(
        result.status.success(),
        "Cannot prepare inline image thumbnail: {}",
        String::from_utf8_lossy(&result.stderr).trim()
    );
    ensure!(
        std::fs::metadata(&output)?.len() <= 20 * 1024 * 1024,
        "Inline thumbnail exceeds 20 MiB"
    );
    let mut reader = image::ImageReader::open(output)?;
    let mut limits = image::Limits::default();
    limits.max_image_width = Some(MAX_EDGE);
    limits.max_image_height = Some(MAX_EDGE);
    limits.max_alloc = Some(64 * 1024 * 1024);
    reader.limits(limits);
    Ok(image::Frame::new(reader.decode()?.into_rgba8()))
}

fn decoded_preview(bytes: &[u8]) -> Result<image::Frame> {
    let image = image::load_from_memory(bytes)?;
    let image = if image.width() > MAX_EDGE || image.height() > MAX_EDGE {
        image.thumbnail(MAX_EDGE, MAX_EDGE)
    } else {
        image
    };
    Ok(image::Frame::new(image.into_rgba8()))
}
