//! The savegame container: a sequence of named GFF blocks.
//!
//! ```text
//! u32 blockCount
//! repeat blockCount times:
//!     u32 nameLength   (including the trailing NUL)
//!     u8  name[nameLength]
//!     u32 blockSize
//!     u8  block[blockSize]   ("SAV V3.2" GFF)
//! ```

use std::fs;
use std::io::Write as _;
use std::ops::Range;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::error::{Error, Result};
use crate::gff::Gff;

/// Name of the block that stores the global game state and the player creature.
pub const GLOBAL_BLOCK: &str = "save.sav";
/// Name of the block used by older Jade Empire releases for character sheets.
pub const POLYPLAYER_BLOCK: &str = "polyplayer.sav";
/// Largest accepted block name, including the trailing NUL.
const MAX_NAME_LEN: usize = 260;

/// A single block of a savegame.
#[derive(Debug, Clone)]
pub struct Block {
    /// Resource name, for example `save.sav`.
    pub name: String,
    gff: Gff,
    span: Range<usize>,
    modified: bool,
}

impl Block {
    /// Name without the `.sav` suffix.
    #[must_use]
    pub fn stem(&self) -> &str {
        self.name.strip_suffix(".sav").unwrap_or(&self.name)
    }

    /// True when the block was edited since it was loaded.
    #[must_use]
    pub fn is_modified(&self) -> bool {
        self.modified
    }

    /// Marks the block as modified, so that the next save rewrites it.
    pub fn mark_modified(&mut self) {
        self.modified = true;
    }

    /// The parsed GFF tree.
    #[must_use]
    pub fn gff(&self) -> &Gff {
        &self.gff
    }

    /// Mutable access to the GFF tree; marks the block as modified.
    pub fn gff_mut(&mut self) -> &mut Gff {
        self.modified = true;
        &mut self.gff
    }

    /// Drops the modified flag.
    pub fn mark_clean(&mut self) {
        self.modified = false;
    }
}

/// Options controlling how a savegame is written.
#[derive(Debug, Clone, Copy)]
pub struct SaveOptions {
    /// Copy the previous file next to the new one before overwriting it.
    pub backup: bool,
}

impl Default for SaveOptions {
    fn default() -> Self {
        Self { backup: true }
    }
}

/// A loaded Jade Empire savegame.
#[derive(Debug, Clone)]
pub struct SaveGame {
    path: PathBuf,
    bytes: Vec<u8>,
    blocks: Vec<Block>,
}

impl SaveGame {
    /// Loads a savegame from disk.
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        let bytes = fs::read(path).map_err(|error| Error::io(path, error))?;
        Self::parse(path.to_path_buf(), bytes)
    }

    /// Loads a savegame from memory. `path` is only used for error messages and
    /// as the default target for [`SaveGame::save`].
    pub fn parse(path: PathBuf, bytes: Vec<u8>) -> Result<Self> {
        let mut reader = Reader {
            bytes: &bytes,
            position: 0,
        };
        let count = reader.u32("block count")? as usize;
        if count == 0 {
            return Err(Error::NotASavegame(
                "the container does not contain any block".to_owned(),
            ));
        }
        if count > 4096 {
            return Err(Error::Corrupt(format!(
                "implausible block count {count}; this is probably not a Jade Empire savegame"
            )));
        }

        let mut blocks = Vec::with_capacity(count);
        for index in 0..count {
            let name_length = reader.u32("block name length")? as usize;
            if name_length == 0 || name_length > MAX_NAME_LEN {
                return Err(Error::Corrupt(format!(
                    "block {index} has an invalid name length of {name_length}"
                )));
            }
            let raw_name = reader.bytes(name_length, "block name")?;
            let name = std::str::from_utf8(&raw_name[..raw_name.len().saturating_sub(1)])
                .map_err(|_| Error::Corrupt(format!("block {index} has a non UTF-8 name")))?
                .to_owned();
            let size = reader.u32("block size")? as usize;
            let start = reader.position;
            let payload = reader.bytes(size, "block payload")?;
            let gff = Gff::parse(&name, payload)?;
            blocks.push(Block {
                name,
                gff,
                span: start..start + size,
                modified: false,
            });
        }
        if reader.position != bytes.len() {
            return Err(Error::Corrupt(format!(
                "{} trailing bytes after the last block",
                bytes.len() - reader.position
            )));
        }

        Ok(Self {
            path,
            bytes,
            blocks,
        })
    }

    /// Path this savegame was loaded from.
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// All blocks in file order.
    #[must_use]
    pub fn blocks(&self) -> &[Block] {
        &self.blocks
    }

    /// All blocks, mutably.
    #[must_use]
    pub fn blocks_mut(&mut self) -> &mut [Block] {
        &mut self.blocks
    }

    /// Names of all blocks in file order.
    #[must_use]
    pub fn block_names(&self) -> Vec<&str> {
        self.blocks
            .iter()
            .map(|block| block.name.as_str())
            .collect()
    }

    /// Looks up a block by its exact name.
    #[must_use]
    pub fn block(&self, name: &str) -> Option<&Block> {
        self.blocks.iter().find(|block| block.name == name)
    }

    /// Looks up a block by its exact name, mutably.
    #[must_use]
    pub fn block_mut(&mut self, name: &str) -> Option<&mut Block> {
        self.blocks.iter_mut().find(|block| block.name == name)
    }

    /// Removes a block by name.
    pub fn remove_block(&mut self, name: &str) -> bool {
        let before = self.blocks.len();
        self.blocks.retain(|block| block.name != name);
        self.blocks.len() != before
    }

    /// The global state block, which also contains the player creature.
    pub fn global_block(&self) -> Result<&Block> {
        self.block(GLOBAL_BLOCK).ok_or_else(|| {
            Error::Corrupt("the savegame does not contain a save.sav block".to_owned())
        })
    }

    /// The global state block, mutably.
    pub fn global_block_mut(&mut self) -> Result<&mut Block> {
        self.block_mut(GLOBAL_BLOCK).ok_or_else(|| {
            Error::Corrupt("the savegame does not contain a save.sav block".to_owned())
        })
    }

    /// True when at least one block was edited.
    #[must_use]
    pub fn is_modified(&self) -> bool {
        self.blocks.iter().any(Block::is_modified)
    }

    /// Names of the blocks that were edited.
    #[must_use]
    pub fn modified_blocks(&self) -> Vec<&str> {
        self.blocks
            .iter()
            .filter(|block| block.is_modified())
            .map(|block| block.name.as_str())
            .collect()
    }

    /// Serializes the whole savegame.
    ///
    /// Blocks that were not edited are copied verbatim, so opening and saving a
    /// file without changes produces exactly the original bytes.
    pub fn to_bytes(&self) -> Result<Vec<u8>> {
        let mut out = Vec::with_capacity(self.bytes.len() + 1024);
        let count = u32::try_from(self.blocks.len())
            .map_err(|_| Error::Corrupt("too many blocks".to_owned()))?;
        out.extend_from_slice(&count.to_le_bytes());
        for block in &self.blocks {
            let payload = if block.modified {
                block.gff.to_bytes()?
            } else {
                self.bytes[block.span.clone()].to_vec()
            };
            let name_length = block.name.len() + 1;
            let name_length = u32::try_from(name_length)
                .map_err(|_| Error::Corrupt("block name is too long".to_owned()))?;
            out.extend_from_slice(&name_length.to_le_bytes());
            out.extend_from_slice(block.name.as_bytes());
            out.push(0);
            let size = u32::try_from(payload.len())
                .map_err(|_| Error::Corrupt("block is too large".to_owned()))?;
            out.extend_from_slice(&size.to_le_bytes());
            out.extend_from_slice(&payload);
        }
        Ok(out)
    }

    /// Verifies the current in-memory state by serializing and re-parsing it.
    ///
    /// This is deliberately strict: a savegame is only written after the tree
    /// survived a full round trip.
    pub fn verify(&self) -> Result<()> {
        let bytes = self.to_bytes()?;
        let reparsed = Self::parse(self.path.clone(), bytes)
            .map_err(|error| Error::Verification(error.to_string()))?;
        if reparsed.blocks.len() != self.blocks.len() {
            return Err(Error::Verification(format!(
                "expected {} blocks, re-read {}",
                self.blocks.len(),
                reparsed.blocks.len()
            )));
        }
        for (index, (expected, actual)) in self.blocks.iter().zip(&reparsed.blocks).enumerate() {
            if expected.name != actual.name {
                return Err(Error::Verification(format!(
                    "block {index} is named {:?} but was re-read as {:?}",
                    expected.name, actual.name
                )));
            }
            if expected.gff.structs.len() != actual.gff.structs.len()
                || expected.gff.labels != actual.gff.labels
            {
                return Err(Error::Verification(format!(
                    "block {:?} changed shape while round tripping",
                    expected.name
                )));
            }
            for (struct_index, (expected_struct, actual_struct)) in expected
                .gff
                .structs
                .iter()
                .zip(&actual.gff.structs)
                .enumerate()
            {
                if expected_struct != actual_struct {
                    return Err(Error::Verification(format!(
                        "block {:?} struct {struct_index} changed while round tripping",
                        expected.name
                    )));
                }
            }
        }
        Ok(())
    }

    /// Saves the savegame back to its original path.
    pub fn save(&mut self, options: SaveOptions) -> Result<PathBuf> {
        self.save_to(self.path.clone(), options)
    }

    /// Saves the savegame to `target`.
    ///
    /// The file is verified first, written to a sibling temporary file and then
    /// moved into place, so a crash can never leave a half written savegame
    /// behind.
    pub fn save_to(&mut self, target: impl AsRef<Path>, options: SaveOptions) -> Result<PathBuf> {
        let target = target.as_ref();
        self.verify()?;
        let bytes = self.to_bytes()?;

        if options.backup && target.exists() {
            let backup = backup_path(target);
            fs::copy(target, &backup).map_err(|error| Error::io(&backup, error))?;
        }

        let temporary = temporary_path(target);
        {
            let mut file =
                fs::File::create(&temporary).map_err(|error| Error::io(&temporary, error))?;
            file.write_all(&bytes)
                .map_err(|error| Error::io(&temporary, error))?;
            file.sync_all()
                .map_err(|error| Error::io(&temporary, error))?;
        }
        fs::rename(&temporary, target).map_err(|error| {
            let _ = fs::remove_file(&temporary);
            Error::io(target, error)
        })?;

        // Re-parse what was written so that the in-memory state (including all
        // byte ranges) is guaranteed to describe the file on disk.
        let reloaded = Self::parse(target.to_path_buf(), bytes)?;
        *self = reloaded;
        Ok(target.to_path_buf())
    }
}

/// Path of the backup file that is created before overwriting `target`.
#[must_use]
pub fn backup_path(target: &Path) -> PathBuf {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| duration.as_secs());
    let name = target.file_name().map_or_else(
        || "save.sac".to_owned(),
        |name| name.to_string_lossy().into(),
    );
    target.with_file_name(format!("{name}.jadeforge-backup-{stamp}"))
}

fn temporary_path(target: &Path) -> PathBuf {
    let name = target.file_name().map_or_else(
        || "save.sac".to_owned(),
        |name| name.to_string_lossy().into(),
    );
    target.with_file_name(format!(".{name}.jadeforge-tmp-{}", std::process::id()))
}

struct Reader<'a> {
    bytes: &'a [u8],
    position: usize,
}

impl<'a> Reader<'a> {
    fn u32(&mut self, what: &str) -> Result<u32> {
        let bytes = self.bytes(4, what)?;
        Ok(u32::from_le_bytes(bytes.try_into().expect("4 bytes")))
    }

    fn bytes(&mut self, count: usize, what: &str) -> Result<&'a [u8]> {
        let end = self
            .position
            .checked_add(count)
            .ok_or_else(|| Error::Corrupt(format!("{what} overflows the address space")))?;
        let slice = self.bytes.get(self.position..end).ok_or_else(|| {
            Error::NotASavegame(format!("unexpected end of file while reading {what}"))
        })?;
        self.position = end;
        Ok(slice)
    }
}
