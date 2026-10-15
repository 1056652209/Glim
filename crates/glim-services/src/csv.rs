//! Bounded CSV records for a read-only table; source bytes are never rewritten.
use crate::limits as reader_limits;
use anyhow::Result;
pub fn parse(text: &str) -> Result<Vec<Vec<String>>> {
    parse_delimited(text, b',')
}
pub fn parse_delimited(text: &str, delimiter: u8) -> Result<Vec<Vec<String>>> {
    csv::ReaderBuilder::new()
        .delimiter(delimiter)
        .has_headers(false)
        .flexible(true)
        .from_reader(text.strip_prefix('\u{feff}').unwrap_or(text).as_bytes())
        .records()
        .map(|record| Ok(record?.iter().map(str::to_owned).collect()))
        .collect()
}

pub struct TablePage {
    pub rows: Vec<Vec<String>>,
    pub columns: usize,
    pub more: bool,
}
struct RecordReader<R> {
    inner: R,
    remaining: usize,
    cancel: crate::work::Cancellation,
}
impl<R: std::io::Read> std::io::Read for RecordReader<R> {
    fn read(&mut self, bytes: &mut [u8]) -> std::io::Result<usize> {
        self.cancel.check().map_err(std::io::Error::other)?;
        if self.remaining == 0 {
            return Err(std::io::Error::other(
                "CSV record exceeds the active table budget; use Source to read or edit the complete value.",
            ));
        }
        let size = bytes.len().min(self.remaining);
        let read = self.inner.read(&mut bytes[..size])?;
        self.remaining -= read;
        Ok(read)
    }
}
pub fn read_page(
    input: impl std::io::Read,
    delimiter: u8,
    row_start: usize,
    column_start: usize,
    cancel: &crate::work::Cancellation,
) -> Result<TablePage> {
    let input = RecordReader {
        inner: input,
        remaining: reader_limits::CSV_INPUT_BYTES,
        cancel: cancel.clone(),
    };
    let mut reader = csv::ReaderBuilder::new()
        .delimiter(delimiter)
        .has_headers(false)
        .flexible(true)
        .buffer_capacity(crate::limits::IO_BUFFER_BYTES)
        .from_reader(input);
    let mut record = csv::StringRecord::new();
    let mut rows = Vec::new();
    let mut columns = 0;
    let mut bytes = 0;
    let mut cells = 0;
    let mut index = 0;
    while reader.read_record(&mut record)? {
        cancel.check()?;
        reader.get_mut().remaining = reader_limits::CSV_INPUT_BYTES;
        if index < row_start {
            index += 1;
            continue;
        }
        columns = columns.max(record.len());
        let values = record
            .iter()
            .skip(column_start)
            .take(reader_limits::CSV_COLUMNS);
        let row_bytes = values.clone().map(str::len).sum::<usize>();
        let row_cells = values.clone().count();
        if !rows.is_empty()
            && (rows.len() >= reader_limits::CSV_ROWS
                || bytes + row_bytes > reader_limits::CSV_INPUT_BYTES
                || cells + row_cells > reader_limits::CSV_CELLS)
        {
            return Ok(TablePage {
                rows,
                columns,
                more: true,
            });
        }
        bytes += row_bytes;
        cells += row_cells;
        rows.push(values.map(str::to_owned).collect());
        index += 1;
    }
    Ok(TablePage {
        rows,
        columns,
        more: false,
    })
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
