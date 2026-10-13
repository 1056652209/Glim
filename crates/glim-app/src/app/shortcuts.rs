use super::actions::{CloseWindow, CloseWorkspace};
use gpui_kit::{
    component::{TitleBar, h_flex, v_flex},
    *,
};

struct Shortcuts {
    focus: FocusHandle,
}
impl Render for Shortcuts {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let entries = [
            ("新窗口", "⌘⇧N"),
            ("打开文件", "⌘O"),
            ("打开文件夹", "⌘⇧O"),
            ("追加工作空间目录", "⌘⌥O"),
            ("保存", "⌘S"),
            ("刷新", "⌘R"),
            ("关闭当前标签；无标签时关闭窗口", "⌘W"),
            ("关闭整个工作空间窗口", "⌘⇧W"),
            ("退出 Glim", "⌘Q"),
            ("搜索工作空间文件", "⌘P"),
            ("显示/隐藏侧栏", "⌘B"),
            ("预览/原文切换", "⌘⇧V"),
            ("自动换行", "⌥Z"),
            ("下一标签", "Ctrl+Tab"),
            ("上一标签", "Ctrl+Shift+Tab"),
            ("导航后退/前进", "⌥← / ⌥→"),
            ("深浅色主题", "⌘⌥T"),
            ("快捷键说明", "F1"),
            ("复制/剪切/粘贴/全选（编辑器）", "⌘C / X / V / A"),
            ("撤销（编辑器）", "⌘Z"),
            ("重做（编辑器）", "⌘⇧Z"),
            ("文件内查找（编辑器）", "⌘F"),
            ("提交已暂存内容（提交输入框）", "⌘Enter"),
            ("文件树选择/展开/打开", "↑↓ / ←→ / Enter"),
        ];
        v_flex()
            .size_full()
            .track_focus(&self.focus)
            .on_action(cx.listener(|_, _: &CloseWindow, window, _| window.remove_window()))
            .on_action(cx.listener(|_, _: &CloseWorkspace, window, _| window.remove_window()))
            .child(TitleBar::new().child(glim_services::locale::tr("Keyboard Shortcuts — Glim")))
            .child(
                v_flex()
                    .id("shortcut-list")
                    .flex_1()
                    .min_h_0()
                    .overflow_y_scroll()
                    .p_4()
                    .gap_2()
                    .child(
                        h_flex()
                            .gap_4()
                            .child(div().flex_1().child(glim_services::locale::tr("Operation")))
                            .child(
                                div()
                                    .w(px(190.))
                                    .child(glim_services::locale::tr("Shortcut")),
                            ),
                    )
                    .children(entries.into_iter().map(|(description, mac)| {
                        h_flex()
                            .gap_4()
                            .py_2()
                            .child(div().flex_1().child(glim_services::locale::tr(description)))
                            .child(div().w(px(190.)).child(mac))
                    })),
            )
    }
}
pub fn open(cx: &mut App) {
    let options = WindowOptions {
        window_bounds: Some(WindowBounds::Windowed(Bounds::centered(
            None,
            size(px(680.), px(760.)),
            cx,
        ))),
        ..TitleBar::window_options()
    };
    if let Err(error) = gpui_kit::open_window(options, cx, |window, cx| {
        cx.new(|cx| {
            let focus = cx.focus_handle();
            window.focus(&focus, cx);
            Shortcuts { focus }
        })
    }) {
        eprintln!("Cannot open keyboard shortcuts: {error:#}");
    }
}
