//! HTML picture selection and bounded remote image preparation, independent of UI.
use anyhow::{Context, Result, ensure};
use html5ever::{parse_document, tendril::TendrilSink};
use markup5ever_rcdom::{Handle, NodeData, RcDom};
use std::{
    collections::{HashMap, HashSet},
    io::{Read, Write},
    process::{Command, Stdio},
    sync::{Arc, OnceLock},
};
const LIMIT: usize = 16 * 1024 * 1024;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PictureSources {
    pub light: Option<String>,
    pub dark: Option<String>,
}
pub fn picture_sources(text: &str, html: bool) -> HashMap<String, PictureSources> {
    if text.len() > 8 * 1024 * 1024 || !text.to_ascii_lowercase().contains("<picture") {
        return HashMap::new();
    }
    let mut fragments = Vec::new();
    if html {
        fragments.push(text.to_owned());
    } else if let Ok(tree) = markdown::to_mdast(text, &markdown::ParseOptions::gfm()) {
        fn collect(node: &markdown::mdast::Node, result: &mut Vec<String>) {
            if let markdown::mdast::Node::Html(html) = node {
                result.push(html.value.clone());
            }
            if let Some(children) = node.children() {
                for child in children {
                    collect(child, result);
                }
            }
        }
        collect(&tree, &mut fragments);
    }
    let mut result = HashMap::new();
    let mut ambiguous = HashSet::new();
    for fragment in fragments {
        if !fragment.to_ascii_lowercase().contains("<picture") {
            continue;
        }
        let dom = parse_document(RcDom::default(), Default::default()).one(fragment);
        // Keep the DOM root alive: its iterative destructor clears descendant children.
        let mut pending = vec![dom.document.clone()];
        while let Some(node) = pending.pop() {
            pending.extend(node.children.borrow().iter().cloned());
            if !is_tag(&node, "picture") {
                continue;
            }
            let children = node.children.borrow();
            let Some(fallback) = children
                .iter()
                .find(|n| is_tag(n, "img"))
                .and_then(|n| attr(n, "src"))
            else {
                continue;
            };
            let mut variants = PictureSources::default();
            for child in children.iter().filter(|n| is_tag(n, "source")) {
                let media = attr(child, "media")
                    .unwrap_or_default()
                    .chars()
                    .filter(|c| !c.is_whitespace())
                    .collect::<String>()
                    .to_ascii_lowercase();
                let Some(srcset) = attr(child, "srcset") else {
                    continue;
                };
                let candidate = if srcset.starts_with("data:") {
                    srcset.split_whitespace().next()
                } else {
                    srcset
                        .split(',')
                        .next()
                        .and_then(|v| v.split_whitespace().next())
                };
                let Some(candidate) = candidate.filter(|s| !s.is_empty()) else {
                    continue;
                };
                if (media.is_empty() || media == "(prefers-color-scheme:light)")
                    && variants.light.is_none()
                {
                    variants.light = Some(candidate.to_owned());
                }
                if (media.is_empty() || media == "(prefers-color-scheme:dark)")
                    && variants.dark.is_none()
                {
                    variants.dark = Some(candidate.to_owned());
                }
            }
            if ambiguous.contains(&fallback) {
                continue;
            }
            if result.get(&fallback).is_some_and(|old| old != &variants) {
                result.remove(&fallback);
                ambiguous.insert(fallback);
            } else {
                result.insert(fallback, variants);
            }
        }
    }
    result
}
fn is_tag(node: &Handle, tag: &str) -> bool {
    matches!(&node.data, NodeData::Element { name, .. } if name.local.as_ref() == tag)
}
fn attr(node: &Handle, key: &str) -> Option<String> {
    if let NodeData::Element { attrs, .. } = &node.data {
        attrs
            .borrow()
            .iter()
            .find(|a| a.name.local.as_ref() == key)
            .map(|a| a.value.to_string())
    } else {
        None
    }
}

// GitHub's Markdown renderer rewrites repository-page image links to raw assets.
// Match that behavior for images only; ordinary links retain their destinations.
fn download_url(value: &str) -> Result<url::Url> {
    let mut url = url::Url::parse(value)?;
    ensure!(
        matches!(url.scheme(), "http" | "https")
            && url.username().is_empty()
            && url.password().is_none(),
        "Unsupported image URL"
    );
    if url.host_str() == Some("github.com") && url.port().is_none() {
        let mut parts = url.path().trim_start_matches('/').splitn(4, '/');
        if let (Some(owner), Some(repo), Some(kind), Some(asset)) =
            (parts.next(), parts.next(), parts.next(), parts.next())
            && matches!(kind, "blob" | "raw")
            && !owner.is_empty()
            && !repo.is_empty()
            && asset
                .split_once('/')
                .is_some_and(|(reference, path)| !reference.is_empty() && !path.is_empty())
        {
            let path = format!("/{owner}/{repo}/{asset}");
            url.set_host(Some("raw.githubusercontent.com"))?;
            url.set_path(&path);
            url.set_query(None);
            url.set_fragment(None);
        }
    }
    Ok(url)
}

/// Image references actually present in rendered Markdown or HTML.
/// Code blocks and examples do not initiate downloads.
pub fn image_uris(text: &str, html: bool) -> HashSet<String> {
    use markdown::mdast::Node;
    if text.len() > 8 * 1024 * 1024 {
        return HashSet::new();
    }
    let mut images = HashSet::new();
    let mut fragments = Vec::new();
    if html {
        fragments.push(text.to_owned());
    } else if let Ok(tree) = markdown::to_mdast(text, &markdown::ParseOptions::gfm()) {
        let mut definitions = HashMap::new();
        let mut references = Vec::new();
        let mut pending = vec![&tree];
        while let Some(node) = pending.pop() {
            match node {
                Node::Image(image) => {
                    images.insert(image.url.clone());
                }
                Node::ImageReference(image) => references.push(image.identifier.clone()),
                Node::Definition(definition) => {
                    definitions
                        .entry(definition.identifier.clone())
                        .or_insert_with(|| definition.url.clone());
                }
                Node::Html(html) => fragments.push(html.value.clone()),
                _ => {}
            }
            if let Some(children) = node.children() {
                pending.extend(children.iter().rev());
            }
        }
        for reference in references {
            if let Some(url) = definitions.get(&reference) {
                images.insert(url.clone());
            }
        }
    }
    for fragment in fragments {
        let dom = parse_document(RcDom::default(), Default::default()).one(fragment);
        let mut pending = vec![dom.document.clone()];
        while let Some(node) = pending.pop() {
            pending.extend(node.children.borrow().iter().cloned());
            if is_tag(&node, "img")
                && let Some(src) = attr(&node, "src")
            {
                images.insert(src);
            }
        }
    }
    images
}

fn github_api_url(url: &url::Url) -> Option<url::Url> {
    if url.host_str() != Some("raw.githubusercontent.com") || url.port().is_some() {
        return None;
    }
    let mut parts = url.path().trim_start_matches('/').splitn(4, '/');
    let (owner, repo, reference, path) =
        (parts.next()?, parts.next()?, parts.next()?, parts.next()?);
    if [owner, repo, reference, path]
        .iter()
        .any(|part| part.is_empty())
        || reference == "refs"
    {
        return None;
    }
    let mut api = url::Url::parse("https://api.github.com").ok()?;
    api.set_path(&format!("/repos/{owner}/{repo}/contents/{path}"));
    // The path reference is already percent-encoded. Preserve encoded slashes,
    // but escape literal '+' because it would mean a space in a query.
    api.set_query(Some(&format!("ref={}", reference.replace('+', "%2B"))));
    Some(api)
}

pub fn fetch_image(url: &str) -> Result<Vec<u8>> {
    prepare_image(&fetch_image_data(url)?)
}

pub fn fetch_inline_image(url: &str) -> Result<image::Frame> {
    let bytes = fetch_image_data(url)?;
    if image::guess_format(&bytes).is_ok() {
        crate::inline_images::from_bytes(&bytes)
    } else {
        crate::inline_images::from_bytes(&svg_png(&bytes)?)
    }
}

fn fetch_image_data(url: &str) -> Result<Vec<u8>> {
    use std::time::{Duration, Instant};
    let url = download_url(url)?;
    let fallback = github_api_url(&url);
    let started = Instant::now();
    let first_timeout = Duration::from_secs(if fallback.is_some() { 8 } else { 20 });
    let bytes = match download_image(&url, first_timeout, false) {
        Ok(bytes) => bytes,
        Err(error) => {
            let remaining = Duration::from_secs(20).saturating_sub(started.elapsed());
            let Some(fallback) = fallback.filter(|_| !remaining.is_zero()) else {
                return Err(error);
            };
            download_image(&fallback, remaining, true)
                .context("GitHub raw image and API fallback requests failed")?
        }
    };
    Ok(bytes)
}

fn download_image(url: &url::Url, timeout: std::time::Duration, api: bool) -> Result<Vec<u8>> {
    let mut command = Command::new("/usr/bin/curl");
    command
        .args([
            "--disable",
            "--fail",
            "--silent",
            "--location",
            "--max-redirs",
            "5",
            "--connect-timeout",
            "8",
            "--proto",
            "=http,https",
            "--proto-redir",
            "=http,https",
            "--user-agent",
            "Glim",
            "--url",
            url.as_str(),
        ])
        .arg("--max-time")
        .arg(timeout.as_secs_f64().to_string())
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null());

    if api {
        command.args(["--header", "Accept: application/vnd.github.raw+json"]);
    }
    let mut child = command
        .spawn()
        .context("Cannot start the image downloader")?;
    let mut bytes = Vec::new();
    let read = child
        .stdout
        .take()
        .context("Image downloader has no output")?
        .take(LIMIT as u64 + 1)
        .read_to_end(&mut bytes);
    if read.is_err() || bytes.len() > LIMIT {
        let _ = child.kill();
        let _ = child.wait();
        read?;
        anyhow::bail!("Remote image exceeds 16 MiB");
    }
    ensure!(child.wait()?.success(), "Remote image request failed");
    ensure!(!bytes.is_empty(), "Remote image is empty");
    Ok(bytes)
}
fn prepare_image(bytes: &[u8]) -> Result<Vec<u8>> {
    ensure!(bytes.len() <= LIMIT, "Remote image exceeds 16 MiB");
    let Ok(format) = image::guess_format(bytes) else {
        return svg_png(bytes);
    };
    let suffix = match format {
        image::ImageFormat::Gif => ".gif",
        image::ImageFormat::Tiff => ".tiff",
        _ => ".image",
    };
    let mut file = tempfile::Builder::new().suffix(suffix).tempfile()?;
    file.write_all(bytes)?;
    let image = crate::media::read_image(file.path())?;
    // GIF data remains animated; multi-page inputs use their first preview page.
    Ok(image.png)
}
fn svg_png(bytes: &[u8]) -> Result<Vec<u8>> {
    static FONTS: OnceLock<Arc<resvg::usvg::fontdb::Database>> = OnceLock::new();
    let fonts = FONTS.get_or_init(|| {
        let mut db = resvg::usvg::fontdb::Database::new();
        db.load_system_fonts();
        Arc::new(db)
    });
    let mut options = resvg::usvg::Options {
        fontdb: fonts.clone(),
        ..Default::default()
    };
    // Remote SVG may contain data images, but may not read local files or fetch references.
    options.image_href_resolver.resolve_string = Box::new(|_, _| None);
    let embedded_budget = std::sync::atomic::AtomicU64::new(0);
    let decode_data = resvg::usvg::ImageHrefResolver::default_data_resolver();
    options.image_href_resolver.resolve_data = Box::new(move |mime, data, options| {
        // Nested SVG and oversized embedded rasters must not bypass the canvas budget.
        if mime.contains("svg") || data.len() > LIMIT {
            return None;
        }
        let reader = image::ImageReader::new(std::io::Cursor::new(data.as_slice()))
            .with_guessed_format()
            .ok()?;
        let (width, height) = reader.into_dimensions().ok()?;
        let size = u64::from(width)
            .checked_mul(u64::from(height))?
            .checked_mul(4)?;
        embedded_budget
            .try_update(
                std::sync::atomic::Ordering::Relaxed,
                std::sync::atomic::Ordering::Relaxed,
                |old| {
                    old.checked_add(size)
                        .filter(|total| *total <= 64 * 1024 * 1024)
                },
            )
            .ok()?;
        decode_data(mime, data, options)
    });
    let tree = resvg::usvg::Tree::from_data(bytes, &options)?;
    let scale = (4000. / tree.size().width().max(tree.size().height())).min(1.);
    let width = (tree.size().width() * scale).ceil().clamp(1., 4000.) as u32;
    let height = (tree.size().height() * scale).ceil().clamp(1., 4000.) as u32;
    let mut pixmap =
        resvg::tiny_skia::Pixmap::new(width, height).context("Invalid remote SVG dimensions")?;
    resvg::render(
        &tree,
        resvg::tiny_skia::Transform::from_scale(scale, scale),
        &mut pixmap.as_mut(),
    );
    Ok(pixmap.encode_png()?)
}
pub fn unavailable_image() -> Result<Vec<u8>> {
    svg_png(br##"<svg xmlns="http://www.w3.org/2000/svg" width="200" height="64"><rect width="200" height="64" rx="6" fill="#f2f2f2"/><path d="M12 20L32 40M32 20L12 40" stroke="#b42318" stroke-width="3"/><text x="44" y="30" font-family="sans-serif" font-size="13" fill="#444">Image unavailable</text><text x="44" y="48" font-family="sans-serif" font-size="10" fill="#666">Check connection or URL</text></svg>"##)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn github_repository_image_urls_download_raw_assets() -> Result<()> {
        for kind in ["blob", "raw"] {
            let url = format!(
                "https://github.com/humanlayer/12-factor-agents/{kind}/main/img/027-agent-loop-animation.gif?raw=true#preview"
            );
            assert_eq!(
                download_url(&url)?.as_str(),
                "https://raw.githubusercontent.com/humanlayer/12-factor-agents/main/img/027-agent-loop-animation.gif"
            );
        }
        assert_eq!(
            download_url("https://github.com/o/r/blob/a123/img/hello%20world.png")?.as_str(),
            "https://raw.githubusercontent.com/o/r/a123/img/hello%20world.png"
        );
        for url in [
            "https://github.com/user-attachments/assets/example",
            "https://github.com/o/r/issues/1",
            "https://example.com/o/r/blob/main/image.png?token=value",
            "https://raw.githubusercontent.com/o/r/main/image.png",
        ] {
            assert_eq!(download_url(url)?.as_str(), url);
        }
        assert!(download_url("https://user:password@github.com/o/r/blob/main/a.png").is_err());
        Ok(())
    }

    #[test]
    fn github_raw_fallback_preserves_asset_paths_and_refs() -> Result<()> {
        let url = download_url("https://github.com/o/r/blob/feature%2Fbranch/img/a%20b.png")?;
        assert_eq!(
            github_api_url(&url).unwrap().as_str(),
            "https://api.github.com/repos/o/r/contents/img/a%20b.png?ref=feature%2Fbranch"
        );
        assert!(github_api_url(&url::Url::parse("https://example.com/o/r/main/a.png")?).is_none());
        assert!(
            github_api_url(&url::Url::parse(
                "https://raw.githubusercontent.com/o/r/refs/heads/main/a.png"
            )?)
            .is_none()
        );
        Ok(())
    }

    #[test]
    fn image_references_include_linked_gifs_html_and_reference_images() {
        let text = "[![loop](https://github.com/o/r/blob/main/loop.gif)](https://example.com)\n\n![reference][asset]\n\n[asset]: ../img/reference.gif\n\n<img src='html.gif'>\n\n```markdown\n![example](wrong.gif)\n```";
        let images = image_uris(text, false);
        assert_eq!(
            images,
            HashSet::from([
                "https://github.com/o/r/blob/main/loop.gif".to_owned(),
                "../img/reference.gif".to_owned(),
                "html.gif".to_owned(),
            ])
        );
        assert_eq!(
            image_uris("<picture><img src='fallback.gif'></picture>", true),
            HashSet::from(["fallback.gif".to_owned()])
        );
    }

    #[test]
    fn pictures_in_tables_select_theme_but_code_examples_do_not() {
        let text = "```html\n<picture><source srcset='wrong.png'><img src='same.png'></picture>\n```\n\n<table><tr><td><a href='https://example.com'><picture><source media='(prefers-color-scheme: light)' srcset='light.svg 1x, light2.svg 2x'><source media='(prefers-color-scheme:dark)' srcset='dark.svg'><img src='same.png' alt='Logo' width='80'></picture></a></td></tr></table>";
        let sources = picture_sources(text, false);
        assert_eq!(sources["same.png"].light.as_deref(), Some("light.svg"));
        assert_eq!(sources["same.png"].dark.as_deref(), Some("dark.svg"));
    }
    #[test]
    fn conflicting_pictures_keep_fallback_and_non_theme_media_is_ignored() {
        let html = "<picture><source srcset='a.png'><img src='same.png'></picture><picture><source srcset='b.png'><img src='same.png'></picture><picture><source media='(min-width:1000px)' srcset='wide.png'><img src='other.png'></picture>";
        let sources = picture_sources(html, true);
        assert!(!sources.contains_key("same.png"));
        assert_eq!(sources["other.png"], PictureSources::default());
    }
    #[test]
    fn remote_svg_cannot_embed_a_local_image() {
        let dir = tempfile::tempdir().unwrap();
        let image_path = dir.path().join("private.png");
        image::RgbaImage::from_pixel(1, 1, image::Rgba([255, 0, 0, 255]))
            .save(&image_path)
            .unwrap();
        let svg = format!(
            "<svg xmlns='http://www.w3.org/2000/svg' width='16' height='16'><image href='{}' width='16' height='16'/></svg>",
            image_path.display()
        );
        let png = prepare_image(svg.as_bytes()).unwrap();
        let rgba = image::load_from_memory(&png).unwrap().to_rgba8();
        assert_eq!(rgba.get_pixel(8, 8).0[3], 0);
    }
    #[test]
    fn remote_svg_and_failure_placeholder_are_real_bounded_images() {
        let png = prepare_image(br#"<svg xmlns="http://www.w3.org/2000/svg" width="100000" height="50000"><rect width="100000" height="50000" fill="red"/></svg>"#).unwrap();
        let decoded = image::load_from_memory(&png).unwrap();
        assert_eq!((decoded.width(), decoded.height()), (4000, 2000));
        assert!(image::load_from_memory(&unavailable_image().unwrap()).is_ok());
        assert!(prepare_image(b"not an image").is_err());
        assert!(fetch_image("file:///private/local.png").is_err());
    }
}
