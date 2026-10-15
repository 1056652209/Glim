use super::source::SourceReader;
use glim_core::{DiffDocument, diff_content, split_diff::split_patch};
use glim_services::limits as reader_limits;
use gpui_kit::*;
pub(super) struct DiffReader {
    pub source: SourceReader,
    pub split: Option<Entity<crate::views::split_diff::SplitDiff>>,
    pub side_by_side: bool,
    pub(super) notice: Option<&'static str>,
}
impl DiffReader {
    pub fn new(document: &DiffDocument, window: &mut Window, cx: &mut App) -> Self {
        let mut preview = String::new();
        let mut shortened = false;
        for (index, line) in document.patch.split_inclusive('\n').enumerate() {
            if index >= reader_limits::DIFF_LINES
                || preview.len() >= reader_limits::DIFF_PREVIEW_BYTES
            {
                shortened = true;
                break;
            }
            if line.len() > reader_limits::DIFF_LINE_BYTES {
                let mut end = reader_limits::DIFF_LINE_BYTES;
                while !line.is_char_boundary(end) {
                    end -= 1;
                }
                preview.push_str(&line[..end]);
                preview.push_str(" … [此行预览已截断]\n");
                shortened = true;
            } else {
                preview.push_str(line);
            }
        }
        let (mut text, spans) = diff_content(&preview);
        if text == "Binary file changed."
            && document
                .change
                .path
                .file_name()
                .is_some_and(|name| name == ".DS_Store")
        {
            text = ".DS_Store changed.\n\nThis is macOS Finder metadata (folder layout and icon positions), stored in a binary format. Git cannot show a line-by-line text diff.\n\nUse View file to inspect its bytes. If this file should not be versioned, ignore it in Git; already tracked files must also be removed from the index.".into();
        }
        let source =
            SourceReader::new(&text, "plain", true, spans, ScrollHandle::new(), window, cx);
        let split = (!shortened)
            .then(|| split_patch(&preview))
            .flatten()
            .map(|patch| cx.new(|cx| crate::views::split_diff::SplitDiff::new(patch, window, cx)));
        Self {
            source,
            side_by_side: split.is_some()
                && cx
                    .try_global::<crate::app::ReaderPreferences>()
                    .and_then(|p| p.diff_side_by_side)
                    .unwrap_or(true),
            split,
            notice: shortened.then_some("大型 diff 使用受限单栏预览：最多 16 MiB、50,000 行，每行 64 KiB。截断仅影响显示，文件及整文件暂存不变；完整内容可用 View file 查看。"),
        }
    }
    pub fn render(&mut self, cx: &mut App) -> AnyElement {
        if self.side_by_side
            && let Some(split) = &self.split
        {
            split.clone().into_any_element()
        } else {
            gpui_kit::component::v_flex()
                .size_full()
                .min_h_0()
                .children(
                    self.notice
                        .map(|notice| div().p_3().text_sm().child(notice)),
                )
                .child(
                    div()
                        .flex_1()
                        .min_h_0()
                        .size_full()
                        .child(self.source.render(cx)),
                )
                .into_any_element()
        }
    }
}
