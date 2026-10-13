use gpui_kit::{
    component::{
        ActiveTheme, WindowExt,
        button::{Button, ButtonVariants},
        v_flex,
    },
    prelude::FluentBuilder,
    *,
};
use std::path::PathBuf;

pub(super) struct NativeReader {
    path: PathBuf,
    folder: bool,
    shuffle: bool,
    autoplay: bool,
    task: Option<Task<()>>,
    // Drop the native controller before its prepared data (including SQLite snapshots).
    preview: Option<super::native_bridge::NativePreview>,
    prepared: Option<glim_services::preview::PreparedPreview>,
    error: Option<String>,
    obscured: bool,
}
impl NativeReader {
    pub fn new(path: PathBuf, folder: bool, shuffle: bool, cx: &mut Context<Self>) -> Self {
        Self::with_playback(path, folder, shuffle, true, cx)
    }
    pub fn new_paused(path: PathBuf, folder: bool, shuffle: bool, cx: &mut Context<Self>) -> Self {
        Self::with_playback(path, folder, shuffle, false, cx)
    }
    pub fn playlist_shuffle(&self) -> Option<bool> {
        self.folder.then_some(self.shuffle)
    }
    fn with_playback(
        path: PathBuf,
        folder: bool,
        shuffle: bool,
        autoplay: bool,
        cx: &mut Context<Self>,
    ) -> Self {
        let mut view = Self {
            path,
            folder,
            shuffle,
            autoplay,
            task: None,
            preview: None,
            prepared: None,
            error: None,
            obscured: false,
        };
        view.open(cx);
        view
    }
    pub(super) fn open(&mut self, cx: &mut Context<Self>) {
        if self.task.is_some() || self.prepared.is_some() {
            return;
        }
        self.error = None;
        let path = self.path.clone();
        let folder = self.folder;
        let shuffle = self.shuffle;
        let autoplay = self.autoplay;
        let work = cx.background_executor().spawn(async move {
            glim_services::preview::prepare(&path, folder, shuffle, autoplay)
        });
        self.task = Some(cx.spawn(async move |view, cx| {
            let result = work.await;
            let _ = view.update(cx, |v, cx| {
                v.task = None;
                match result {
                    Ok(prepared) => {
                        v.preview = Some(super::native_bridge::NativePreview::new(
                            &prepared.configuration,
                        ));
                        v.prepared = Some(prepared);
                    }
                    Err(e) => v.error = Some(format!("{e:#}")),
                }
                cx.notify();
            });
        }));
        cx.notify();
    }
    pub(super) fn hide(&self) {
        if let Some(preview) = &self.preview {
            preview.hide();
        }
    }
}
impl Render for NativeReader {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // Native views sit above GPUI's Metal surface. Hide them while a toolkit
        // menu/dialog is open so the overlay remains visible and clickable.
        let view = cx.entity().downgrade();
        window.on_next_frame(move |window, cx| {
            let obscured = window.has_active_dialog(cx)
                || window.has_active_sheet(cx)
                || window
                    .context_stack()
                    .iter()
                    .any(|context| context.contains("PopupMenu"));
            let _ = view.update(cx, |view, cx| {
                if view.obscured != obscured {
                    view.obscured = obscured;
                    if obscured {
                        view.hide();
                    }
                    cx.notify();
                }
            });
        });
        if self.obscured {
            return div().size_full().into_any_element();
        }

        if self.preview.is_some() {
            let view = cx.entity();
            return canvas(
                move |bounds, window, cx| {
                    if let Some(preview) = &view.read(cx).preview {
                        preview.layout(bounds, window);
                    }
                },
                |_, _, _, _| {},
            )
            .size_full()
            .into_any_element();
        }
        v_flex()
            .size_full()
            .items_center()
            .justify_center()
            .gap_3()
            .p_6()
            .child(
                div().text_lg().child(
                    self.path
                        .file_name()
                        .unwrap_or_default()
                        .to_string_lossy()
                        .into_owned(),
                ),
            )
            .when(self.task.is_some(), |view| {
                view.child(glim_services::locale::tr("Loading preview…"))
            })
            .when_some(self.error.clone(), |view, error| {
                view.child(div().text_sm().text_color(cx.theme().danger).child(error))
                    .child(
                        Button::new("retry-native-preview")
                            .primary()
                            .label(glim_services::locale::tr("Retry"))
                            .on_click(cx.listener(|v, _, _, cx| v.open(cx))),
                    )
            })
            .into_any_element()
    }
}
