//! Bounded image decoding, kept off the UI executor.
use anyhow::{Context, Result, ensure};
use image::{AnimationDecoder, ImageDecoder};
use std::{
    io::{Cursor, Read},
    path::{Path, PathBuf},
};

pub struct ImageDocument {
    pub path: PathBuf,
    pub png: Vec<u8>,
    pub fingerprint: String,
    pub gif: bool,
    pub pages: Vec<Vec<u8>>,
}
pub fn supports(path: &Path) -> bool {
    matches!(
        path.extension()
            .and_then(|s| s.to_str())
            .unwrap_or_default()
            .to_ascii_lowercase()
            .as_str(),
        "png"
            | "jpg"
            | "jpeg"
            | "webp"
            | "gif"
            | "bmp"
            | "tif"
            | "tiff"
            | "ico"
            | "icns"
            | "svg"
            | "exr"
            | "hdr"
    )
}
pub fn read_image(path: &Path) -> Result<ImageDocument> {
    use std::hash::{Hash, Hasher};
    let mut bytes = Vec::new();
    std::fs::File::open(path)?
        .take(128 * 1024 * 1024 + 1)
        .read_to_end(&mut bytes)?;
    ensure!(
        bytes.len() <= 128 * 1024 * 1024,
        "Image exceeds 128 MiB limit"
    );
    let mut hash = std::collections::hash_map::DefaultHasher::new();
    bytes.hash(&mut hash);
    let fingerprint = format!("image:{:x}", hash.finish());
    let ext = path
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    let mut pages = Vec::new();
    let png = if ext == "gif" {
        let decoder = image::codecs::gif::GifDecoder::new(Cursor::new(&bytes))?;
        let (width, height) = decoder.dimensions();
        ensure!(
            u64::from(width) * u64::from(height) * 4 <= 512 * 1024 * 1024,
            "GIF frame exceeds 512 MiB decoded memory"
        );
        let mut decoded_bytes = 0_u64;
        for (index, frame) in decoder.into_frames().enumerate() {
            ensure!(index < 2000, "GIF exceeds 2000 frames");
            let frame = frame.context("GIF frame is damaged")?;
            decoded_bytes += u64::from(width) * u64::from(height) * 4;
            ensure!(
                decoded_bytes <= 512 * 1024 * 1024,
                "GIF animation exceeds 512 MiB decoded memory"
            );
            drop(frame);
        }
        ensure!(decoded_bytes > 0, "GIF contains no frames");
        bytes.clone()
    } else if matches!(ext.as_str(), "tif" | "tiff") {
        pages = tiff_pages(&bytes)?;
        pages[0].clone()
    } else if ext == "svg" {
        // No resources_dir: external file references are intentionally not resolved.
        let tree = resvg::usvg::Tree::from_data(&bytes, &resvg::usvg::Options::default())?;
        // Vector documents can describe enormous canvases. Rasterize a bounded
        // preview instead of rejecting them or allocating their full native size.
        let size = tree.size();
        let scale = (4000.0 / size.width().max(size.height())).min(1.0);
        let width = (size.width() * scale).ceil().clamp(1.0, 4000.0) as u32;
        let height = (size.height() * scale).ceil().clamp(1.0, 4000.0) as u32;
        let mut pixmap =
            resvg::tiny_skia::Pixmap::new(width, height).context("Invalid SVG dimensions")?;
        resvg::render(
            &tree,
            resvg::tiny_skia::Transform::from_scale(scale, scale),
            &mut pixmap.as_mut(),
        );
        pixmap.encode_png()?
    } else {
        let data = if ext == "icns" {
            icns_png(&bytes)?
        } else {
            &bytes
        };
        let mut reader = image::ImageReader::new(Cursor::new(data)).with_guessed_format()?;
        let mut limits = image::Limits::default();
        limits.max_image_width = None;
        limits.max_image_height = None;
        limits.max_alloc = Some(512 * 1024 * 1024);
        reader.limits(limits);
        let decoder = reader.into_decoder()?;
        let (width, height) = decoder.dimensions();
        // Check before allocating the full pixel buffer. Compressed file size
        // alone is not a reliable indication of an image's memory requirements.
        ensure!(
            u64::from(width) * u64::from(height) <= 100_000_000,
            "Image exceeds 100 megapixels"
        );
        ensure!(
            decoder.total_bytes() <= 512 * 1024 * 1024,
            "Decoded image exceeds 512 MiB"
        );
        let decoded = image::DynamicImage::from_decoder(decoder)?;
        let mut out = Cursor::new(Vec::new());
        // Float EXR/HDR pixels must be converted before PNG encoding. This is an
        // SDR preview, not a color-managed HDR mastering surface.
        let preview = if matches!(
            decoded,
            image::DynamicImage::ImageRgb32F(_) | image::DynamicImage::ImageRgba32F(_)
        ) {
            image::DynamicImage::ImageRgba8(decoded.to_rgba8())
        } else {
            decoded
        };
        let preview = if preview.width() > 4000 || preview.height() > 4000 {
            preview.thumbnail(4000, 4000)
        } else {
            preview
        };
        preview.write_to(&mut out, image::ImageFormat::Png)?;
        out.into_inner()
    };
    Ok(ImageDocument {
        path: std::path::absolute(path)?,
        png,
        fingerprint,
        gif: ext == "gif",
        pages,
    })
}

fn tiff_pages(bytes: &[u8]) -> Result<Vec<Vec<u8>>> {
    use tiff::{
        ColorType,
        decoder::{Decoder, DecodingResult},
    };
    let mut limits = tiff::decoder::Limits::default();
    limits.decoding_buffer_size = 512 * 1024 * 1024;
    let mut decoder = Decoder::new(Cursor::new(bytes))
        .context("TIFF header is damaged")?
        .with_limits(limits);
    let mut pages = Vec::new();
    let mut budget = 0_u64;
    let mut pixel_budget = 0_u64;
    loop {
        ensure!(pages.len() < 500, "TIFF exceeds 500 pages");
        let (width, height) = decoder.dimensions()?;
        ensure!(
            u64::from(width) * u64::from(height) <= 100_000_000,
            "TIFF page exceeds 100 megapixels"
        );
        let color = decoder.colortype()?;
        let pixels = match decoder
            .read_image()
            .context("TIFF page is damaged or its encoding is unsupported")?
        {
            DecodingResult::U8(data) => data,
            DecodingResult::U16(data) => data.into_iter().map(|v| (v >> 8) as u8).collect(),
            _ => anyhow::bail!("TIFF page uses unsupported pixel sample type"),
        };
        let decoded = match color {
            ColorType::RGB(8 | 16) => image::DynamicImage::ImageRgb8(
                image::RgbImage::from_raw(width, height, pixels)
                    .context("Invalid TIFF RGB pixels")?,
            ),
            ColorType::RGBA(8 | 16) => image::DynamicImage::ImageRgba8(
                image::RgbaImage::from_raw(width, height, pixels)
                    .context("Invalid TIFF RGBA pixels")?,
            ),
            ColorType::Gray(8 | 16) => image::DynamicImage::ImageLuma8(
                image::GrayImage::from_raw(width, height, pixels)
                    .context("Invalid TIFF grayscale pixels")?,
            ),
            ColorType::GrayA(8 | 16) => image::DynamicImage::ImageLumaA8(
                image::GrayAlphaImage::from_raw(width, height, pixels)
                    .context("Invalid TIFF grayscale alpha pixels")?,
            ),
            _ => anyhow::bail!("TIFF page color encoding is unsupported: {color:?}"),
        };
        let mut out = Cursor::new(Vec::new());
        let preview = if decoded.width() > 4000 || decoded.height() > 4000 {
            decoded.thumbnail(4000, 4000)
        } else {
            decoded
        };
        pixel_budget += u64::from(preview.width()) * u64::from(preview.height()) * 4;
        ensure!(
            pixel_budget <= 512 * 1024 * 1024,
            "TIFF page previews exceed 512 MiB decoded memory"
        );
        preview.write_to(&mut out, image::ImageFormat::Png)?;
        let png = out.into_inner();
        budget += png.len() as u64;
        ensure!(budget <= 256 * 1024 * 1024, "TIFF previews exceed 256 MiB");
        pages.push(png);
        if !decoder.more_images() {
            break;
        }
        decoder.next_image()?;
    }
    Ok(pages)
}
// Modern macOS ICNS contains PNG representations; choose the largest representation.
fn icns_png(bytes: &[u8]) -> Result<&[u8]> {
    ensure!(
        bytes.starts_with(b"icns") && bytes.len() >= 8,
        "Invalid ICNS file"
    );
    let mut offset = 8;
    let mut best: Option<&[u8]> = None;
    while offset + 8 <= bytes.len() {
        let len = u32::from_be_bytes(bytes[offset + 4..offset + 8].try_into()?) as usize;
        ensure!(
            len >= 8 && len <= bytes.len() - offset,
            "Invalid ICNS entry"
        );
        let data = &bytes[offset + 8..offset + len];
        if data.starts_with(b"\x89PNG\r\n\x1a\n") && best.is_none_or(|b| data.len() > b.len()) {
            best = Some(data);
        }
        offset += len;
    }
    best.context("This legacy ICNS has no PNG representation")
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn float_images_produce_displayable_png_previews() -> Result<()> {
        let dir = tempfile::tempdir()?;
        let pixels = image::Rgb32FImage::from_pixel(2, 2, image::Rgb([0.25, 0.5, 1.0]));
        for (extension, format) in [
            ("exr", image::ImageFormat::OpenExr),
            ("hdr", image::ImageFormat::Hdr),
        ] {
            let path = dir.path().join(format!("preview.{extension}"));
            image::DynamicImage::ImageRgb32F(pixels.clone()).save_with_format(&path, format)?;
            assert!(supports(&path));
            let preview = read_image(&path)?;
            let decoded = image::load_from_memory(&preview.png)?;
            assert_eq!((decoded.width(), decoded.height()), (2, 2));
            assert_eq!(decoded.to_rgb8().get_pixel(0, 0).0, [64, 128, 255]);
            std::fs::write(&path, b"invalid image")?;
            assert!(read_image(&path).is_err());
        }
        Ok(())
    }

    #[test]
    fn opens_project_assets_and_rejects_malformed_images() -> Result<()> {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        for p in [
            "assets/branding/glim.png",
            "assets/branding/glim.svg",
            "assets/macos/Glim.icns",
        ] {
            assert!(read_image(&root.join(p))?.png.starts_with(b"\x89PNG"));
        }
        assert!(icns_png(b"icns\0\0\0\x10bad!\0\0\0\0").is_err());
        let temp = tempfile::tempdir()?;
        let path = temp.path().join("large.svg");
        std::fs::write(
            &path,
            r#"<svg xmlns="http://www.w3.org/2000/svg" width="100000" height="50000"><rect width="100000" height="50000" fill="red"/></svg>"#,
        )?;
        let preview = image::load_from_memory(&read_image(&path)?.png)?;
        assert_eq!((preview.width(), preview.height()), (4000, 2000));
        assert_eq!(preview.to_rgb8().get_pixel(2000, 1000).0, [255, 0, 0]);
        Ok(())
    }
}
