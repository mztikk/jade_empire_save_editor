#![allow(dead_code)] // each game-data test binary uses a different subset
#![allow(clippy::expect_used)] // fixtures fail loudly; expects are assertions

//! Tiny synthetic game installations for the game-data tests.

use std::path::{Path, PathBuf};

/// A temporary game root that removes itself when the test ends.
pub struct TempDir {
    path: PathBuf,
}

impl TempDir {
    pub fn new(prefix: &str) -> Self {
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |duration| duration.as_nanos());
        let path = std::env::temp_dir().join(format!(
            "jadeforge-gamedata-{prefix}-{}-{stamp}",
            std::process::id()
        ));
        std::fs::create_dir_all(&path).expect("create temp directory");
        Self { path }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn join(&self, name: &str) -> PathBuf {
        self.path.join(name)
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.path);
    }
}

/// Writes a file, creating its parent directories.
pub fn write(path: &Path, bytes: &[u8]) {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).expect("create parent directory");
    }
    std::fs::write(path, bytes).expect("write fixture file");
}

/// Writes a UTF-8 text file, creating its parent directories.
pub fn write_text(path: &Path, text: &str) {
    write(path, text.as_bytes());
}

/// Writes a `TLK V4.0` talk table with the given strings.
pub fn write_tlk(path: &Path, strings: &[&str]) {
    let mut data = Vec::new();
    data.extend_from_slice(b"TLK V4.0");
    data.extend_from_slice(&2_u32.to_le_bytes());
    data.extend_from_slice(&(strings.len() as u32).to_le_bytes());
    data.extend_from_slice(&24_u32.to_le_bytes());
    let data_offset = 24 + 10 * strings.len();
    data.extend_from_slice(&(data_offset as u32).to_le_bytes());
    let mut text = Vec::new();
    for string in strings {
        let bytes = string.as_bytes();
        data.extend_from_slice(&0_u32.to_le_bytes());
        // Entries store absolute file offsets.
        data.extend_from_slice(&((data_offset + text.len()) as u32).to_le_bytes());
        data.extend_from_slice(&(bytes.len() as u16).to_le_bytes());
        text.extend_from_slice(bytes);
    }
    data.extend_from_slice(&text);
    write(path, &data);
}

/// Builds a binary `2DA V2.b` table with tab separated column names.
///
/// Cells are laid out row major. Empty strings and `****` become `None` in the
/// parser, exactly like the text format.
pub fn binary_2da(columns: &[&str], rows: &[(&str, Vec<&str>)]) -> Vec<u8> {
    let mut data = Vec::new();
    data.extend_from_slice(b"2DA V2.b\n");
    for column in columns {
        data.extend_from_slice(column.as_bytes());
        data.push(b'\t');
    }
    data.push(0);
    data.extend_from_slice(&u32::try_from(rows.len()).expect("row count").to_le_bytes());
    for (label, _) in rows {
        data.extend_from_slice(label.as_bytes());
        data.push(b'\t');
    }

    let mut pool = Vec::new();
    let mut offsets = Vec::new();
    for (_, cells) in rows {
        for column in 0..columns.len() {
            let cell = cells.get(column).copied().unwrap_or("");
            offsets.push(u16::try_from(pool.len()).expect("cell offset"));
            pool.extend_from_slice(cell.as_bytes());
            pool.push(0);
        }
    }
    for offset in offsets {
        data.extend_from_slice(&offset.to_le_bytes());
    }
    data.extend_from_slice(&u16::try_from(pool.len()).expect("pool size").to_le_bytes());
    data.extend_from_slice(&pool);
    data
}

/// Builds a `RIM V1.0` archive whose payloads follow the entry table.
pub fn rim(entries: &[(&str, u32, &[u8])]) -> Vec<u8> {
    let table_offset = 20_u32;
    let payload_start = table_offset as usize + entries.len() * 32;
    let mut bytes = Vec::new();
    bytes.extend_from_slice(b"RIM V1.0");
    bytes.extend_from_slice(&0_u32.to_le_bytes());
    bytes.extend_from_slice(&(entries.len() as u32).to_le_bytes());
    bytes.extend_from_slice(&table_offset.to_le_bytes());

    let mut payloads = Vec::new();
    let mut offset = payload_start;
    for (name, kind, payload) in entries {
        let mut entry = [0_u8; 32];
        let name = name.as_bytes();
        entry[..name.len()].copy_from_slice(name);
        entry[16..20].copy_from_slice(&kind.to_le_bytes());
        entry[20..24].copy_from_slice(&1_u32.to_le_bytes());
        entry[24..28]
            .copy_from_slice(&u32::try_from(offset).expect("payload offset").to_le_bytes());
        entry[28..32].copy_from_slice(
            &u32::try_from(payload.len())
                .expect("payload size")
                .to_le_bytes(),
        );
        bytes.extend_from_slice(&entry);
        payloads.extend_from_slice(payload);
        offset += payload.len();
    }
    bytes.extend_from_slice(&payloads);
    bytes
}
