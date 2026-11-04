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
        if signature_parts.next() == Some("V2.b") {
            return Err(Error::TwoDa(
                "binary 2DA tables are not supported".to_owned(),
            ));
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
                Ok(index) => index,
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
}
