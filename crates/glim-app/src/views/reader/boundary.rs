//! Attribute lazy GPUI layout and paint failures to the owning file.
use glim_services::diagnostics::FileScope;
use gpui_kit::*;
use std::path::PathBuf;

pub(super) struct FileBoundary {
    pub path: PathBuf,
    pub content: AnyElement,
}
impl IntoElement for FileBoundary {
    type Element = Self;
    fn into_element(self) -> Self {
        self
    }
}
impl Element for FileBoundary {
    type RequestLayoutState = ();
    type PrepaintState = ();
    fn id(&self) -> Option<ElementId> {
        None
    }
    fn source_location(&self) -> Option<&'static std::panic::Location<'static>> {
        None
    }
    fn request_layout(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, ()) {
        let _scope = FileScope::enter(&self.path, "layout");
        // Measure the entire file subtree inside prepaint's file scope. Taffy
        // otherwise invokes lazy measurement after request_layout has returned.
        let mut style = Style::default();
        style.size.width = relative(1.).into();
        style.size.height = relative(1.).into();
        (window.request_layout(style, [], cx), ())
    }
    fn prepaint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        _: &mut (),
        window: &mut Window,
        cx: &mut App,
    ) {
        let _scope = FileScope::enter(&self.path, "prepaint");
        self.content
            .prepaint_as_root(bounds.origin, bounds.size.into(), window, cx);
    }
    fn paint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        _: Bounds<Pixels>,
        _: &mut (),
        _: &mut (),
        window: &mut Window,
        cx: &mut App,
    ) {
        let _scope = FileScope::enter(&self.path, "paint");
        self.content.paint(window, cx);
    }
}

pub(super) fn failed(path: &std::path::Path, error: String, cx: &App) -> AnyElement {
    use gpui_kit::component::{ActiveTheme, button::Button};
    let retry = path.to_path_buf();
    let log_directory = glim_services::diagnostics::directory();
    div()
        .size_full()
        .p_6()
        .flex()
        .flex_col()
        .gap_3()
        .text_color(cx.theme().foreground)
        .child(glim_services::locale::tr("This file could not be rendered"))
        .child(path.display().to_string())
        .child(
            div()
                .text_sm()
                .text_color(cx.theme().muted_foreground)
                .child(error),
        )
        .child(glim_services::locale::tr(
            "Other files remain available. Details were saved to the log.",
        ))
        .child(
            Button::new("retry-file-render")
                .label(glim_services::locale::tr("Retry rendering"))
                .on_click(move |_, window, _| {
                    glim_services::diagnostics::retry(&retry);
                    window.refresh();
                }),
        )
        .child(
            Button::new("open-diagnostic-logs")
                .label(glim_services::locale::tr("Open logs"))
                .on_click(move |_, _, cx| {
                    cx.reveal_path(&log_directory);
                }),
        )
        .into_any_element()
}
