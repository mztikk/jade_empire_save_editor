//! Reader for the `TNamed*` string tables of the game.
//!
//! Jade Empire ships its strings in `dialog.tlk` (and `dialogf.tlk` for the
//! female player character). The format handled here is `TLK V4.0`:
//!
//! ```text
//! 0x00  "TLK "
//! 0x04  "V4.0"
//! 0x08  u32 languageCount
//! 0x0c  u32 stringCount
//! 0x10  u32 entryTableOffset
//! 0x14  u32 stringDataOffset
//!       entries: 10 bytes each, (u32 flags, u32 offset, u16 length)
//!       string data: UTF-8, not NUL terminated
//! ```

use std::path::{Path, PathBuf};

use crate::error::{Error, Result};

const MAGIC: &[u8; 4] = b"TLK ";
const VERSION: &[u8; 4] = b"V4.0";
const HEADER_SIZE: usize = 24;
const ENTRY_SIZE: usize = 10;

/// A parsed talk table.
pub struct TalkTable {
    path: PathBuf,
    data: Vec<u8>,
    entries: Vec<(u32, u16)>,
}

impl TalkTable {
    /// Opens and indexes a talk table.
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref().to_path_buf();
        let data = std::fs::read(&path).map_err(|source| Error::Io {
            path: path.clone(),
            source,
        })?;
        Self::parse(path, data)
    }

    /// Parses a talk table from memory.
    pub fn parse(path: PathBuf, data: Vec<u8>) -> Result<Self> {
        if data.len() < HEADER_SIZE {
            return Err(Error::TalkTable(
                "file is shorter than the header".to_owned(),
            ));
        }
        if &data[..4] != MAGIC {
            return Err(Error::TalkTable(format!(
                "bad magic {:?}",
                String::from_utf8_lossy(&data[..4])
            )));
        }
        if &data[4..8] != VERSION {
            return Err(Error::TalkTable(format!(
                "unsupported version {:?}",
                String::from_utf8_lossy(&data[4..8])
            )));
        }
        let word = |offset: usize| -> Result<u32> {
            data.get(offset..offset + 4)
                .and_then(|slice| slice.try_into().ok())
                .map(u32::from_le_bytes)
                .ok_or_else(|| {
                    Error::TalkTable(format!("header word at offset {offset} is truncated"))
                })
        };
        let _languages = word(8)?;
        let count = word(12)? as usize;
        let table_offset = word(16)? as usize;
        let data_offset = word(20)? as usize;

        let table_end = table_offset
            .checked_add(count * ENTRY_SIZE)
            .ok_or_else(|| Error::TalkTable("entry table overflows".to_owned()))?;
        if table_end > data.len() || data_offset > data.len() {
            return Err(Error::TalkTable(
                "entry table or string data is out of bounds".to_owned(),
            ));
        }

        let mut entries = Vec::with_capacity(count);
        for index in 0..count {
            let start = table_offset + index * ENTRY_SIZE;
            let offset = word(start + 4)?;
            let length = data
                .get(start + 8..start + 10)
                .and_then(|slice| slice.try_into().ok())
                .map(u16::from_le_bytes)
                .ok_or_else(|| Error::TalkTable("entry table is truncated".to_owned()))?;
            entries.push((offset, length));
        }
        Ok(Self {
            path,
            data,
            entries,
        })
    }

    /// Number of strings in the table.
    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// True when the table has no strings at all.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Path of the file that was opened.
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Returns a string by its talk table reference.
    #[must_use]
    pub fn get(&self, strref: u32) -> Option<String> {
        let (offset, length) = *self.entries.get(strref as usize)?;
        let start = offset as usize;
        let bytes = self.data.get(start..start + length as usize)?;
        Some(String::from_utf8_lossy(bytes).into_owned())
    }

    /// Returns the raw bytes of a string, useful to detect encoding problems.
    #[must_use]
    pub fn get_bytes(&self, strref: u32) -> Option<&[u8]> {
        let (offset, length) = *self.entries.get(strref as usize)?;
        let start = offset as usize;
        self.data.get(start..start + length as usize)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample(strings: &[&str]) -> TalkTable {
        let mut entries = Vec::new();
        let mut data = Vec::new();
        let base = HEADER_SIZE + strings.len() * ENTRY_SIZE;
        for text in strings {
            entries.push((base + data.len(), text.len() as u16));
            data.extend_from_slice(text.as_bytes());
        }
        let mut file = Vec::new();
        file.extend_from_slice(b"TLK V4.0");
        file.extend_from_slice(&2_u32.to_le_bytes());
        file.extend_from_slice(&(strings.len() as u32).to_le_bytes());
        file.extend_from_slice(&(HEADER_SIZE as u32).to_le_bytes());
        file.extend_from_slice(&(base as u32).to_le_bytes());
        for (offset, length) in entries {
            file.extend_from_slice(&u32::MAX.to_le_bytes());
            file.extend_from_slice(&u32::try_from(offset).expect("fits").to_le_bytes());
            file.extend_from_slice(&length.to_le_bytes());
        }
        file.extend_from_slice(&data);
        TalkTable::parse(PathBuf::from("test.tlk"), file).expect("parse")
    }

    #[test]
    fn reads_strings() {
        let table = sample(&["hello", "Welt", ""]);
        assert_eq!(table.get(0).as_deref(), Some("hello"));
        assert_eq!(table.get(1).as_deref(), Some("Welt"));
        assert_eq!(table.get(2).as_deref(), Some(""));
        assert_eq!(table.get(3), None);
    }

    #[test]
    fn rejects_unknown_version() {
        let mut file = Vec::new();
        file.extend_from_slice(b"TLK V3.0");
        file.resize(64, 0);
        assert!(TalkTable::parse(PathBuf::from("test.tlk"), file).is_err());
    }
}
