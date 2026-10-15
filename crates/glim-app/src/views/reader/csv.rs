use glim_services::limits as reader_limits;
use gpui_kit::{
    base::InteractiveElementExt as _,
    component::{
        ActiveTheme, Disableable,
        button::{Button, ButtonVariants},
        h_flex, v_flex,
    },
    *,
};
use std::{path::PathBuf, sync::Arc};

#[derive(Clone)]
enum Source {
    Text(Arc<String>),
    File(PathBuf),
}
pub(super) struct CsvTable {
    source: Option<Source>,
    delimiter: u8,
    rows: Vec<Vec<String>>,
    columns: usize,
    format: &'static str,
    row_start: usize,
    column_start: usize,
    previous: Vec<usize>,
    more: bool,
    task: Option<Task<()>>,
    error: Option<String>,
    scroll: UniformListScrollHandle,
}
impl CsvTable {
    pub fn new() -> Self {
        Self {
            source: None,
            delimiter: b',',
            rows: Vec::new(),
            columns: 0,
            format: "CSV",
            row_start: 0,
            column_start: 0,
            previous: Vec::new(),
            more: false,
            task: None,
            error: None,
            scroll: UniformListScrollHandle::new(),
        }
    }
    pub fn from_file(path: PathBuf, delimiter: u8) -> Self {
        let mut table = Self::new();
        table.source = Some(Source::File(path));
        table.delimiter = delimiter;
        table.format = if delimiter == b'\t' { "TSV" } else { "CSV" };
        table
    }
    pub fn load(&mut self, text: String, delimiter: u8, cx: &mut Context<Self>) {
        if matches!(&self.source, Some(Source::Text(old)) if old.as_str() == text) {
            return;
        }
        self.source = Some(Source::Text(Arc::new(text)));
        self.delimiter = delimiter;
        self.format = if delimiter == b'\t' { "TSV" } else { "CSV" };
        self.reload(cx);
    }
    pub fn reload(&mut self, cx: &mut Context<Self>) {
        self.row_start = 0;
        self.column_start = 0;
        self.previous.clear();
        self.load_page(cx);
    }
    fn load_page(&mut self, cx: &mut Context<Self>) {
        let Some(source) = self.source.clone() else {
            return;
        };
        self.task = None;
        self.error = None;
        self.rows.clear();
        self.more = false;
        let (delimiter, row, column) = (self.delimiter, self.row_start, self.column_start);
        let (cancel, guard) = glim_services::work::Cancellation::task();
        let work = cx.background_executor().spawn(async move {
            match source {
                Source::Text(text) => {
                    glim_services::csv::read_page(text.as_bytes(), delimiter, row, column, &cancel)
                }
                Source::File(path) => glim_services::csv::read_page(
                    std::fs::File::open(path)?,
                    delimiter,
                    row,
                    column,
                    &cancel,
                ),
            }
        });
        self.task = Some(cx.spawn(async move |view, cx| {
            let _guard = guard;
            let result = work.await;
            let _ = view.update(cx, |view, cx| {
                view.task = None;
                match result {
                    Ok(page) => {
                        view.rows = page.rows;
                        view.columns = page.columns;
                        view.more = page.more;
                    }
                    Err(error) => {
                        view.error = Some(format!("Cannot show {} table: {error:#}", view.format))
                    }
                }
                view.scroll = UniformListScrollHandle::new();
                cx.notify();
            });
        }));
        cx.notify();
    }
    fn show_cell(&self, row: usize, column: usize, window: &mut Window, cx: &mut Context<Self>) {
        let Some(value) = self.rows.get(row).and_then(|row| row.get(column)).cloned() else {
            return;
        };
        super::source::show_text_dialog(
            glim_services::locale::tr("Full cell value"),
            value,
            window,
            cx,
        );
    }
}
impl Render for CsvTable {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let busy = self.task.is_some();
        let columns = self
            .columns
            .saturating_sub(self.column_start)
            .min(reader_limits::CSV_COLUMNS);
        let width = 64. + columns as f32 * 200.;
        v_flex().size_full().min_h_0()
            .child(h_flex().p_2().gap_2().text_xs()
                .child(div().flex_1().min_w_0().truncate().child(if busy { glim_services::locale::tr("Loading table…") } else {
                    self.error.clone().unwrap_or_else(|| format!("{} · rows {}–{} · columns {}–{}", self.format,
                        self.row_start + 1, self.row_start + self.rows.len(), self.column_start + 1, self.column_start + columns))
                }))
                .child(Button::new("csv-prev").ghost().label(glim_services::locale::tr("Previous"))
                    .disabled(busy || self.previous.is_empty()).on_click(cx.listener(|view, _, _, cx| {
                        if let Some(start) = view.previous.pop() { view.row_start = start; view.load_page(cx); }
                    })))
                .child(Button::new("csv-next").ghost().label(glim_services::locale::tr("Next page"))
                    .disabled(busy || !self.more).on_click(cx.listener(|view, _, _, cx| {
                        view.previous.push(view.row_start); view.row_start += view.rows.len(); view.load_page(cx);
                    })))
                .child(Button::new("csv-prev-cols").ghost().label(glim_services::locale::tr("Previous columns"))
                    .disabled(busy || self.column_start == 0).on_click(cx.listener(|view, _, _, cx| {
                        view.column_start = view.column_start.saturating_sub(reader_limits::CSV_COLUMNS); view.load_page(cx);
                    })))
                .child(Button::new("csv-next-cols").ghost().label(glim_services::locale::tr("Next columns"))
                    .disabled(busy || self.column_start + columns >= self.columns).on_click(cx.listener(|view, _, _, cx| {
                        view.column_start += reader_limits::CSV_COLUMNS; view.load_page(cx);
                    }))))
            .child(div().id("csv-horizontal").flex_1().min_h_0().overflow_x_scroll().child(
                v_flex().w(px(width)).h_full()
                    .child(h_flex().h(px(32.)).child(div().w(px(64.)).child(glim_services::locale::tr("Row")))
                        .children((0..columns).map(|column| div().w(px(200.)).px_3().child((self.column_start + column + 1).to_string()))))
                    .child(uniform_list("csv-rows", self.rows.len(), cx.processor(move |view, range: std::ops::Range<usize>, _, cx| {
                        range.map(|row| {
                            h_flex().h(px(32.)).w_full().border_b_1().border_color(cx.theme().border)
                                .child(div().w(px(64.)).flex_shrink_0().px_2().text_color(cx.theme().muted_foreground)
                                    .child((view.row_start + row + 1).to_string()))
                                .children((0..columns).map(|column| {
                                    let value = view.rows[row].get(column).map(String::as_str).unwrap_or("");
                                    div().id(("csv-cell", row * columns + column)).w(px(200.)).flex_shrink_0().px_3().truncate()
                                        .child(value.chars().take(reader_limits::CSV_CELL_CHARS).collect::<String>())
                                        .on_double_click(cx.listener(move |view, _, window, cx| view.show_cell(row, column, window, cx)))
                                })).into_any_element()
                        }).collect()
                    })).track_scroll(&self.scroll).flex_1().min_h_0())
            ))
    }
}
