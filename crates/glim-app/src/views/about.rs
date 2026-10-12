use crate::app::actions::{CloseWindow, CloseWorkspace};
use gpui_kit::{
    component::{ActiveTheme, TitleBar, v_flex},
    *,
};

pub(crate) struct About {
    focus: FocusHandle,
}

impl About {
    pub(crate) fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let focus = cx.focus_handle();
        window.focus(&focus, cx);
        Self { focus }
    }
}

impl Render for About {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .size_full()
            .track_focus(&self.focus)
            .on_action(cx.listener(|_, _: &CloseWindow, window, _| window.remove_window()))
            .on_action(cx.listener(|_, _: &CloseWorkspace, window, _| window.remove_window()))
            .child(TitleBar::new().child(glim_services::locale::tr("About Glim")))
            .child(
                v_flex()
                    .flex_1()
                    .items_center()
                    .justify_center()
                    .gap_3()
                    .child(img("branding/glim.png").w(px(88.)).h(px(88.)))
                    .child(div().text_3xl().child("Glim"))
                    .child(env!("CARGO_PKG_VERSION"))
                    .child(div().text_color(cx.theme().muted_foreground).child(
                        glim_services::locale::tr("The all-purpose viewer for the AI era."),
                    )),
            )
    }
}
