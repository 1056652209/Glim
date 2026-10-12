use gpui_kit::{component::TitleBar, *};

#[derive(Default)]
struct AboutWindow(Option<AnyWindowHandle>);
impl Global for AboutWindow {}

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
        cx.new(|cx| crate::views::about::About::new(window, cx))
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
