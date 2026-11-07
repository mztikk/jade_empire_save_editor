//! Reader for the RIM archives that ship the global game tables.
//!
//! Jade Empire keeps most of its 2DA tables inside `data/miniglobal-a.rim`
//! (and the style animation tables in `data/globalunload-a.rim`), not in the
//! `override` folder. Files in `override` take precedence over the archive, the
//! same way the engine loads them.
//!
//! ```text
//! "RIM " "V1.0"  u32 unknown  u32 entryCount  u32 entryTableOffset
//! entry (32 bytes): name[16]  u32 type  u32 id  u32 offset  u32 size
//! ```

use std::path::{Path, PathBuf};

use crate::error::{Error, Result};

/// Resource type of a 2DA table inside a RIM archive.
pub const TYPE_2DA: u32 = 2017;

/// One resource stored in a RIM archive.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RimEntry {
    /// Resource name, lower case.
    pub name: String,
    /// Resource type id.
    pub kind: u32,
    /// Offset of the payload from the start of the archive.
    pub offset: u32,
    /// Payload size in bytes.
    pub size: u32,
}

/// A parsed RIM archive with its payload kept in memory.
#[derive(Debug, Clone)]
pub struct RimArchive {
    path: PathBuf,
    data: Vec<u8>,
    entries: Vec<RimEntry>,
}

impl RimArchive {
    /// Reads and indexes a RIM archive.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Archive`] when the signature is wrong and
    /// [`Error::Io`] when the file cannot be read.
    pub fn open(path: impl Into<PathBuf>) -> Result<Self> {
        let path = path.into();
        let data = std::fs::read(&path).map_err(|source| Error::Io {
            path: path.clone(),
            source,
        })?;
        if data.len() < 20 || &data[..8] != b"RIM V1.0" {
            return Err(Error::Archive(format!(
                "{} is not a RIM V1.0 archive",
                path.display()
            )));
        }
        let count = u32::from_le_bytes([data[12], data[13], data[14], data[15]]) as usize;
        let table = u32::from_le_bytes([data[16], data[17], data[18], data[19]]) as usize;
        if table < 20 {
            return Err(Error::Archive(format!(
                "{} has an entry table inside its header",
                path.display()
            )));
        }
        let table_size = count
            .checked_mul(32)
            .ok_or_else(|| Error::Archive(format!("{} has too many entries", path.display())))?;
        let table_end = table
            .checked_add(table_size)
            .ok_or_else(|| Error::Archive(format!("{} entry table overflows", path.display())))?;
        if table_end > data.len() {
            return Err(Error::Archive(format!(
                "{} ends in the middle of its entry table",
                path.display()
            )));
        }
        let mut entries = Vec::with_capacity(count);
        for index in 0..count {
            let base = table + index * 32;
            let Some(raw) = data.get(base..base + 32) else {
                return Err(Error::Archive(format!(
                    "{} ends in the middle of its entry table",
                    path.display()
                )));
            };
            let name = raw[..16]
                .split(|byte| *byte == 0)
                .next()
                .unwrap_or_default();
            let name = String::from_utf8_lossy(name).to_lowercase();
            let kind = u32::from_le_bytes([raw[16], raw[17], raw[18], raw[19]]);
            let offset = u32::from_le_bytes([raw[24], raw[25], raw[26], raw[27]]);
            let size = u32::from_le_bytes([raw[28], raw[29], raw[30], raw[31]]);
            let payload_start = offset as usize;
            let payload_size = size as usize;
            let payload_end = payload_start
                .checked_add(payload_size)
                .ok_or_else(|| Error::Archive(format!("entry {index} payload overflows")))?;
            if payload_end > data.len() || (payload_size > 0 && payload_start < table_end) {
                return Err(Error::Archive(format!(
                    "entry {index} payload points outside the archive or into its table"
                )));
            }
            entries.push(RimEntry {
                name,
                kind,
                offset,
                size,
            });
        }
        Ok(Self {
            path,
            data,
            entries,
        })
    }

    /// Path the archive was read from.
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// All indexed resources.
    #[must_use]
    pub fn entries(&self) -> &[RimEntry] {
        &self.entries
    }

    /// Resource names, lower case, in archive order.
    pub fn names(&self) -> impl Iterator<Item = &str> {
        self.entries.iter().map(|entry| entry.name.as_str())
    }

    /// Looks a resource up by name (case insensitive).
    #[must_use]
    pub fn get(&self, name: &str) -> Option<&[u8]> {
        let name = name.to_lowercase();
        let entry = self.entries.iter().find(|entry| entry.name == name)?;
        let start = entry.offset as usize;
        let end = start.checked_add(entry.size as usize)?;
        self.data.get(start..end)
    }

    /// Looks a 2DA table up by name (case insensitive).
    #[must_use]
    pub fn get_2da(&self, name: &str) -> Option<&[u8]> {
        let name = name.to_lowercase();
        let entry = self
            .entries
            .iter()
            .find(|entry| entry.name == name && entry.kind == TYPE_2DA)?;
        let start = entry.offset as usize;
        let end = start.checked_add(entry.size as usize)?;
        self.data.get(start..end)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_foreign_files() {
        let path = std::env::temp_dir().join(format!("jadeforge-rim-{}", std::process::id()));
        std::fs::write(&path, b"not an archive").expect("write");
        let error = RimArchive::open(&path).expect_err("must fail");
        assert!(error.to_string().contains("RIM"));
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn rejects_hostile_entry_counts_and_versions() {
        let path = std::env::temp_dir().join(format!("jadeforge-rim-count-{}", std::process::id()));
        let mut data = Vec::new();
        data.extend_from_slice(b"RIM V1.0");
        data.extend_from_slice(&0_u32.to_le_bytes());
        data.extend_from_slice(&u32::MAX.to_le_bytes());
        data.extend_from_slice(&20_u32.to_le_bytes());
        std::fs::write(&path, &data).expect("write");
        assert!(RimArchive::open(&path).is_err());

        let mut old = b"RIM V2.0".to_vec();
        old.resize(20, 0);
        std::fs::write(&path, &old).expect("write");
        assert!(RimArchive::open(&path).is_err());
        let _ = std::fs::remove_file(path);
    }
}
