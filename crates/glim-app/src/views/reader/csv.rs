use gpui_kit::{
    component::{ActiveTheme, h_flex, v_flex},
    *,
};
pub(super) struct CsvTable {
    rows: Vec<Vec<String>>,
    columns: usize,
    loaded: Option<String>,
    task: Option<Task<()>>,
    error: Option<String>,
    scroll: UniformListScrollHandle,
}
impl CsvTable {
    pub fn new() -> Self {
        Self {
            rows: Vec::new(),
            columns: 0,
            loaded: None,
            task: None,
            error: None,
            scroll: UniformListScrollHandle::new(),
        }
    }
    pub fn load(&mut self, text: String, cx: &mut Context<Self>) {
        if self.loaded.as_deref() == Some(&text) {
            return;
        }
        self.loaded = Some(text.clone());
        self.error = None;
        self.rows.clear();
        self.scroll = UniformListScrollHandle::new();
        let work = cx
            .background_executor()
            .spawn(async move { glim_services::csv::parse(&text) });
        self.task = Some(cx.spawn(async move |view, cx| {
            let result = work.await;
            let _ = view.update(cx, |view, cx| {
                view.task = None;
                match result {
                    Ok(rows) => {
                        view.columns = rows.iter().map(Vec::len).max().unwrap_or(0);
                        view.rows = rows;
                    }
                    Err(error) => view.error = Some(format!("Cannot show CSV table: {error:#}")),
                }
                cx.notify();
            });
        }));
        cx.notify();
    }
}
impl Render for CsvTable {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let width = 64. + self.columns as f32 * 200.;
        v_flex()
            .size_full()
            .min_h_0()
            .child(div().p_3().text_sm().child(if self.task.is_some() {
                glim_services::locale::tr("Loading table…")
            } else {
                self.error.clone().unwrap_or_else(|| {
                    format!(
                        "CSV · {} 行 × {} 列 · 只读表格，编辑请切换原文",
                        self.rows.len(),
                        self.columns
                    )
                })
            }))
            .child(
                div()
                    .id("csv-horizontal")
                    .flex_1()
                    .min_h_0()
                    .overflow_x_scroll()
                    .child(
                        v_flex()
                            .w(px(width.max(64.)))
                            .h_full()
                            .child(
                                h_flex()
                                    .h(px(32.))
                                    .child(div().w(px(64.)).child(glim_services::locale::tr("Row")))
                                    .children((0..self.columns).map(|column| {
                                        div().w(px(200.)).px_3().child(format!("列 {}", column + 1))
                                    })),
                            )
                            .child(
                                uniform_list(
                                    "csv-rows",
                                    self.rows.len(),
                                    cx.processor(|view, range: std::ops::Range<usize>, _, cx| {
                                        range
                                            .map(|row| {
                                                h_flex()
                                                    .h(px(32.))
                                                    .w_full()
                                                    .border_b_1()
                                                    .border_color(cx.theme().border)
                                                    .child(
                                                        div()
                                                            .w(px(64.))
                                                            .flex_shrink_0()
                                                            .px_2()
                                                            .text_color(cx.theme().muted_foreground)
                                                            .child((row + 1).to_string()),
                                                    )
                                                    .children((0..view.columns).map(|column| {
                                                        let value = view.rows[row]
                                                            .get(column)
                                                            .map(String::as_str)
                                                            .unwrap_or("");
                                                        div()
                                                            .w(px(200.))
                                                            .flex_shrink_0()
                                                            .px_3()
                                                            .truncate()
                                                            .child(
                                                                value
                                                                    .chars()
                                                                    .take(4096)
                                                                    .collect::<String>(),
                                                            )
                                                    }))
                                                    .into_any_element()
                                            })
                                            .collect()
                                    }),
                                )
                                .track_scroll(&self.scroll)
                                .flex_1()
                                .min_h_0(),
                            ),
                    ),
            )
    }
}
