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

use std::fs::{self, File, OpenOptions};
use std::io::{self, Read as _, Write as _};
use std::ops::Range;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::error::{Error, Result};
use crate::gff::{Gff, Value};

/// Name of the block that stores the global game state and the player creature.
pub const GLOBAL_BLOCK: &str = "save.sav";
/// Name of the block used by older Jade Empire releases for character sheets.
pub const POLYPLAYER_BLOCK: &str = "polyplayer.sav";
/// Largest accepted block name, including the trailing NUL.
const MAX_NAME_LEN: usize = 260;
/// Suffix counter so two saves in the same clock tick never share a path.
static UNIQUE_COUNTER: AtomicU64 = AtomicU64::new(0);

/// Values needed to describe a savegame in a list, without building the tree.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SaveMetadata {
    /// Character name shown in the list.
    pub name: String,
    /// Character level (defaults to 1 like [`crate::Character::level`]).
    pub level: u8,
    /// Play time in microseconds.
    pub time_played: u64,
}

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

    /// Serializes this block from its parsed tree, ignoring the copy-through
    /// optimisation of [`SaveGame::to_bytes`].
    ///
    /// Two blocks with the same content produce the same bytes even when the
    /// original block was not canonically encoded. Content comparisons use
    /// this; writing a savegame never does.
    pub fn to_canonical_bytes(&self) -> Result<Vec<u8>> {
        Ok(self.gff.to_bytes_named(&self.name)?)
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

/// Identity of the file that was opened, used to detect external edits.
#[derive(Debug, Clone)]
struct SourceStamp {
    len: u64,
    modified: Option<SystemTime>,
    #[cfg(unix)]
    dev: u64,
    #[cfg(unix)]
    ino: u64,
    #[cfg(windows)]
    created: Option<SystemTime>,
}

impl SourceStamp {
    fn from_metadata(metadata: &fs::Metadata) -> Self {
        #[cfg(unix)]
        use std::os::unix::fs::MetadataExt as _;
        Self {
            len: metadata.len(),
            modified: metadata.modified().ok(),
            #[cfg(unix)]
            dev: metadata.dev(),
            #[cfg(unix)]
            ino: metadata.ino(),
            #[cfg(windows)]
            created: metadata.created().ok(),
        }
    }

    fn matches(&self, metadata: &fs::Metadata) -> bool {
        #[cfg(unix)]
        use std::os::unix::fs::MetadataExt as _;
        let same = self.len == metadata.len() && self.modified == metadata.modified().ok();
        #[cfg(unix)]
        {
            same && self.dev == metadata.dev() && self.ino == metadata.ino()
        }
        #[cfg(windows)]
        {
            same && self.created == metadata.created().ok()
        }
        #[cfg(not(any(unix, windows)))]
        {
            same
        }
    }
}

/// A loaded Jade Empire savegame.
#[derive(Debug, Clone)]
pub struct SaveGame {
    path: PathBuf,
    bytes: Vec<u8>,
    blocks: Vec<Block>,
    source: Option<SourceStamp>,
    /// Backup written by the most recent [`SaveGame::save`] call.
    ///
    /// The path is kept so the desktop UI can show the user which file holds
    /// the pre-save copy instead of leaving them to search the save folder.
    last_backup: Option<PathBuf>,
}

impl SaveGame {
    /// Loads a savegame from disk.
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        let mut file = File::open(path).map_err(|error| Error::io(path, error))?;
        let metadata = file.metadata().map_err(|error| Error::io(path, error))?;
        let mut bytes = Vec::new();
        file.read_to_end(&mut bytes)
            .map_err(|error| Error::io(path, error))?;
        if metadata.len() != u64::try_from(bytes.len()).unwrap_or(u64::MAX) {
            return Err(Error::Conflict {
                path: path.to_path_buf(),
            });
        }
        Self::parse_container(
            path.to_path_buf(),
            bytes,
            Some(SourceStamp::from_metadata(&metadata)),
        )
    }

    /// Loads a savegame from memory. `path` is only used for error messages and
    /// as the default target for [`SaveGame::save`].
    pub fn parse(path: PathBuf, bytes: Vec<u8>) -> Result<Self> {
        Self::parse_container(path, bytes, None)
    }

    fn parse_container(path: PathBuf, bytes: Vec<u8>, source: Option<SourceStamp>) -> Result<Self> {
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

        let mut blocks: Vec<Block> = Vec::with_capacity(count);
        for index in 0..count {
            let name = read_block_name(&mut reader, index)?;
            let size = reader.u32("block size")? as usize;
            let start = reader.position;
            let payload = reader.bytes(size, "block payload")?;
            let end = reader.position;
            if blocks.iter().any(|block| block.name == name) {
                return Err(Error::Corrupt(format!(
                    "the container contains the block name {name:?} twice"
                )));
            }
            let gff = Gff::parse(&name, payload)?;
            blocks.push(Block {
                name,
                gff,
                span: start..end,
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
            source,
            last_backup: None,
        })
    }

    /// Reads only the values needed for the savegame list.
    ///
    /// Unlike [`SaveGame::open`] this never parses the full GFF trees: it walks
    /// the container headers and asks the GFF reader for three selected fields.
    pub fn metadata(path: impl AsRef<Path>) -> Result<SaveMetadata> {
        let path = path.as_ref();
        let bytes = fs::read(path).map_err(|error| Error::io(path, error))?;
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
        let mut global: Option<&[u8]> = None;
        let mut polyplayer: Option<&[u8]> = None;
        let mut seen = std::collections::HashSet::new();
        for index in 0..count {
            let name = read_block_name(&mut reader, index)?;
            if !seen.insert(name.clone()) {
                return Err(Error::Corrupt(format!(
                    "the container contains the block name {name:?} twice"
                )));
            }
            let size = reader.u32("block size")? as usize;
            let payload = reader.bytes(size, "block payload")?;
            if name == GLOBAL_BLOCK && global.is_none() {
                global = Some(payload);
            } else if name == POLYPLAYER_BLOCK && polyplayer.is_none() {
                polyplayer = Some(payload);
            }
        }
        if reader.position != bytes.len() {
            return Err(Error::Corrupt(format!(
                "{} trailing bytes after the last block",
                bytes.len() - reader.position
            )));
        }
        let mut name = None;
        let mut level = 1_u8;
        let mut time_played = 0_u64;
        if let Some(block) = global {
            let selected = Gff::selected_fields(
                GLOBAL_BLOCK,
                block,
                &[
                    "PlayerCreature.FirstName",
                    "PlayerCreature.Level",
                    "Campaign.TimePlayed",
                ],
            )?;
            name = selected[0]
                .as_ref()
                .and_then(Value::as_str)
                .map(str::to_owned);
            level = selected[1]
                .as_ref()
                .and_then(Value::as_u64)
                .and_then(|value| u8::try_from(value).ok())
                .unwrap_or(1);
            time_played = selected[2].as_ref().and_then(Value::as_u64).unwrap_or(0);
        }
        if name.is_none()
            && let Some(block) = polyplayer
        {
            let selected = Gff::selected_fields(POLYPLAYER_BLOCK, block, &["FirstName", "Level"])?;
            name = selected[0]
                .as_ref()
                .and_then(Value::as_str)
                .map(str::to_owned);
            level = selected[1]
                .as_ref()
                .and_then(Value::as_u64)
                .and_then(|value| u8::try_from(value).ok())
                .unwrap_or(1);
        }
        let name = name.ok_or_else(|| {
            Error::Corrupt("the savegame does not contain a player character".to_owned())
        })?;
        Ok(SaveMetadata {
            name,
            level,
            time_played,
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

    /// Edits the player block on a working copy and commits only on success.
    ///
    /// Typed editors use this so a rejected edit never dirties the savegame or
    /// leaves orphan structs behind. `edit` returns `None` for a no-op.
    pub(crate) fn edit_player_gff<T>(
        &mut self,
        edit: impl FnOnce(&mut Gff, &str) -> Result<Option<T>>,
    ) -> Result<Option<T>> {
        let location = self.player_location()?;
        let block = self
            .block_mut(location.block)
            .ok_or_else(|| Error::Corrupt("character block vanished".to_owned()))?;
        let mut working = block.gff().clone();
        let result = edit(&mut working, location.prefix)?;
        if result.is_some() {
            *block.gff_mut() = working;
        }
        Ok(result)
    }

    /// True when at least one block was edited.
    #[must_use]
    pub fn is_modified(&self) -> bool {
        self.blocks.iter().any(Block::is_modified)
    }

    /// Canonical payload of one block by index.
    ///
    /// The undo history captures the payload of the blocks an edit touched, so
    /// it can restore them without keeping a second copy of the whole
    /// savegame. The bytes are canonical, so restoring them and writing the
    /// savegame produces a deterministic file.
    ///
    /// # Errors
    ///
    /// Returns an error when the index is out of range or the block cannot be
    /// serialized.
    pub fn block_canonical_payload(&self, index: usize) -> Result<Vec<u8>> {
        self.blocks
            .get(index)
            .ok_or_else(|| Error::Corrupt(format!("block {index} does not exist")))?
            .to_canonical_bytes()
    }

    /// Replaces a block with a canonical payload captured earlier.
    ///
    /// The payload must parse as the same block kind; the block is marked as
    /// modified so the next save serializes it instead of copying the original
    /// byte range. Used by the undo history.
    ///
    /// # Errors
    ///
    /// Returns an error for an out-of-range index, an unparsable payload or a
    /// payload whose block cannot be validated.
    pub fn restore_block_payload(&mut self, index: usize, payload: &[u8]) -> Result<()> {
        let block = self
            .blocks
            .get_mut(index)
            .ok_or_else(|| Error::Corrupt(format!("block {index} does not exist")))?;
        let gff = Gff::parse(&block.name, payload)?;
        gff.validate(&block.name)?;
        block.gff = gff;
        block.modified = true;
        Ok(())
    }

    /// Restores a block to the payload it had when the file was opened.
    ///
    /// The payload comes back out of the container bytes, so an undo of the
    /// first edit restores the exact original block and marks it clean, which
    /// lets the next save copy the original range through verbatim. The undo
    /// history is discarded when a save replaces the file, so the stored source
    /// bytes always describe the current on-disk baseline.
    ///
    /// # Errors
    ///
    /// Returns an error for an out-of-range index or a corrupt stored block.
    pub fn restore_block_from_source(&mut self, index: usize) -> Result<()> {
        let block = self
            .blocks
            .get(index)
            .ok_or_else(|| Error::Corrupt(format!("block {index} does not exist")))?;
        let payload = self.bytes.get(block.span.clone()).ok_or_else(|| {
            Error::Corrupt(format!(
                "the stored byte range for block {:?} is invalid",
                block.name
            ))
        })?;
        let gff = Gff::parse(&block.name, payload)?;
        gff.validate(&block.name)?;
        let block = self
            .blocks
            .get_mut(index)
            .ok_or_else(|| Error::Corrupt(format!("block {index} does not exist")))?;
        block.gff = gff;
        block.mark_clean();
        Ok(())
    }

    /// Canonical payloads of every block, in file order.
    ///
    /// Used to compare the in-memory savegame with a baseline by content: an
    /// edit that is undone produces the same payloads again, even for a block
    /// that was not stored canonically on disk.
    pub fn canonical_blocks(&self) -> Result<Vec<Vec<u8>>> {
        self.blocks.iter().map(Block::to_canonical_bytes).collect()
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
                block.gff.to_bytes_named(&block.name)?
            } else {
                self.bytes
                    .get(block.span.clone())
                    .ok_or_else(|| {
                        Error::Corrupt(format!(
                            "the stored byte range for block {:?} is invalid",
                            block.name
                        ))
                    })?
                    .to_vec()
            };
            validate_block_name(&block.name)?;
            let name_length = u32::try_from(block.name.len() + 1)
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
    /// survived a full round trip. It checks the tree, labels, block names,
    /// version and root. The semantic invariants of validated edits are checked
    /// by the typed editors and are not a substitute for the game accepting the
    /// file.
    pub fn verify(&self) -> Result<()> {
        let bytes = self.to_bytes()?;
        self.verify_bytes(&bytes)?;
        self.verify_semantics()
    }

    /// Checks the small, cheap invariants that must hold after a typed edit.
    ///
    /// The tree round trip is checked by [`SaveGame::verify_bytes`]; this layer
    /// catches the domain mistakes that are structurally valid, such as an
    /// amulet list that grew beyond its seven sockets or a style level that no
    /// longer matches its mask.
    fn verify_semantics(&self) -> Result<()> {
        let Ok(location) = self.player_location() else {
            return Ok(());
        };
        let block = self
            .block(location.block)
            .ok_or_else(|| Error::Corrupt("character block vanished".to_owned()))?;
        let gff = block.gff();
        self.verify_amulet_semantics(gff, location.prefix)?;
        self.verify_style_semantics(gff)
    }

    fn verify_amulet_semantics(&self, gff: &Gff, prefix: &str) -> Result<()> {
        let amulet_path = format!("{prefix}Amulet.OpenGemsList");
        if let Ok(field) = gff.get(&amulet_path) {
            let Some(elements) = field.value.as_list() else {
                return Err(Error::Verification(format!("{amulet_path} is not a list")));
            };
            if elements.len() != crate::gems::AMULET_SOCKET_SLOTS {
                return Err(Error::Verification(format!(
                    "{amulet_path} has {} entries, expected {}",
                    elements.len(),
                    crate::gems::AMULET_SOCKET_SLOTS
                )));
            }
            let amulet = self.amulet()?;
            let socketed = self
                .gems()?
                .into_iter()
                .filter(|gem| {
                    gem.location == crate::GemLocation::Amulet
                        && (gem.item_id != 0 || !gem.effects.is_empty())
                })
                .count();
            if let Some(amulet) = amulet {
                if socketed > usize::from(amulet.sockets) {
                    return Err(Error::Verification(format!(
                        "{socketed} gems are socketed, but the amulet only has {} sockets",
                        amulet.sockets
                    )));
                }
            } else if socketed > 0 {
                return Err(Error::Verification(
                    "gems are socketed while the amulet is not unlocked".to_owned(),
                ));
            }
        } else if self.amulet()?.is_some() {
            return Err(Error::Verification(format!(
                "the amulet is unlocked but {amulet_path} is missing"
            )));
        }
        Ok(())
    }

    fn verify_style_semantics(&self, gff: &Gff) -> Result<()> {
        for style in self.styles()? {
            let Some(structure) = gff.struct_at(style.struct_index) else {
                return Err(Error::Verification(format!(
                    "style struct {} is missing",
                    style.struct_index
                )));
            };
            let field = |name: &str| {
                structure
                    .fields
                    .iter()
                    .find(|field| gff.label_name(field.label) == Some(name))
                    .map(|field| &field.value)
            };
            let level = field("StyleLevel");
            let mask = field("StyleLevelInfo");
            let expected = style.mask.count_ones() + 1;
            match (level, mask) {
                (Some(level), Some(_)) => {
                    let Some(level) = level.as_u64() else {
                        return Err(Error::Verification(
                            "a style level is not a number".to_owned(),
                        ));
                    };
                    if level != u64::from(expected) {
                        return Err(Error::Verification(format!(
                            "style {} has level {level} but its mask describes {expected}",
                            style.style_id
                        )));
                    }
                }
                (Some(level), None) => {
                    let Some(level) = level.as_u64() else {
                        return Err(Error::Verification(
                            "a style level is not a number".to_owned(),
                        ));
                    };
                    if level != 1 {
                        return Err(Error::Verification(format!(
                            "style {} has level {level} but no upgrade mask",
                            style.style_id
                        )));
                    }
                }
                (None, Some(mask)) => {
                    let Some(mask) = mask.as_u64() else {
                        return Err(Error::Verification(
                            "a style upgrade mask is not a number".to_owned(),
                        ));
                    };
                    if mask != 0 {
                        return Err(Error::Verification(format!(
                            "style {} has upgrade bits but no level",
                            style.style_id
                        )));
                    }
                }
                (None, None) => {}
            }
        }
        Ok(())
    }

    fn verify_bytes(&self, bytes: &[u8]) -> Result<()> {
        let reparsed = Self::parse_container(self.path.clone(), bytes.to_vec(), None)
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
            if expected.gff.version != actual.gff.version {
                return Err(Error::Verification(format!(
                    "block {:?} changed its version while round tripping",
                    expected.name
                )));
            }
            if expected.gff.root != actual.gff.root {
                return Err(Error::Verification(format!(
                    "block {:?} changed its root while round tripping",
                    expected.name
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

    fn verified_bytes(&self) -> Result<Vec<u8>> {
        let bytes = self.to_bytes()?;
        self.verify_bytes(&bytes)?;
        self.verify_semantics()?;
        Ok(bytes)
    }

    /// Saves the savegame back to its original path.
    pub fn save(&mut self, options: SaveOptions) -> Result<PathBuf> {
        self.save_to(self.path.clone(), options)
    }

    /// Saves the savegame to `target`.
    ///
    /// The file is verified first, written to an exclusive sibling temporary
    /// file and then moved into place, so a crash can never leave a half written
    /// savegame behind. The temporary file never follows a pre-existing link,
    /// the target's permissions are preserved, and a target that changed on disk
    /// since it was opened is reported as a conflict instead of overwritten.
    #[allow(clippy::too_many_lines)] // the ordering of these steps is the contract
    pub fn save_to(&mut self, target: impl AsRef<Path>, options: SaveOptions) -> Result<PathBuf> {
        let target = target.as_ref();
        let write_target = resolve_write_target(target)?;
        let bytes = self.verified_bytes()?;
        self.ensure_unchanged(&write_target)?;
        let target_metadata = match fs::metadata(&write_target) {
            Ok(metadata) => Some(metadata),
            Err(error) if error.kind() == io::ErrorKind::NotFound => None,
            Err(error) => return Err(Error::io(&write_target, error)),
        };

        // Keep backups next to the path the caller named, even when that path
        // is a symlink. The backup browser and the reported backup path then
        // use the same visible name as the savegame.
        let backup = if options.backup && target_metadata.is_some() {
            Some(create_backup(target)?)
        } else {
            None
        };

        let (temporary, mut file) = create_temporary(&write_target)?;
        let write_result = (|| -> Result<SourceStamp> {
            if let Some(metadata) = &target_metadata {
                file.set_permissions(metadata.permissions())
                    .map_err(|error| Error::io(&temporary, error))?;
            }
            file.write_all(&bytes)
                .map_err(|error| Error::io(&temporary, error))?;
            file.sync_all()
                .map_err(|error| Error::io(&temporary, error))?;
            let stamp = SourceStamp::from_metadata(
                &file
                    .metadata()
                    .map_err(|error| Error::io(&temporary, error))?,
            );
            drop(file);
            self.ensure_unchanged(&write_target)?;
            fs::rename(&temporary, &write_target)
                .map_err(|error| Error::io(&write_target, error))?;
            sync_parent(&write_target)?;
            Ok(stamp)
        })();

        let stamp = match write_result {
            Ok(stamp) => stamp,
            Err(error) => {
                let _ = fs::remove_file(&temporary);
                return Err(error);
            }
        };

        let reloaded = Self::parse_container(target.to_path_buf(), bytes, Some(stamp))?;
        *self = reloaded;
        self.last_backup = backup;
        Ok(target.to_path_buf())
    }

    /// Backup file written by the most recent save, when one was kept.
    ///
    /// `None` means the previous save had no backup (a new target or
    /// [`SaveOptions`] without backups), not that a backup failed: a failed
    /// backup aborts the save before the savegame is replaced.
    #[must_use]
    pub fn last_backup(&self) -> Option<&Path> {
        self.last_backup.as_deref()
    }

    fn ensure_unchanged(&self, target: &Path) -> Result<()> {
        let Some(stamp) = &self.source else {
            return Ok(());
        };
        // Only the file this savegame was loaded from is protected; saving to a
        // new target is an explicit copy and has no previous identity to check.
        let source_path = fs::canonicalize(&self.path).unwrap_or_else(|_| self.path.clone());
        let target_path = fs::canonicalize(target).unwrap_or_else(|_| target.to_path_buf());
        if source_path != target_path {
            return Ok(());
        }
        let metadata = fs::metadata(target).map_err(|error| {
            if error.kind() == io::ErrorKind::NotFound {
                Error::Conflict {
                    path: target.to_path_buf(),
                }
            } else {
                Error::io(target, error)
            }
        })?;
        if !stamp.matches(&metadata) {
            return Err(Error::Conflict {
                path: target.to_path_buf(),
            });
        }
        Ok(())
    }
}

/// Reads one NUL terminated block name.
fn read_block_name(reader: &mut Reader<'_>, index: usize) -> Result<String> {
    let name_length = reader.u32("block name length")? as usize;
    if name_length == 0 || name_length > MAX_NAME_LEN {
        return Err(Error::Corrupt(format!(
            "block {index} has an invalid name length of {name_length}"
        )));
    }
    let raw_name = reader.bytes(name_length, "block name")?;
    let Some((&0, name)) = raw_name.split_last() else {
        return Err(Error::Corrupt(format!(
            "block {index} name is missing its trailing NUL"
        )));
    };
    if name.is_empty() || name.contains(&0) {
        return Err(Error::Corrupt(format!(
            "block {index} has an empty or embedded-NUL name"
        )));
    }
    std::str::from_utf8(name)
        .map(str::to_owned)
        .map_err(|_| Error::Corrupt(format!("block {index} has a non UTF-8 name")))
}

/// Validates a name before writing it into the container.
fn validate_block_name(name: &str) -> Result<()> {
    if name.is_empty() || name.len() + 1 > MAX_NAME_LEN || name.as_bytes().contains(&0) {
        return Err(Error::Corrupt(format!(
            "block name {name:?} cannot be stored in the container"
        )));
    }
    Ok(())
}

/// Path of a backup file for `target`.
///
/// The name contains the current second, nanosecond and a process-local
/// counter, so two saves in the same second never share a backup path. Creation
/// still uses an exclusive open and retries, so an existing file is never
/// overwritten.
#[must_use]
pub fn backup_path(target: &Path) -> PathBuf {
    let name = target.file_name().map_or_else(
        || "save.sac".to_owned(),
        |name| name.to_string_lossy().into(),
    );
    target.with_file_name(format!("{name}.jadeforge-backup-{}", unique_stamp()))
}

/// Creates a complete backup of `target` and returns its path.
pub(crate) fn create_backup(target: &Path) -> Result<PathBuf> {
    let metadata = fs::metadata(target).map_err(|error| Error::io(target, error))?;
    let (temporary, temporary_file) = create_temporary_named(target, "backup")?;
    let result = (|| -> Result<PathBuf> {
        let mut destination = temporary_file;
        destination
            .set_permissions(metadata.permissions())
            .map_err(|error| Error::io(&temporary, error))?;
        let mut source = File::open(target).map_err(|error| Error::io(target, error))?;
        io::copy(&mut source, &mut destination).map_err(|error| Error::io(&temporary, error))?;
        destination
            .sync_all()
            .map_err(|error| Error::io(&temporary, error))?;
        drop(destination);
        publish_backup(&temporary, target)
    })();
    match result {
        Ok(backup) => Ok(backup),
        Err(error) => {
            let _ = fs::remove_file(&temporary);
            Err(error)
        }
    }
}

/// Publishes a complete temporary backup without replacing an existing one.
fn publish_backup(temporary: &Path, target: &Path) -> Result<PathBuf> {
    for _ in 0..64 {
        let backup = backup_path(target);
        if publish_backup_at(temporary, &backup)? {
            return Ok(backup);
        }
    }
    Err(Error::Corrupt(
        "could not find a free backup name".to_owned(),
    ))
}

/// Publishes `temporary` at `backup`, returning whether the name was free.
///
/// A hard link is the primary no-replace operation. Filesystems without hard
/// links fall back to reserving the name with `create_new` and renaming the
/// complete temporary file over that reservation.
fn publish_backup_at(temporary: &Path, backup: &Path) -> Result<bool> {
    match fs::hard_link(temporary, backup) {
        Ok(()) => {
            let _ = fs::remove_file(temporary);
            sync_parent(backup)?;
            return Ok(true);
        }
        Err(error) if error.kind() == io::ErrorKind::AlreadyExists => return Ok(false),
        Err(_) => {}
    }

    match OpenOptions::new().write(true).create_new(true).open(backup) {
        Ok(file) => drop(file),
        Err(error) if error.kind() == io::ErrorKind::AlreadyExists => return Ok(false),
        Err(error) => return Err(Error::io(backup, error)),
    }
    if let Err(error) = fs::rename(temporary, backup) {
        let _ = fs::remove_file(backup);
        return Err(Error::io(backup, error));
    }
    sync_parent(backup)?;
    Ok(true)
}

/// Creates an exclusive temporary sibling and returns it with an open handle.
fn create_temporary(target: &Path) -> Result<(PathBuf, File)> {
    create_temporary_named(target, "tmp")
}

fn create_temporary_named(target: &Path, kind: &str) -> Result<(PathBuf, File)> {
    let name = target.file_name().map_or_else(
        || "save.sac".to_owned(),
        |name| name.to_string_lossy().into(),
    );
    for _ in 0..64 {
        let path = target.with_file_name(format!(".{name}.jadeforge-{kind}-{}", unique_stamp()));
        match OpenOptions::new().write(true).create_new(true).open(&path) {
            Ok(file) => return Ok((path, file)),
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {}
            Err(error) => return Err(Error::io(&path, error)),
        }
    }
    Err(Error::Corrupt(format!(
        "could not create an exclusive {kind} file next to {}",
        target.display()
    )))
}

/// Resolves a symlinked target to the file it points at, so saving and
/// restoring never replace the link with a regular file.
pub(crate) fn resolve_write_target(target: &Path) -> Result<PathBuf> {
    match fs::symlink_metadata(target) {
        Ok(metadata) if metadata.file_type().is_symlink() => {
            fs::canonicalize(target).map_err(|error| Error::io(target, error))
        }
        Ok(_) | Err(_) => Ok(target.to_path_buf()),
    }
}

/// Returns the directory that contains `path`.
///
/// A bare relative path has an empty parent. Treating it as the current
/// directory keeps save, backup and restore operations working for names such
/// as `save.sac`.
pub(crate) fn parent_directory(path: &Path) -> Option<&Path> {
    path.parent().map(|parent| {
        if parent.as_os_str().is_empty() {
            Path::new(".")
        } else {
            parent
        }
    })
}

/// Syncs the containing directory after a rename, on platforms where that is
/// meaningful.
// Windows has no directory fsync; keeping the same Result signature lets the
// save path call this unconditionally.
#[cfg_attr(not(unix), allow(clippy::unnecessary_wraps))]
pub(crate) fn sync_parent(path: &Path) -> Result<()> {
    #[cfg(unix)]
    {
        if let Some(parent) = parent_directory(path) {
            let directory = File::open(parent).map_err(|error| Error::io(parent, error))?;
            directory
                .sync_all()
                .map_err(|error| Error::io(parent, error))?;
        }
    }
    #[cfg(not(unix))]
    {
        let _ = path;
    }
    Ok(())
}

/// A timestamp plus a counter, unique within a process.
fn unique_stamp() -> String {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    let counter = UNIQUE_COUNTER.fetch_add(1, Ordering::Relaxed);
    format!("{}.{:09}.{counter}", now.as_secs(), now.subsec_nanos())
}

struct Reader<'a> {
    bytes: &'a [u8],
    position: usize,
}

impl<'a> Reader<'a> {
    fn u32(&mut self, what: &str) -> Result<u32> {
        let bytes = self.bytes(4, what)?;
        bytes
            .try_into()
            .map(u32::from_le_bytes)
            .map_err(|_| Error::Corrupt(format!("{what} is not four bytes")))
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

#[cfg(test)]
mod tests {
    use super::{parent_directory, publish_backup_at};
    use std::fs;
    use std::path::{Path, PathBuf};

    fn test_directory(tag: &str) -> PathBuf {
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |duration| duration.as_nanos());
        let path = std::env::temp_dir().join(format!(
            "jadeforge-save-test-{tag}-{}-{stamp}",
            std::process::id()
        ));
        fs::create_dir_all(&path).expect("create test directory");
        path
    }

    #[test]
    fn a_bare_relative_path_uses_the_current_directory() {
        assert_eq!(
            parent_directory(Path::new("save.sac")),
            Some(Path::new("."))
        );
    }

    #[test]
    fn an_absolute_path_keeps_its_directory() {
        assert_eq!(
            parent_directory(Path::new("/games/Jade Empire/Save/save.sac")),
            Some(Path::new("/games/Jade Empire/Save"))
        );
    }

    #[test]
    fn a_preexisting_backup_name_is_never_replaced() {
        let directory = test_directory("backup-publish");
        let temporary = directory.join("temporary");
        let backup = directory.join("backup");
        fs::write(&temporary, b"new").expect("write temporary");
        fs::write(&backup, b"old").expect("write existing backup");

        assert!(
            !publish_backup_at(&temporary, &backup).expect("attempt publication"),
            "an existing backup was treated as free"
        );
        assert_eq!(fs::read(&backup).expect("read backup"), b"old");
        assert!(
            temporary.exists(),
            "the occupied publication removed its input"
        );

        fs::remove_file(&backup).expect("remove existing backup");
        assert!(
            publish_backup_at(&temporary, &backup).expect("publish free name"),
            "a free backup name was not published"
        );
        assert_eq!(fs::read(&backup).expect("read published backup"), b"new");
        assert!(!temporary.exists(), "the published temporary was not moved");

        fs::remove_dir_all(directory).expect("remove test directory");
    }
}
