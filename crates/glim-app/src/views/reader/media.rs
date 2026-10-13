use gpui_kit::base::Disableable;
use gpui_kit::{
    component::{
        button::{Button, ButtonVariants},
        h_flex, v_flex,
    },
    *,
};
use std::{path::Path, sync::Arc};

pub(super) struct ImageReader {
    pub pages: Vec<Arc<Image>>,
    pub page: Entity<usize>,
}
impl ImageReader {
    pub fn render(&self, cx: &mut App) -> AnyElement {
        let page = *self.page.read(cx);
        let previous = self.page.clone();
        let next = self.page.clone();
        let count = self.pages.len();
        div()
            .relative()
            .size_full()
            .overflow_hidden()
            .child(
                // Give Contain a symmetric, definite viewport. Padding combined
                // with a percentage-sized image can offset its painting bounds.
                div()
                    .absolute()
                    .inset(px(24.))
                    .flex()
                    .items_center()
                    .justify_center()
                    .overflow_hidden()
                    .child(
                        img(self.pages[page].clone())
                            .size_full()
                            .min_w_0()
                            .min_h_0()
                            .object_fit(ObjectFit::Contain),
                    ),
            )
            .child(if count > 1 {
                h_flex()
                    .absolute()
                    .bottom(px(8.))
                    .left(px(24.))
                    .gap_3()
                    .child(
                        Button::new("image-previous-page")
                            .label(glim_services::locale::tr("Previous page"))
                            .disabled(page == 0)
                            .on_click(move |_, _, cx| {
                                previous.update(cx, |index, cx| {
                                    *index = index.saturating_sub(1);
                                    cx.refresh_windows();
                                })
                            }),
                    )
                    .child(format!("{} / {}", page + 1, count))
                    .child(
                        Button::new("image-next-page")
                            .label(glim_services::locale::tr("Next page"))
                            .disabled(page + 1 == count)
                            .on_click(move |_, _, cx| {
                                next.update(cx, |index, cx| {
                                    *index = (*index + 1).min(count - 1);
                                    cx.refresh_windows();
                                })
                            }),
                    )
                    .into_any_element()
            } else {
                div().into_any_element()
            })
            .into_any_element()
    }
}

/// Keeps one displayed frame and at most one decoded frame awaiting presentation.
pub(super) struct GifReader {
    frame: Option<Arc<RenderImage>>,
    error: Option<String>,
    active: Arc<std::sync::atomic::AtomicBool>,
    task: Option<Task<()>>,
}
impl GifReader {
    pub fn new(bytes: Vec<u8>, cx: &mut Context<Self>) -> Self {
        let data = cx.background_executor().spawn(async move { Ok(bytes) });
        Self::with_data(data, cx)
    }

    pub fn from_url(url: url::Url, cx: &mut Context<Self>) -> Self {
        let data = cx.background_executor().spawn(async move {
            match url.scheme() {
                "http" | "https" => glim_services::markup_images::fetch_image(url.as_str()),
                "file" => {
                    let path = url
                        .to_file_path()
                        .map_err(|_| anyhow::anyhow!("Invalid image path"))?;
                    Ok(glim_services::media::read_image(&path)?.png)
                }
                _ => anyhow::bail!("Unsupported GIF URL"),
            }
        });
        Self::with_data(data, cx)
    }

    fn with_data(data: Task<anyhow::Result<Vec<u8>>>, cx: &mut Context<Self>) -> Self {
        use std::{sync::atomic::Ordering, time::Duration};
        let active = Arc::new(std::sync::atomic::AtomicBool::new(true));
        let playing = active.clone();
        let executor = cx.background_executor().clone();
        let task = cx.spawn(async move |view, cx| {
            let result = async {
                let bytes = data.await?;
                let mut playback = executor
                    .spawn(async move { glim_services::media::GifPlayback::new(Arc::from(bytes)) })
                    .await?;
                let mut delay = Duration::ZERO;
                loop {
                    while !playing.load(Ordering::Relaxed) {
                        cx.background_executor()
                            .timer(Duration::from_millis(100))
                            .await;
                    }
                    // Decode the next frame while the current frame's delay elapses.
                    let work = cx.background_executor().spawn(async move {
                        let result = playback.next_frame().map(|frame| {
                            frame.map(|(mut frame, delay)| {
                                // GPUI's directly supplied textures use BGRA pixels.
                                for pixel in frame.buffer_mut().as_chunks_mut::<4>().0 {
                                    pixel.swap(0, 2);
                                }
                                (Arc::new(RenderImage::new(vec![frame])), delay)
                            })
                        });
                        (playback, result)
                    });
                    cx.background_executor().timer(delay).await;
                    let (decoder, next) = work.await;
                    playback = decoder;
                    let Some((frame, next_delay)) = next? else {
                        break;
                    };
                    while !playing.load(Ordering::Relaxed) {
                        cx.background_executor()
                            .timer(Duration::from_millis(100))
                            .await;
                    }
                    view.update(cx, |view, cx| {
                        if let Some(previous) = view.frame.replace(frame) {
                            cx.drop_image(previous, None);
                        }
                        cx.notify();
                    })?;
                    delay = next_delay;
                }
                Ok::<(), anyhow::Error>(())
            }
            .await;
            let _ = view.update(cx, |view, cx| {
                view.task = None;
                if let Err(error) = result {
                    view.error = Some(format!("{error:#}"));
                }
                cx.notify();
            });
        });
        cx.on_release(|view, cx| {
            view.task = None;
            if let Some(frame) = view.frame.take() {
                cx.defer(move |cx| cx.drop_image(frame, None));
            }
        })
        .detach();
        Self {
            frame: None,
            error: None,
            active,
            task: Some(task),
        }
    }

    pub fn image(&self) -> Option<Result<Arc<RenderImage>, ImageCacheError>> {
        if let Some(frame) = &self.frame {
            Some(Ok(frame.clone()))
        } else {
            self.error.as_ref().map(|error| {
                Err(ImageCacheError::Other(Arc::new(anyhow::anyhow!(
                    error.clone()
                ))))
            })
        }
    }

    pub fn set_active(&self, active: bool) {
        self.active
            .store(active, std::sync::atomic::Ordering::Relaxed);
    }
}
impl Render for GifReader {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let mut content = div().relative().size_full().overflow_hidden();
        if let Some(frame) = &self.frame {
            content = content.child(
                div().absolute().inset(px(24.)).overflow_hidden().child(
                    img(frame.clone())
                        .size_full()
                        .min_w_0()
                        .min_h_0()
                        .object_fit(ObjectFit::Contain),
                ),
            );
        }
        if let Some(error) = &self.error {
            content = content.child(
                div()
                    .absolute()
                    .bottom(px(8.))
                    .left(px(24.))
                    .child(error.clone()),
            );
        }
        content
    }
}

pub(super) struct UnavailableReader {
    issue_url: String,
    reason: String,
}
impl UnavailableReader {
    pub fn new(path: &Path, reason: String) -> Self {
        let extension = path
            .extension()
            .and_then(|s| s.to_str())
            .filter(|s| !s.is_empty())
            .or_else(|| path.file_name().and_then(|s| s.to_str()))
            .unwrap_or("unknown");
        let mut url = url::Url::parse("https://github.com/NeoPanic/Glim/issues/new").unwrap();
        url.query_pairs_mut().append_pair("title", &format!("Unsupported file format: {extension}"))
            .append_pair("body", &format!("Format: {extension}\nGlim: {}\nPlatform: {} / {}\n\nWhat happened:\nCould not display this file.\n\nExpected behavior:\n\nAdditional details (optional):\n", env!("CARGO_PKG_VERSION"), std::env::consts::OS, std::env::consts::ARCH));
        Self {
            issue_url: url.into(),
            reason,
        }
    }
    pub fn render(&self, _cx: &mut App) -> AnyElement {
        let url = self.issue_url.clone();
        v_flex()
            .size_full()
            .items_center()
            .justify_center()
            .gap_3()
            .p_8()
            .child(
                div()
                    .text_xl()
                    .child(glim_services::locale::tr("Cannot open this file")),
            )
            .child(div().text_sm().child(self.reason.clone()))
            .child(
                Button::new("unsupported-issue")
                    .ghost()
                    .label(glim_services::locale::tr("Report an issue on GitHub"))
                    .on_click(move |_, _, cx| cx.open_url(&url)),
            )
            .into_any_element()
    }
}

#[cfg(test)]
mod tests {
    use super::UnavailableReader;
    #[test]
    fn dotfiles_and_extensionless_files_use_their_actual_names() {
        for name in [".DS_Store", ".env", "README"] {
            let issue = UnavailableReader::new(
                &std::path::Path::new("/private/project").join(name),
                "unsupported".into(),
            );
            let url = url::Url::parse(&issue.issue_url).unwrap();
            let query = url
                .query_pairs()
                .collect::<std::collections::HashMap<_, _>>();
            assert_eq!(query["title"], format!("Unsupported file format: {name}"));
            assert!(query["body"].contains(&format!("Format: {name}")));
            assert!(!issue.issue_url.contains("private"));
            assert!(!issue.issue_url.contains("no+extension"));
        }
    }
    #[test]
    fn issue_draft_contains_format_and_version_but_not_private_path() {
        let issue = UnavailableReader::new(
            std::path::Path::new("/private/project/customer.pdf"),
            "Could not decode /private/project/customer.pdf".into(),
        );
        let url = url::Url::parse(&issue.issue_url).unwrap();
        let query = url
            .query_pairs()
            .collect::<std::collections::HashMap<_, _>>();
        assert!(query["body"].contains("Format: pdf"));
        assert!(query["body"].contains(env!("CARGO_PKG_VERSION")));
        assert!(!issue.issue_url.contains("customer"));
        assert!(!issue.issue_url.contains("private"));
    }
}
