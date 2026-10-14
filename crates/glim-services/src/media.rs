//! Bounded image decoding, kept off the UI executor.
use anyhow::{Context, Result, ensure};
use image::ImageDecoder;
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
        "qoi"
            | "png"
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
        // Check the header and canvas budget without decoding the animation.
        GifPlayback::new(std::sync::Arc::from(bytes.clone()))?;
        bytes
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
        if ext == "png" && (width > 4000 || height > 4000) {
            // ImageIO handles large PNG resizing/encoding without slow debug-build
            // pixel loops. Read from the already captured snapshot, not the source
            // path, so concurrent source changes cannot mix preview and fingerprint.
            use std::io::Write;
            let mut snapshot = tempfile::NamedTempFile::new()?;
            snapshot.write_all(data)?;
            drop(decoder);
            crate::inline_images::thumbnail_png(snapshot.path(), 4000)?
        } else {
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
        }
    };
    Ok(ImageDocument {
        path: std::path::absolute(path)?,
        png,
        fingerprint,
        gif: ext == "gif",
        pages,
    })
}

/// Sequential, original-resolution GIF decoding with bounded compositing state.
/// The UI decodes small batches on its background executor. Display previews
/// are bounded independently of the original-resolution compositing canvas.
pub struct GifPlayback {
    bytes: std::sync::Arc<[u8]>,
    decoder: gif::Decoder<Cursor<std::sync::Arc<[u8]>>>,
    canvas: image::RgbaImage,
    restore: Option<image::RgbaImage>,
    disposal: gif::DisposalMethod,
    rectangle: (u32, u32, u32, u32),
    frames: usize,
    loops: u32,
    finished: bool,
}
impl GifPlayback {
    pub fn new(bytes: std::sync::Arc<[u8]>) -> Result<Self> {
        let decoder = Self::decoder(bytes.clone())?;
        let (width, height) = (u32::from(decoder.width()), u32::from(decoder.height()));
        // Allow room for canvas, restore, decoder, displayed frame and next frame.
        ensure!(
            u64::from(width) * u64::from(height) * 4 * 5 <= 512 * 1024 * 1024,
            "GIF canvas exceeds 512 MiB playback memory budget"
        );
        ensure!(width > 0 && height > 0, "GIF has an empty canvas");
        Ok(Self {
            bytes,
            decoder,
            canvas: image::RgbaImage::new(width, height),
            restore: None,
            disposal: gif::DisposalMethod::Keep,
            rectangle: (0, 0, 0, 0),
            frames: 0,
            loops: 0,
            finished: false,
        })
    }

    fn decoder(bytes: std::sync::Arc<[u8]>) -> Result<gif::Decoder<Cursor<std::sync::Arc<[u8]>>>> {
        let mut options = gif::DecodeOptions::new();
        options.set_color_output(gif::ColorOutput::RGBA);
        options.check_frame_consistency(true);
        options.set_memory_limit(gif::MemoryLimit::Bytes(
            std::num::NonZeroU64::new(512 * 1024 * 1024 / 5).unwrap(),
        ));
        Ok(options.read_info(Cursor::new(bytes))?)
    }

    pub fn preview_frame_bytes(&self) -> usize {
        let (width, height) = self.preview_dimensions();
        width as usize * height as usize * 4
    }

    fn preview_dimensions(&self) -> (u32, u32) {
        let (width, height) = self.canvas.dimensions();
        let edge = width.max(height);
        if edge <= 2048 {
            (width, height)
        } else {
            (
                ((u64::from(width) * 2048 / u64::from(edge)) as u32).max(1),
                ((u64::from(height) * 2048 / u64::from(edge)) as u32).max(1),
            )
        }
    }

    pub fn next_frame(&mut self) -> Result<Option<(image::Frame, std::time::Duration)>> {
        self.decode_frame(false)
    }

    pub fn next_preview_frame(&mut self) -> Result<Option<(image::Frame, std::time::Duration)>> {
        self.decode_frame(true)
    }

    fn decode_frame(
        &mut self,
        preview: bool,
    ) -> Result<Option<(image::Frame, std::time::Duration)>> {
        if self.finished {
            return Ok(None);
        }
        match self.disposal {
            gif::DisposalMethod::Background => {
                let (left, top, width, height) = self.rectangle;
                let stride = self.canvas.width() as usize * 4;
                let pixels: &mut [u8] = self.canvas.as_mut();
                for y in top..top + height {
                    let start = y as usize * stride + left as usize * 4;
                    pixels[start..start + width as usize * 4].fill(0);
                }
            }
            gif::DisposalMethod::Previous => {
                if let Some(restore) = self.restore.take() {
                    self.canvas = restore;
                }
            }
            _ => {}
        }
        if self.decoder.read_next_frame()?.is_none() {
            ensure!(self.frames > 0, "GIF contains no frames");
            self.loops = self.loops.saturating_add(1);
            if let gif::Repeat::Finite(repeats) = self.decoder.repeat()
                && self.loops > u32::from(repeats)
            {
                self.finished = true;
                return Ok(None);
            }
            self.decoder = Self::decoder(self.bytes.clone())?;
            self.canvas.fill(0);
            self.restore = None;
            self.frames = 0;
            ensure!(
                self.decoder.read_next_frame()?.is_some(),
                "GIF contains no frames"
            );
        }
        let frame = self
            .decoder
            .current_frame_info()
            .context("GIF frame is missing")?;
        self.frames += 1;
        let left = u32::from(frame.left);
        let top = u32::from(frame.top);
        let width = u32::from(frame.width);
        let height = u32::from(frame.height);
        self.restore =
            (frame.dispose == gif::DisposalMethod::Previous).then(|| self.canvas.clone());
        let stride = self.canvas.width() as usize * 4;
        let row_bytes = width as usize * 4;
        let canvas: &mut [u8] = self.canvas.as_mut();
        if row_bytes > 0 {
            for (y, source) in frame
                .buffer
                .chunks_exact(row_bytes.max(1))
                .take(height as usize)
                .enumerate()
            {
                let start = (top as usize + y) * stride + left as usize * 4;
                let destination = &mut canvas[start..start + row_bytes];
                if frame.transparent.is_none() {
                    destination.copy_from_slice(source);
                } else {
                    for (source, destination) in source
                        .as_chunks::<4>()
                        .0
                        .iter()
                        .zip(destination.as_chunks_mut::<4>().0.iter_mut())
                    {
                        if source[3] != 0 {
                            destination.copy_from_slice(source);
                        }
                    }
                }
            }
        }
        self.disposal = frame.dispose;
        self.rectangle = (left, top, width, height);
        // Match browser handling of zero/one-centisecond delays to avoid busy loops.
        let delay_ms = if frame.delay <= 1 {
            100
        } else {
            u32::from(frame.delay) * 10
        };
        Ok(Some((
            image::Frame::new(
                if preview && self.canvas.width().max(self.canvas.height()) > 2048 {
                    let (width, height) = self.preview_dimensions();
                    image::imageops::resize(
                        &self.canvas,
                        width,
                        height,
                        image::imageops::FilterType::Nearest,
                    )
                } else {
                    self.canvas.clone()
                },
            ),
            std::time::Duration::from_millis(u64::from(delay_ms)),
        )))
    }
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
    fn gif_bytes(
        frames: &[(u16, Vec<u8>, gif::DisposalMethod)],
        repeat: gif::Repeat,
    ) -> Result<Vec<u8>> {
        let mut bytes = Vec::new();
        {
            let mut encoder = gif::Encoder::new(
                &mut bytes,
                3,
                1,
                &[255, 0, 0, 0, 255, 0, 0, 0, 255, 0, 0, 0],
            )?;
            encoder.set_repeat(repeat)?;
            for (left, pixels, disposal) in frames {
                let frame = gif::Frame {
                    left: *left,
                    width: pixels.len() as u16,
                    height: 1,
                    delay: 5,
                    dispose: *disposal,
                    transparent: Some(3),
                    buffer: std::borrow::Cow::Borrowed(pixels),
                    ..Default::default()
                };
                encoder.write_frame(&frame)?;
            }
        }
        Ok(bytes)
    }

    #[test]
    fn gif_display_preview_is_bounded_without_changing_source_frames() -> Result<()> {
        let mut bytes = Vec::new();
        {
            let mut encoder = gif::Encoder::new(&mut bytes, 4096, 1, &[255, 0, 0])?;
            encoder.set_repeat(gif::Repeat::Finite(0))?;
            encoder.write_frame(&gif::Frame {
                width: 4096,
                height: 1,
                delay: 5,
                buffer: std::borrow::Cow::Owned(vec![0; 4096]),
                ..Default::default()
            })?;
        }
        let mut source = GifPlayback::new(std::sync::Arc::from(bytes.clone()))?;
        let mut preview = GifPlayback::new(std::sync::Arc::from(bytes))?;
        assert_eq!(
            source.next_frame()?.unwrap().0.buffer().dimensions(),
            (4096, 1)
        );
        assert_eq!(preview.preview_frame_bytes(), 2048 * 4);
        let (frame, delay) = preview.next_preview_frame()?.unwrap();
        assert_eq!(frame.buffer().dimensions(), (2048, 1));
        assert!(
            frame
                .buffer()
                .pixels()
                .all(|pixel| pixel.0 == [255, 0, 0, 255])
        );
        assert_eq!(delay, std::time::Duration::from_millis(50));
        assert!(preview.next_preview_frame()?.is_none());
        Ok(())
    }

    #[test]
    fn gif_playback_composites_disposal_and_respects_finite_loops() -> Result<()> {
        use gif::DisposalMethod::{Background, Keep, Previous};
        let bytes = gif_bytes(
            &[
                (0, vec![0, 0, 0], Keep),
                (1, vec![1], Background),
                (0, vec![2], Previous),
                (2, vec![1], Keep),
            ],
            gif::Repeat::Finite(1),
        )?;
        let mut playback = GifPlayback::new(std::sync::Arc::from(bytes))?;
        let red = [255, 0, 0, 255];
        let green = [0, 255, 0, 255];
        let blue = [0, 0, 255, 255];
        let transparent = [0; 4];
        for _ in 0..2 {
            for expected in [
                [red, red, red],
                [red, green, red],
                [blue, transparent, red],
                [red, transparent, green],
            ] {
                let (frame, delay) = playback.next_frame()?.unwrap();
                assert_eq!(frame.buffer().dimensions(), (3, 1));
                assert_eq!(delay, std::time::Duration::from_millis(50));
                for (pixel, expected) in frame.buffer().pixels().zip(expected) {
                    assert_eq!(pixel.0, expected);
                }
            }
        }
        assert!(playback.next_frame()?.is_none());
        assert!(playback.next_frame()?.is_none());
        Ok(())
    }

    #[test]
    fn gif_playback_has_no_total_frame_limit() -> Result<()> {
        let frames = vec![(0, vec![0, 1, 2], gif::DisposalMethod::Keep); 2101];
        let bytes = gif_bytes(&frames, gif::Repeat::Infinite)?;
        let mut playback = GifPlayback::new(std::sync::Arc::from(bytes))?;
        for _ in 0..4202 {
            assert!(playback.next_frame()?.is_some());
            assert_eq!(playback.canvas.len(), 12);
            assert!(playback.restore.is_none());
        }
        Ok(())
    }

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
