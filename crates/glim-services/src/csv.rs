//! Bounded CSV records for a read-only table; source bytes are never rewritten.
use anyhow::{Result, ensure};
pub fn parse(text: &str) -> Result<Vec<Vec<String>>> {
    parse_delimited(text, b',')
}
pub fn parse_delimited(text: &str, delimiter: u8) -> Result<Vec<Vec<String>>> {
    ensure!(
        text.len() <= 16 * 1024 * 1024,
        "CSV table preview is limited to 16 MiB; use Source"
    );
    let mut reader = csv::ReaderBuilder::new()
        .delimiter(delimiter)
        .has_headers(false)
        .flexible(true)
        .from_reader(text.strip_prefix('\u{feff}').unwrap_or(text).as_bytes());
    let mut rows = Vec::new();
    let mut cells = 0;
    for record in reader.records() {
        let record = record?;
        ensure!(
            rows.len() < 100_000,
            "CSV table preview is limited to 100,000 rows; use Source"
        );
        ensure!(
            record.len() <= 256,
            "CSV table preview is limited to 256 columns; use Source"
        );
        cells += record.len();
        ensure!(
            cells <= 1_000_000,
            "CSV table preview is limited to 1,000,000 cells; use Source"
        );
        rows.push(record.iter().map(str::to_owned).collect());
    }
    Ok(rows)
}

#[cfg(test)]
mod tests {
    #[test]
    fn quoted_commas_newlines_bom_and_ragged_records() {
        let rows = super::parse("\u{feff}name,value\r\n中文,\"a,b\"\r\nquoted,\"line 1\nline 2 \"\"yes\"\"\"\r\nshort\r\n").unwrap();
        assert_eq!(rows[0], ["name", "value"]);
        assert_eq!(rows[1], ["中文", "a,b"]);
        assert_eq!(rows[2][1], "line 1\nline 2 \"yes\"");
        assert_eq!(rows[3], ["short"]);
    }
}
