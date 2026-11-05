//! Reader for `BioWare`'s text based 2DA tables.
//!
//! ```text
//! 2DA V2.0
//! <default values>
//! LABEL   TYPE    ID
//! 0       Sword   weapons
//! 1       ****    gems
//! ```
//!
//! Cells contain `****` when they have no value. Values may be quoted when they
//! contain spaces. The row number in front of each line is the row id; rows do
//! not have to be contiguous.

use std::collections::HashMap;

use crate::error::{Error, Result};

/// Hard cap on a 2DA row number.
///
/// Row labels are untrusted 64 bit numbers in the text format. Without a bound
/// a two line table can ask for a multi-gigabyte `Vec`.
const MAX_ROW_INDEX: usize = 100_000;

/// One row of a 2DA table.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Row {
    values: Vec<Option<String>>,
}

impl Row {
    /// Raw cell value, `None` for `****`.
    #[must_use]
    pub fn cell(&self, column: usize) -> Option<&str> {
        self.values.get(column)?.as_deref()
    }

    /// Cell value parsed as an unsigned integer.
    #[must_use]
    pub fn uint(&self, column: usize) -> Option<u64> {
        self.cell(column)?.trim().parse().ok()
    }

    /// Cell value parsed as a 32 bit unsigned integer.
    #[must_use]
    pub fn u32(&self, column: usize) -> Option<u32> {
        self.uint(column)
            .and_then(|value| u32::try_from(value).ok())
    }

    /// Cell value parsed as an integer.
    #[must_use]
    pub fn int(&self, column: usize) -> Option<i64> {
        self.cell(column)?.trim().parse().ok()
    }

    /// Cell value parsed as a float.
    #[must_use]
    pub fn float(&self, column: usize) -> Option<f32> {
        self.cell(column)?.trim().parse().ok()
    }

    /// True when the cell is empty or `****`.
    #[must_use]
    pub fn is_empty(&self, column: usize) -> bool {
        self.cell(column).is_none_or(str::is_empty)
    }
}

/// A parsed 2DA table.
#[derive(Debug, Clone, Default)]
pub struct TwoDa {
    columns: Vec<String>,
    column_index: HashMap<String, usize>,
    rows: Vec<Option<Row>>,
}

impl TwoDa {
    /// Parses a binary `2DA V2.b` table as shipped inside the game archives.
    ///
    /// Layout: signature, tab separated column headers, the row count, tab
    /// separated row labels, one 16 bit offset per cell (row major), the cell
    /// data size and finally a NUL separated string pool the offsets point
    /// into.
    ///
    /// # Errors
    ///
    /// Returns [`Error::TwoDa`] when the signature or the structure is invalid.
    pub fn parse_binary(data: &[u8]) -> Result<Self> {
        if data.len() < 10 || &data[..9] != b"2DA V2.b\n" {
            return Err(Error::TwoDa(
                "not a 2DA V2.b table (bad signature or version)".to_owned(),
            ));
        }
        let mut position = 9;
        let mut columns: Vec<String> = Vec::new();
        while position < data.len() && data[position] != 0 {
            let end = find_byte(data, position, b'\t')?;
            columns.push(
                std::str::from_utf8(&data[position..end])
                    .map_err(|error| Error::TwoDa(format!("invalid column name: {error}")))?
                    .to_owned(),
            );
            position = end + 1;
        }
        if columns.is_empty() {
            return Err(Error::TwoDa("binary 2DA has no columns".to_owned()));
        }
        position += 1;
        let row_count = read_u32(data, position)? as usize;
        position += 4;
        validate_row_count(row_count, data.len().saturating_sub(position))?;
        let mut labels: Vec<String> = Vec::with_capacity(row_count);
        for _ in 0..row_count {
            let end = find_byte(data, position, b'\t')?;
            labels.push(
                std::str::from_utf8(&data[position..end])
                    .map_err(|error| Error::TwoDa(format!("invalid row label: {error}")))?
                    .to_owned(),
            );
            position = end + 1;
        }
        let cell_count = row_count
            .checked_mul(columns.len())
            .ok_or_else(|| Error::TwoDa("2DA cell count overflows".to_owned()))?;
        if cell_count > data.len() / 2 {
            return Err(Error::TwoDa(format!(
                "{cell_count} cells do not fit in the {} byte table",
                data.len()
            )));
        }
        let mut offsets: Vec<u16> = Vec::with_capacity(cell_count);
        for _ in 0..cell_count {
            offsets.push(read_u16(data, position)?);
            position += 2;
        }
        let cells_size = usize::from(read_u16(data, position)?);
        position += 2;
        let cell_base = position;
        let cell_end = cell_base
            .checked_add(cells_size)
            .ok_or_else(|| Error::TwoDa("2DA string pool overflows".to_owned()))?;
        if cell_end > data.len() {
            return Err(Error::TwoDa(format!(
                "2DA string pool ends at {cell_end}, beyond the {} byte table",
                data.len()
            )));
        }

        let mut column_index = HashMap::new();
        for (index, column) in columns.iter().enumerate() {
            column_index.insert(column.to_lowercase(), index);
        }
        let mut table = Self {
            columns,
            column_index,
            rows: Vec::new(),
        };
        for (row_number, label) in labels.iter().enumerate() {
            let width = table.columns.len();
            let mut values: Vec<Option<String>> = Vec::with_capacity(width);
            for column in 0..width {
                let offset = usize::from(offsets[row_number * width + column]);
                values.push(binary_cell(data, cell_base, cells_size, offset)?);
            }
            let row = Row { values };
            let index = match label.parse::<usize>() {
                Ok(index) => {
                    if index > MAX_ROW_INDEX {
                        return Err(Error::TwoDa(format!(
                            "row label {index} is above the {MAX_ROW_INDEX} row limit"
                        )));
                    }
                    index
                }
                Err(_) => row_number,
            };
            if table.rows.len() <= index {
                table.rows.resize(index + 1, None);
            }
            table.rows[index] = Some(row);
        }
        Ok(table)
    }

    /// Parses a text 2DA file.
    pub fn parse(text: &str) -> Result<Self> {
        let mut lines = text.lines();
        let signature = lines.next().unwrap_or_default().trim();
        let mut signature_parts = signature.split_whitespace();
        let magic = signature_parts.next().unwrap_or_default();
        if magic != "2DA" {
            return Err(Error::TwoDa(format!(
                "missing 2DA signature, found {signature:?}"
            )));
        }
        match signature_parts.next() {
            Some("V2.0") => {}
            Some("V2.b") => {
                return Err(Error::TwoDa(
                    "binary 2DA tables are not supported".to_owned(),
                ));
            }
            Some(version) => {
                return Err(Error::TwoDa(format!("unsupported 2DA version {version:?}")));
            }
            None => return Err(Error::TwoDa("2DA version is missing".to_owned())),
        }

        // The second line holds default values that we do not need, the third
        // line holds the column names.
        let _defaults = lines.next();
        let header_line = lines
            .next()
            .ok_or_else(|| Error::TwoDa("table ends before the column header".to_owned()))?;
        let columns = tokenize(header_line);
        if columns.is_empty() {
            return Err(Error::TwoDa("table has no columns".to_owned()));
        }
        let mut column_index = HashMap::with_capacity(columns.len());
        for (index, name) in columns.iter().enumerate() {
            column_index.insert(name.to_lowercase(), index);
        }

        let mut rows: Vec<Option<Row>> = Vec::new();
        for line in lines {
            let tokens = tokenize(line);
            if tokens.is_empty() {
                continue;
            }
            let index: usize = match tokens[0].parse() {
                Ok(index) if index <= MAX_ROW_INDEX => index,
                Ok(index) => {
                    return Err(Error::TwoDa(format!(
                        "row label {index} is above the {MAX_ROW_INDEX} row limit"
                    )));
                }
                Err(_) => continue,
            };
            let mut values: Vec<Option<String>> = tokens
                .into_iter()
                .skip(1)
                .map(|token| (token != "****").then_some(token))
                .collect();
            values.resize(columns.len(), None);
            if rows.len() <= index {
                rows.resize(index + 1, None);
            }
            rows[index] = Some(Row { values });
        }

        Ok(Self {
            columns,
            column_index,
            rows,
        })
    }

    /// Column names in file order.
    #[must_use]
    pub fn columns(&self) -> &[String] {
        &self.columns
    }

    /// Number of rows including holes.
    #[must_use]
    pub fn len(&self) -> usize {
        self.rows.len()
    }

    /// True when the table has no rows.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.rows.iter().all(Option::is_none)
    }

    /// Looks up a column by name (case insensitive).
    #[must_use]
    pub fn column(&self, name: &str) -> Option<usize> {
        self.column_index.get(&name.to_lowercase()).copied()
    }

    /// Looks up a row by index.
    #[must_use]
    pub fn row(&self, index: usize) -> Option<&Row> {
        self.rows.get(index)?.as_ref()
    }

    /// Looks up a cell by row index and column name.
    #[must_use]
    pub fn cell(&self, row: usize, column: &str) -> Option<&str> {
        self.row(row)?.cell(self.column(column)?)
    }

    /// Parses a cell as an unsigned integer.
    #[must_use]
    pub fn uint(&self, row: usize, column: &str) -> Option<u64> {
        self.row(row)?.uint(self.column(column)?)
    }

    /// Iterates over all present rows.
    pub fn iter(&self) -> impl Iterator<Item = (usize, &Row)> {
        self.rows
            .iter()
            .enumerate()
            .filter_map(|(index, row)| row.as_ref().map(|row| (index, row)))
    }
}

/// Finds the next occurrence of `needle` at or after `from`.
fn find_byte(data: &[u8], from: usize, needle: u8) -> Result<usize> {
    data.get(from..)
        .and_then(|tail| tail.iter().position(|byte| *byte == needle))
        .map(|offset| from + offset)
        .ok_or_else(|| Error::TwoDa("truncated binary 2DA table".to_owned()))
}

fn read_u16(data: &[u8], position: usize) -> Result<u16> {
    let bytes = data
        .get(position..position + 2)
        .ok_or_else(|| Error::TwoDa("truncated binary 2DA table".to_owned()))?;
    Ok(u16::from_le_bytes([bytes[0], bytes[1]]))
}

fn read_u32(data: &[u8], position: usize) -> Result<u32> {
    let bytes = data
        .get(position..position + 4)
        .ok_or_else(|| Error::TwoDa("truncated binary 2DA table".to_owned()))?;
    Ok(u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
}

/// Reads a NUL terminated cell from the declared string pool.
fn binary_cell(
    data: &[u8],
    pool_start: usize,
    pool_size: usize,
    offset: usize,
) -> Result<Option<String>> {
    if offset >= pool_size {
        return Err(Error::TwoDa(format!(
            "cell offset {offset} is outside the {pool_size} byte string pool"
        )));
    }
    let start = pool_start + offset;
    let end = start
        .checked_add(pool_size - offset)
        .ok_or_else(|| Error::TwoDa("cell end overflows".to_owned()))?;
    let pool = &data[start..end];
    let Some(length) = pool.iter().position(|byte| *byte == 0) else {
        return Err(Error::TwoDa(format!(
            "cell at offset {offset} is not NUL terminated in its string pool"
        )));
    };
    let text = std::str::from_utf8(&pool[..length])
        .map_err(|error| Error::TwoDa(format!("cell at offset {offset} is not UTF-8: {error}")))?;
    if text.is_empty() || text == "****" {
        Ok(None)
    } else {
        Ok(Some(text.to_owned()))
    }
}

/// Validates a binary header's row count against the remaining bytes.
fn validate_row_count(row_count: usize, remaining: usize) -> Result<()> {
    if row_count > MAX_ROW_INDEX {
        return Err(Error::TwoDa(format!(
            "binary 2DA row count {row_count} is above the {MAX_ROW_INDEX} row limit"
        )));
    }
    if row_count > remaining {
        return Err(Error::TwoDa(format!(
            "binary 2DA row count {row_count} does not fit in {remaining} remaining bytes"
        )));
    }
    Ok(())
}

/// Splits a 2DA line into tokens, honouring double quotes.
fn tokenize(line: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut current = String::new();
    let mut quoted = false;
    let mut in_token = false;
    for character in line.chars() {
        match character {
            '"' => {
                quoted = !quoted;
                in_token = true;
            }
            c if c.is_whitespace() && !quoted => {
                if in_token {
                    tokens.push(std::mem::take(&mut current));
                    in_token = false;
                }
            }
            c => {
                current.push(c);
                in_token = true;
            }
        }
    }
    if in_token {
        tokens.push(current);
    }
    tokens
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = "2DA V2.0\n\nLABEL TYPE ID Name\n0 Sword weapons 7 \"Long sword\"\n1 **** gems **** Short\n";

    #[test]
    fn parses_columns_and_rows() {
        let table = TwoDa::parse(SAMPLE).expect("parse");
        assert_eq!(table.columns(), &["LABEL", "TYPE", "ID", "Name"]);
        assert_eq!(table.cell(0, "LABEL"), Some("Sword"));
        assert_eq!(table.cell(0, "Name"), Some("Long sword"));
        assert_eq!(table.cell(1, "LABEL"), None);
        assert_eq!(table.uint(0, "ID"), Some(7));
        assert_eq!(table.cell(1, "Name"), Some("Short"));
    }

    #[test]
    fn rejects_binary_tables() {
        assert!(TwoDa::parse("2DA V2.b\n\nA B\n").is_err());
    }

    #[test]
    fn rejects_missing_signature() {
        assert!(TwoDa::parse("hello world").is_err());
    }

    #[test]
    fn tokenizer_handles_quotes() {
        assert_eq!(
            tokenize("0  \"two words\"  three"),
            vec!["0", "two words", "three"]
        );
    }

    #[test]
    fn row_accessors_handle_types_and_empty_cells() {
        let table = TwoDa::parse(
            "2DA V2.0\n\nA B C D\n0 10 -3 1.5 \"\"\n1 4294967296 9223372036854775808 x\n",
        )
        .expect("parse");
        let row = table.row(0).expect("row");
        assert_eq!(row.uint(0), Some(10));
        assert_eq!(row.u32(0), Some(10));
        assert_eq!(row.int(1), Some(-3));
        assert_eq!(row.float(2), Some(1.5));
        assert_eq!(row.cell(3), Some(""));
        assert!(row.is_empty(3));
        assert!(row.is_empty(9));
        assert_eq!(row.cell(9), None);

        let row = table.row(1).expect("row");
        assert_eq!(row.uint(0), Some(4_294_967_296));
        assert_eq!(row.u32(0), None);
        assert_eq!(row.int(1), None);
        assert_eq!(row.float(2), None);
        assert!(!row.is_empty(0));
    }

    #[test]
    fn parser_skips_blank_and_non_numeric_rows() {
        let table =
            TwoDa::parse("2DA V2.0\n\nA B\n0 one 1\n\nnot-a-row 2\n1 two 2\n").expect("parse");
        assert_eq!(table.len(), 2);
        assert_eq!(table.cell(0, "A"), Some("one"));
        assert_eq!(table.cell(1, "A"), Some("two"));
    }

    #[test]
    fn parser_rejects_an_empty_header() {
        assert!(TwoDa::parse("2DA V2.0\n\n\n").is_err());
    }

    #[test]
    fn parser_rejects_hostile_row_numbers_and_versions() {
        assert!(TwoDa::parse("2DA V2.0\n\nA\n18446744073709551615 x\n").is_err());
        assert!(TwoDa::parse("2DA V2.0\n\nA\n4294967295 x\n").is_err());
        assert!(TwoDa::parse("2DA V1.0\n\nA\n0 x\n").is_err());
    }
}

#[cfg(test)]
mod binary_tests {
    use super::*;

    /// Builds a one row, two column binary table: `0` with cells `sword`, `5`.
    fn sample() -> Vec<u8> {
        let mut data = Vec::new();
        data.extend_from_slice(b"2DA V2.b\n");
        data.extend_from_slice(b"label\tcost\t");
        data.push(0);
        data.extend_from_slice(&1_u32.to_le_bytes());
        data.extend_from_slice(b"0\t");
        data.extend_from_slice(&0_u16.to_le_bytes());
        data.extend_from_slice(&6_u16.to_le_bytes());
        data.extend_from_slice(&8_u16.to_le_bytes());
        data.extend_from_slice(b"sword\0");
        data.extend_from_slice(b"5\0");
        data
    }

    #[test]
    fn reads_a_binary_table() {
        let table = TwoDa::parse_binary(&sample()).expect("parse");
        assert_eq!(table.column("Label"), Some(0));
        assert_eq!(table.cell(0, "Label"), Some("sword"));
        assert_eq!(table.uint(0, "cost"), Some(5));
    }

    #[test]
    fn rejects_foreign_data() {
        assert!(TwoDa::parse_binary(b"2DA V2.0\n").is_err());
        assert!(TwoDa::parse_binary(b"nonsense").is_err());
    }

    #[test]
    fn rejects_hostile_counts_and_out_of_pool_offsets() {
        let mut data = b"2DA V2.b\nlabel\t".to_vec();
        data.push(0);
        data.extend_from_slice(&u32::MAX.to_le_bytes());
        assert!(TwoDa::parse_binary(&data).is_err());

        let mut sample = sample();
        // The first cell offset sits directly after the row label.
        let offset = sample.len() - 10;
        sample[offset..offset + 2].copy_from_slice(&99_u16.to_le_bytes());
        assert!(TwoDa::parse_binary(&sample).is_err());
    }
}
