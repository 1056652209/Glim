use crate::app::actions::{CloseWindow, CloseWorkspace};
use gpui_kit::{
    component::{TitleBar, button::Button, h_flex, v_flex},
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
                    .child(
                        h_flex()
                            .gap_4()
                            .child(img("branding/glim.png").size(px(64.)))
                            .child(
                                div()
                                    .text_3xl()
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .child("Glim"),
                            ),
                    )
                    .child(env!("CARGO_PKG_VERSION"))
                    .child(
                        Button::new("about-check-update")
                            .label(glim_services::locale::tr("Check for Updates…"))
                            .on_click(|_, window, cx| {
                                crate::app::updates::show(window, cx);
                                crate::app::updates::command("check", cx);
                            }),
                    ),
            )
    }
}
