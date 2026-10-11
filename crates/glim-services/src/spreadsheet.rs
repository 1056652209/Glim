//! Local, read-only spreadsheet preview. Formula results are never recalculated.
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
        std::fs::metadata(path)?.len() <= 128 * 1024 * 1024,
        "Spreadsheet exceeds 128 MiB preview limit"
    );
    let mut workbook = open_workbook_auto(path)
        .context("Cannot decode spreadsheet; the file may be damaged, encrypted or unsupported")?;
    let names = workbook.sheet_names().to_vec();
    ensure!(!names.is_empty(), "Spreadsheet contains no worksheets");
    let mut html = String::from(
        r#"<!doctype html><html><head><meta charset="utf-8"><meta http-equiv="Content-Security-Policy" content="default-src 'none'; style-src 'unsafe-inline'"><style>
:root{color-scheme:light dark}body{font:14px -apple-system,BlinkMacSystemFont,sans-serif;margin:20px;color:CanvasText;background:Canvas}p{color:GrayText}details{margin:16px 0;border:1px solid GrayText;border-radius:8px;overflow:hidden}summary{padding:12px;font-weight:600;cursor:pointer;background:color-mix(in srgb,CanvasText 5%,Canvas)}.grid{overflow:auto;max-height:70vh}table{border-collapse:separate;border-spacing:0;table-layout:fixed;width:max-content}th,td{box-sizing:border-box;border-right:1px solid GrayText;border-bottom:1px solid GrayText;padding:8px 12px;min-width:160px;max-width:320px;white-space:pre-wrap;overflow-wrap:anywhere;vertical-align:top}th{position:sticky;top:0;background:Canvas;z-index:1}th:first-child{min-width:64px}td{width:240px}tr:nth-child(even){background:color-mix(in srgb,CanvasText 3%,Canvas)}</style></head><body><p>只读预览 · 公式显示已保存的结果，不重新计算 · 图表与原始版式请使用默认应用查看</p>"#,
    );
    for (index, name) in names.iter().take(128).enumerate() {
        let range = workbook
            .worksheet_range(name)
            .with_context(|| format!("Cannot decode worksheet {name}"))?;
        let (rows, cols) = range.get_size();
        html.push_str(&format!(
            "<details {}><summary>{} · {} 行 × {} 列</summary>",
            if index == 0 { "open" } else { "" },
            escape(name),
            rows,
            cols
        ));
        if rows > 5000 || cols > 256 {
            html.push_str("<p>预览仅显示前 5000 行、256 列。</p>");
        }
        html.push_str("<div class=grid><table><thead><tr><th>行</th>");
        for column in 0..cols.min(256) {
            html.push_str(&format!("<th>{}</th>", column + 1));
        }
        html.push_str("</tr></thead><tbody>");
        let mut characters = 0;
        for (row_index, row) in range.rows().take(5000).enumerate() {
            html.push_str(&format!("<tr><th>{}</th>", row_index + 1));
            for cell in row.iter().take(256) {
                let value = cell.to_string();
                characters += value.len();
                ensure!(
                    characters <= 32 * 1024 * 1024,
                    "Worksheet text exceeds 32 MiB preview limit"
                );
                let excerpt: String = value.chars().take(4096).collect();
                html.push_str("<td>");
                html.push_str(&escape(&excerpt));
                if value.chars().count() > 4096 {
                    html.push('…');
                }
                html.push_str("</td>");
            }
            html.push_str("</tr>");
        }
        html.push_str("</tbody></table></div></details>");
        ensure!(
            html.len() <= 64 * 1024 * 1024,
            "Spreadsheet HTML exceeds 64 MiB preview limit"
        );
    }
    if names.len() > 128 {
        html.push_str("<p>预览仅显示前 128 个工作表。</p>");
    }
    html.push_str("</body></html>");
    Ok(html)
}
