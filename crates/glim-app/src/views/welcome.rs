use crate::app::actions::{AddFolder, OpenFile, OpenFolder};
use gpui_kit::{
    component::{
        button::{Button, ButtonVariants},
        h_flex, v_flex,
    },
    *,
};

pub fn welcome(show_open_folder: bool, _cx: &App) -> impl IntoElement {
    v_flex()
        .size_full()
        .items_center()
        .justify_center()
        .px_6()
        .child(
            v_flex()
                .items_center()
                .flex_shrink_0()
                .gap_3()
                .child(
                    h_flex()
                        .gap_4()
                        .child(img("branding/glim.png").size(px(52.)))
                        .child(
                            div()
                                .text_size(px(32.))
                                .font_weight(FontWeight::SEMIBOLD)
                                .child("Glim"),
                        ),
                )
                .child(
                    h_flex()
                        .mt_3()
                        .gap_2()
                        .child(
                            Button::new("welcome-open-file")
                                .primary()
                                .label(glim_services::locale::tr("Open File…"))
                                .on_click(|_, window, cx| {
                                    window.dispatch_action(Box::new(OpenFile), cx);
                                }),
                        )
                        .child(
                            Button::new("welcome-open-folder")
                                .label(glim_services::locale::tr(if show_open_folder {
                                    "Open Folder"
                                } else {
                                    "Add Folder…"
                                }))
                                .on_click(move |_, window, cx| {
                                    if show_open_folder {
                                        window.dispatch_action(Box::new(OpenFolder), cx);
                                    } else {
                                        window.dispatch_action(Box::new(AddFolder), cx);
                                    }
                                }),
                        ),
                ),
        )
}
