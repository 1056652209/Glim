use super::{Page, Workspace};
use crate::views::reader::Reader;
use gpui_kit::{
    assets::IconName,
    component::{
        ActiveTheme, Disableable, Icon, Selectable, Sizable,
        button::{Button, ButtonCustomVariant, ButtonVariants},
        h_flex,
    },
    *,
};

#[derive(Clone, Copy)]
enum ToolbarAction {
    Sidebar,
    Back,
    Forward,
    Preview,
    Source,
    SideBySide,
    Inline,
    ViewFile,
    WordWrap,
    FileFullscreen,
    Update,
}

struct ToolbarItem {
    action: ToolbarAction,
    label: &'static str,
    icon: Option<IconName>,
    selected: bool,
    enabled: bool,
}

impl ToolbarAction {
    fn id(self) -> &'static str {
        match self {
            Self::Sidebar => "toggle-sidebar",
            Self::Back => "back",
            Self::Forward => "forward",
            Self::Preview => "reader-preview",
            Self::Source => "reader-source",
            Self::SideBySide => "diff-split",
            Self::Inline => "diff-inline",
            Self::ViewFile => "show-file",
            Self::WordWrap => "word-wrap",
            Self::FileFullscreen => "file-fullscreen",
            Self::Update => "update-available",
        }
    }
}

impl ToolbarItem {
    fn new(action: ToolbarAction) -> Self {
        let (label, icon) = match action {
            ToolbarAction::Sidebar => ("Toggle Sidebar", Some(IconName::PanelLeft)),
            ToolbarAction::Back => ("Back", Some(IconName::ArrowLeft)),
            ToolbarAction::Forward => ("Forward", Some(IconName::ArrowRight)),
            ToolbarAction::Preview => ("Preview", Some(IconName::Eye)),
            ToolbarAction::Source => ("Source", Some(IconName::Code)),
            ToolbarAction::SideBySide => ("Side by side", Some(IconName::Columns2)),
            ToolbarAction::Inline => ("Inline", Some(IconName::Rows2)),
            ToolbarAction::ViewFile => ("View file", Some(IconName::FileText)),
            ToolbarAction::WordWrap => ("Word wrap", None),
            ToolbarAction::FileFullscreen => ("Enter File Fullscreen", None),
            ToolbarAction::Update => ("Update Available", Some(IconName::Download)),
        };
        Self {
            action,
            label,
            icon,
            selected: false,
            enabled: true,
        }
    }
}

/// Fixed order: viewing mode, related file action, then text layout.
/// Capabilities come from the active reader, not a separate extension allowlist.
fn document_items(reader: &Reader, word_wrap: bool) -> Vec<ToolbarItem> {
    let mut items = Vec::new();
    if reader.diff.is_some() || reader.historical {
        let mut split = ToolbarItem::new(ToolbarAction::SideBySide);
        split.selected = reader.side_by_side();
        split.enabled = reader.split_available();
        let mut inline = ToolbarItem::new(ToolbarAction::Inline);
        inline.selected = !split.selected;
        items.extend([split, inline]);
        if reader.diff.is_some() {
            items.push(ToolbarItem::new(ToolbarAction::ViewFile));
        }
    } else if reader.can_preview() {
        let mut preview = ToolbarItem::new(ToolbarAction::Preview);
        if reader.is_json() {
            preview.label = "JSON tree";
            preview.icon = Some(IconName::ListTree);
        } else if reader.is_csv() {
            preview.label = if reader
                .path
                .extension()
                .is_some_and(|e| e.eq_ignore_ascii_case("tsv"))
            {
                "TSV table"
            } else {
                "CSV table"
            };
            preview.icon = Some(IconName::Rows2);
        }
        preview.selected = reader.toggle_label() == "Source";
        let mut source = ToolbarItem::new(ToolbarAction::Source);
        source.selected = !preview.selected;
        items.extend([preview, source]);
    }
    if reader.can_word_wrap() {
        let mut wrap = ToolbarItem::new(ToolbarAction::WordWrap);
        wrap.selected = word_wrap;
        wrap.enabled = !reader.can_preview() || reader.toggle_label() != "Source";
        if !wrap.enabled {
            wrap.label = "Word wrap · Source view";
        }
        items.push(wrap);
    }
    items
}

impl Workspace {
    fn toolbar_button(&self, item: ToolbarItem, cx: &mut Context<Self>) -> Button {
        let action = item.action;
        let label = glim_services::locale::tr(item.label);
        let icon = match item.icon {
            Some(icon) => Icon::new(icon),
            None if matches!(action, ToolbarAction::FileFullscreen) => {
                Icon::empty().path(if self.file_fullscreen {
                    "navigation/file-fullscreen-exit.svg"
                } else {
                    "navigation/file-fullscreen.svg"
                })
            }
            None => Icon::empty().path("navigation/wrap.svg"),
        };
        Button::new(action.id())
            .custom(
                ButtonCustomVariant::new(cx)
                    .hover(cx.theme().foreground.opacity(0.05))
                    .active(cx.theme().foreground.opacity(0.09)),
            )
            .small()
            .w(px(28.))
            .h(px(28.))
            .flex_shrink_0()
            .rounded(px(7.))
            .icon(icon.size(px(16.)))
            .tooltip(label.clone())
            .accessibility_label(label)
            .selected(item.selected)
            .disabled(!item.enabled)
            .on_click(cx.listener(move |view, _, window, cx| {
                view.run_toolbar_action(action, window, cx);
            }))
    }

    pub(super) fn navigation_toolbar(&self, cx: &mut Context<Self>) -> AnyElement {
        let mut back = ToolbarItem::new(ToolbarAction::Back);
        back.enabled = self.history_cursor.is_some_and(|i| i > 0);
        let mut forward = ToolbarItem::new(ToolbarAction::Forward);
        forward.enabled = self
            .history_cursor
            .is_some_and(|i| i + 1 < self.history.len());
        h_flex()
            .h(px(28.))
            .items_center()
            .gap_1()
            .flex_shrink_0()
            .child(self.toolbar_button(ToolbarItem::new(ToolbarAction::Sidebar), cx))
            .child(self.toolbar_button(back, cx))
            .child(self.toolbar_button(forward, cx))
            .into_any_element()
    }

    pub(super) fn workspace_toolbar(&self, cx: &mut Context<Self>) -> AnyElement {
        let mut document = if self.page == Page::Document {
            self.active_reader()
                .map(|r| document_items(r, self.word_wrap))
                .unwrap_or_default()
        } else {
            Vec::new()
        };
        if self.page == Page::Document && self.active_reader().is_some() {
            let mut fullscreen = ToolbarItem::new(ToolbarAction::FileFullscreen);
            fullscreen.selected = self.file_fullscreen;
            if self.file_fullscreen {
                fullscreen.label = "Exit File Fullscreen";
            }
            document.push(fullscreen);
        }
        let has_update = crate::app::updates::state(cx)
            .is_some_and(|state| state.read(cx).snapshot.has_update());
        let border = cx.theme().border;
        let separator = || div().w(px(1.)).h(px(16.)).mx_1().bg(border);
        let has_document = !document.is_empty();
        let mut toolbar = h_flex().gap_1().flex_shrink_0();
        if !document.is_empty() {
            toolbar = toolbar.children(
                document
                    .into_iter()
                    .map(|item| self.toolbar_button(item, cx)),
            );
        }
        if has_update {
            if has_document {
                toolbar = toolbar.child(separator());
            }
            toolbar =
                toolbar.child(self.toolbar_button(ToolbarItem::new(ToolbarAction::Update), cx));
        }
        toolbar.into_any_element()
    }

    fn run_toolbar_action(
        &mut self,
        action: ToolbarAction,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        match action {
            ToolbarAction::Sidebar => {
                self.sidebar_visible = !self.sidebar_visible;
                self.sidebar_window_width = 0.;
                cx.notify();
            }
            ToolbarAction::Back => self.navigate(false, window, cx),
            ToolbarAction::Forward => self.navigate(true, window, cx),
            ToolbarAction::Preview | ToolbarAction::Source => {
                if self.active_reader().is_some_and(|r| r.can_preview()) {
                    self.set_reader_preview(matches!(action, ToolbarAction::Preview), window, cx);
                }
            }
            ToolbarAction::SideBySide | ToolbarAction::Inline => {
                if self
                    .active_reader()
                    .is_some_and(|r| r.diff.is_some() || r.historical)
                {
                    self.set_diff_mode(matches!(action, ToolbarAction::SideBySide), cx);
                }
            }
            ToolbarAction::ViewFile => {
                if let Some(reader) = self.active_reader().filter(|r| r.diff.is_some()) {
                    self.open_path(reader.path.clone(), window, cx);
                }
            }
            ToolbarAction::WordWrap => {
                if self.active_reader().is_some_and(|r| r.can_word_wrap()) {
                    self.toggle_word_wrap(window, cx);
                }
            }
            ToolbarAction::FileFullscreen => {
                if self.page == Page::Document && self.active_reader().is_some() {
                    self.file_fullscreen = !self.file_fullscreen;
                    self.results.clear();
                    self.search_task = None;
                    self.history_swipe = None;
                    // Keep the existing reader and its editing/preview state alive.
                    window.focus(&self.focus, cx);
                    cx.notify();
                }
            }
            ToolbarAction::Update => crate::app::updates::show(window, cx),
        }
    }
}
