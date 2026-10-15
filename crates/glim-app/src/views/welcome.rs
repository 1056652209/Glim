use crate::app::actions::{AddFolder, NewWindow, OpenFile, OpenFolder};
use gpui_kit::{
    component::{
        ActiveTheme,
        button::{Button, ButtonVariants},
        h_flex, v_flex,
    },
    prelude::FluentBuilder,
    *,
};

pub fn welcome(
    current: &super::workspace::Workspace,
    current_id: EntityId,
    cx: &mut Context<super::workspace::Workspace>,
) -> impl IntoElement {
    let show_open_folder = current.folder_roots().is_empty();
    let recent = cx
        .try_global::<crate::app::RecentWorkspaces>()
        .map(|history| {
            history
                .0
                .iter()
                .filter(|workspace| !current.matches_recent_workspace(workspace))
                .cloned()
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let more = recent.len() > current.recent_visible();
    let content = v_flex()
        .w_full()
        .max_w(px(640.))
        .min_w_0()
        .flex_shrink_0()
        .gap_6()
        .child(
            v_flex()
                .w_full()
                .min_w_0()
                .gap_5()
                .child(
                    h_flex()
                        .items_center()
                        .gap_3()
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
                        .w_full()
                        .flex_wrap()
                        .gap_2()
                        .child(
                            Button::new("welcome-open-file")
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
                        )
                        .child(
                            Button::new("welcome-new-workspace")
                                .label(glim_services::locale::tr("Open New Workspace"))
                                .on_click(|_, window, cx| {
                                    window.dispatch_action(Box::new(NewWindow), cx);
                                }),
                        ),
                ),
        )
        .when(!recent.is_empty(), |view| {
            view.child(
                v_flex()
                    .w_full()
                    .min_w_0()
                    .pt_5()
                    .border_t_1()
                    .border_color(cx.theme().border)
                    .gap_3()
                    .text_left()
                    .child(
                        div()
                            .px_3()
                            .text_sm()
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(cx.theme().muted_foreground)
                            .child(glim_services::locale::tr("Recent Workspaces")),
                    )
                    .child(
                        v_flex()
                            .id("recent-workspaces")
                            .w_full()
                            .min_w_0()
                            .gap_1()
                            .children(
                                recent
                                    .into_iter()
                                    .take(current.recent_visible())
                                    .enumerate()
                                    .map(|(index, workspace)| {
                                        let title = workspace
                                            .folders
                                            .iter()
                                            .map(|path| {
                                                path.file_name()
                                                    .unwrap_or(path.as_os_str())
                                                    .to_string_lossy()
                                                    .into_owned()
                                            })
                                            .collect::<Vec<_>>()
                                            .join(" · ");
                                        let paths = workspace
                                            .folders
                                            .iter()
                                            .map(|path| path.display().to_string())
                                            .collect::<Vec<_>>()
                                            .join(" · ");
                                        let action = glim_services::locale::tr(
                                            if crate::app::recent_workspace_is_open(
                                                &workspace, current_id, cx,
                                            ) {
                                                "Switch to Window"
                                            } else {
                                                "Open in New Window"
                                            },
                                        );
                                        Button::new(("recent-workspace", index))
                                            .ghost()
                                            .w_full()
                                            .h(px(64.))
                                            .px_3()
                                            .flex_shrink_0()
                                            .tooltip(format!("{}\n{}", action, paths))
                                            .accessibility_label(format!("{}: {}", action, title))
                                            .child(
                                                v_flex()
                                                    .w_full()
                                                    .min_w_0()
                                                    .text_left()
                                                    .gap_1()
                                                    .child(div().text_sm().truncate().child(title))
                                                    .child(
                                                        div()
                                                            .text_xs()
                                                            .truncate()
                                                            .text_color(cx.theme().muted_foreground)
                                                            .child(paths),
                                                    ),
                                            )
                                            .on_click(move |_, _, cx| {
                                                crate::app::open_recent_workspace(
                                                    workspace.clone(),
                                                    cx,
                                                );
                                            })
                                    }),
                            ),
                    )
                    .when(more, |view| {
                        view.child(
                            Button::new("recent-more")
                                .ghost()
                                .label(glim_services::locale::tr("Show more"))
                                .on_click(cx.listener(|view, _, _, cx| view.show_more_recent(cx))),
                        )
                    }),
            )
        });

    v_flex()
        .id("welcome-page")
        .size_full()
        .min_h_0()
        .overflow_y_scroll()
        .items_center()
        .px_6()
        .pt(px(64.))
        .pb_8()
        .child(content)
}
