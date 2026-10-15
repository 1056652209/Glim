//! Local, read-only spreadsheet preview. Formula results are never recalculated.
use crate::limits as reader_limits;
use anyhow::{Context, Result, ensure};
use calamine::{Reader, open_workbook_auto};
use std::path::Path;

fn escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

pub fn preview(path: &Path) -> Result<String> {
    ensure!(
        std::fs::metadata(path)?.len() <= reader_limits::SPREADSHEET_INPUT_BYTES,
        "Spreadsheet exceeds the preview input size limit"
    );
    let mut workbook = open_workbook_auto(path)
        .context("Cannot decode spreadsheet; the file may be damaged, encrypted or unsupported")?;
    let names = workbook.sheet_names().to_vec();
    ensure!(!names.is_empty(), "Spreadsheet contains no worksheets");
    let mut html = String::from(
        r#"<!doctype html><html><head><meta charset="utf-8"><meta http-equiv="Content-Security-Policy" content="default-src 'none'; style-src 'unsafe-inline'"><style>
:root{color-scheme:light dark}body{font:14px -apple-system,BlinkMacSystemFont,sans-serif;margin:20px;color:CanvasText;background:Canvas}p{color:GrayText}details{margin:16px 0;border:1px solid GrayText;border-radius:8px;overflow:hidden}summary{padding:12px;font-weight:600;cursor:pointer;background:color-mix(in srgb,CanvasText 5%,Canvas)}.grid{overflow:auto;max-height:70vh}table{border-collapse:separate;border-spacing:0;table-layout:fixed;width:max-content}th,td{box-sizing:border-box;border-right:1px solid GrayText;border-bottom:1px solid GrayText;padding:8px 12px;min-width:160px;max-width:320px;white-space:pre-wrap;overflow-wrap:anywhere;vertical-align:top}th{position:sticky;top:0;background:Canvas;z-index:1}th:first-child{min-width:64px}td{width:240px}tr:nth-child(even){background:color-mix(in srgb,CanvasText 3%,Canvas)}</style></head><body><p>__PREVIEW_NOTE__</p>"#,
    );
    html = html.replace("__PREVIEW_NOTE__", &escape(&crate::locale::tr("Read-only preview · Formulas show saved results without recalculation · Open in the default app for charts and original layout")));
    for (index, name) in names
        .iter()
        .take(reader_limits::SPREADSHEET_SHEETS)
        .enumerate()
    {
        let range = workbook
            .worksheet_range(name)
            .with_context(|| format!("Cannot decode worksheet {name}"))?;
        let (rows, cols) = range.get_size();
        html.push_str(&format!(
            "<details {}><summary>{} · {} {} × {} {}</summary>",
            if index == 0 { "open" } else { "" },
            escape(name),
            rows,
            escape(&crate::locale::tr("rows")),
            cols,
            escape(&crate::locale::tr("columns"))
        ));
        if rows > reader_limits::SPREADSHEET_ROWS || cols > reader_limits::SPREADSHEET_COLUMNS {
            html.push_str(&format!(
                "<p>{}</p>",
                escape(
                    &crate::locale::tr("Preview limited to %rows rows and %columns columns.")
                        .replace("%rows", &reader_limits::SPREADSHEET_ROWS.to_string())
                        .replace("%columns", &reader_limits::SPREADSHEET_COLUMNS.to_string())
                )
            ));
        }
        html.push_str(&format!(
            "<div class=grid><table><thead><tr><th>{}</th>",
            escape(&crate::locale::tr("Row"))
        ));
        for column in 0..cols.min(reader_limits::SPREADSHEET_COLUMNS) {
            html.push_str(&format!("<th>{}</th>", column + 1));
        }
        html.push_str("</tr></thead><tbody>");
        let mut characters = 0;
        for (row_index, row) in range
            .rows()
            .take(reader_limits::SPREADSHEET_ROWS)
            .enumerate()
        {
            html.push_str(&format!("<tr><th>{}</th>", row_index + 1));
            for cell in row.iter().take(reader_limits::SPREADSHEET_COLUMNS) {
                let value = cell.to_string();
                characters += value.len();
                ensure!(
                    characters <= reader_limits::SPREADSHEET_SHEET_TEXT_BYTES,
                    "Worksheet text exceeds the preview budget"
                );
                let excerpt: String = value
                    .chars()
                    .take(reader_limits::SPREADSHEET_CELL_CHARS)
                    .collect();
                html.push_str("<td>");
                html.push_str(&escape(&excerpt));
                if value.chars().count() > reader_limits::SPREADSHEET_CELL_CHARS {
                    html.push('…');
                }
                html.push_str("</td>");
            }
            html.push_str("</tr>");
        }
        html.push_str("</tbody></table></div></details>");
        ensure!(
            html.len() <= reader_limits::SPREADSHEET_HTML_BYTES,
            "Spreadsheet HTML exceeds the preview budget"
        );
    }
    if names.len() > reader_limits::SPREADSHEET_SHEETS {
        html.push_str(&format!(
            "<p>{}</p>",
            escape(
                &crate::locale::tr("Preview limited to %sheets worksheets.")
                    .replace("%sheets", &reader_limits::SPREADSHEET_SHEETS.to_string())
            )
        ));
    }
    html.push_str("</body></html>");
    Ok(html)
}
