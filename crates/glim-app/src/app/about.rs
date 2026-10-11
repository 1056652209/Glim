use super::actions::{CloseWindow, CloseWorkspace};
use gpui_kit::{
    component::{ActiveTheme, TitleBar, v_flex},
    *,
};

#[derive(Default)]
struct AboutWindow(Option<AnyWindowHandle>);
impl Global for AboutWindow {}

struct About {
    focus: FocusHandle,
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

pub fn open(cx: &mut App) {
    if let Some(handle) = cx.try_global::<AboutWindow>().and_then(|state| state.0)
        && handle
            .update(cx, |_, window, _| window.activate_window())
            .is_ok()
    {
        cx.activate(true);
        return;
    }
    let options = WindowOptions {
        window_bounds: Some(WindowBounds::Windowed(Bounds::centered(
            None,
            size(px(420.), px(340.)),
            cx,
        ))),
        ..TitleBar::window_options()
    };
    match gpui_kit::open_window(options, cx, |window, cx| {
        cx.new(|cx| {
            let focus = cx.focus_handle();
            window.focus(&focus, cx);
            About { focus }
        })
    }) {
        Ok((handle, _)) => {
            cx.set_global(AboutWindow(Some(handle)));
            cx.activate(true);
        }
        Err(error) => eprintln!("Cannot open About Glim: {error:#}"),
    }
}

#[cfg(test)]
mod tests {
    use super::AboutWindow;
    use gpui_kit::{AppContext, TestAppContext};

    #[gpui_kit::test]
    fn about_opens_without_workspace_and_reopens_after_close(cx: &mut TestAppContext) {
        cx.update(gpui_kit::init);
        cx.update(|cx| {
            cx.set_global(super::super::OpenWorkspaces::default());
            super::super::show_about(cx);
        });
        let first = cx.update(|cx| {
            assert_eq!(cx.windows().len(), 1);
            cx.global::<AboutWindow>().0.unwrap()
        });
        cx.update(super::super::show_about);
        cx.update(|cx| assert_eq!(cx.windows(), vec![first]));
        cx.update_window(first, |_, window, _| window.remove_window())
            .unwrap();
        cx.run_until_parked();
        cx.update(super::super::show_about);
        cx.update(|cx| {
            assert_eq!(cx.windows().len(), 1);
            assert_ne!(cx.global::<AboutWindow>().0.unwrap(), first);
        });
    }
}
