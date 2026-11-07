#![allow(dead_code)] // each integration test binary uses a different subset
#![allow(clippy::expect_used)] // test fixtures fail loudly; expects are assertions

//! Shared synthetic savegame blocks for the integration tests.
//!
//! Nothing here reads the game or a real savegame: every field is built in
//! memory, so the tests that use it run on CI too.

use std::path::{Path, PathBuf};

use jadeforge_core::{Character, Gff, SaveGame, Value};

/// An empty GFF block with no structs yet.
pub(crate) fn empty_gff() -> Gff {
    Gff {
        version: *b"V3.2",
        labels: Vec::new(),
        structs: Vec::new(),
        root: 0,
    }
}

/// A GFF block whose root is a player struct with every editable field.
///
/// The values match [`character`] so tests can compare a read character with
/// the fixture without building a second literal.
#[allow(clippy::too_many_lines)] // one line per field reads best
pub(crate) fn character_gff() -> Gff {
    let mut gff = empty_gff();
    let root = gff.push_struct(u32::MAX);
    let player = gff.push_struct(0);
    gff.add_field(root, "PlayerCreature", Value::Struct(player))
        .expect("player field");
    add_character_fields(&mut gff, player);
    add_campaign(&mut gff, root, 2, 90_000_000_000);
    gff
}

/// Adds the editable character fields below `player`.
pub(crate) fn add_character_fields(gff: &mut Gff, player: usize) {
    let stats = gff.push_struct(0);
    let focus = gff.push_struct(0);
    let progress = gff.push_struct(0);

    gff.add_field(player, "FirstName", Value::String("Hero".to_owned()))
        .expect("first name");
    gff.add_field(player, "Level", Value::Byte(7))
        .expect("level");
    gff.add_field(player, "CurrentXP", Value::DWord(1234))
        .expect("experience");
    gff.add_field(player, "PlayerOrigApp", Value::Int(5))
        .expect("original appearance");
    gff.add_field(player, "Stats", Value::Struct(stats))
        .expect("stats");
    gff.add_field(player, "Progress", Value::Struct(progress))
        .expect("progress");

    gff.add_field(stats, "Gold", Value::DWord(99))
        .expect("gold");
    gff.add_field(stats, "UnassignedPts", Value::DWord(2))
        .expect("unspent points");
    gff.add_field(stats, "Health", Value::Float(120.0))
        .expect("health");
    gff.add_field(stats, "HealthBase", Value::DWord(150))
        .expect("health base");
    gff.add_field(stats, "Chi", Value::Float(80.0))
        .expect("chi");
    gff.add_field(stats, "ChiBase", Value::DWord(100))
        .expect("chi base");
    gff.add_field(stats, "Focus", Value::Struct(focus))
        .expect("focus");
    gff.add_field(focus, "STAT_CUR", Value::Float(20.0))
        .expect("focus current");
    gff.add_field(focus, "STAT_MAX", Value::Float(30.0))
        .expect("focus max");
    gff.add_field(stats, "Body", Value::DWord(5)).expect("body");
    gff.add_field(stats, "Mind", Value::DWord(6)).expect("mind");
    gff.add_field(stats, "Spirit", Value::DWord(7))
        .expect("spirit");
    gff.add_field(stats, "Charm", Value::Int(1)).expect("charm");
    gff.add_field(stats, "Intuition", Value::Int(2))
        .expect("intuition");
    gff.add_field(stats, "Intimidate", Value::Int(3))
        .expect("intimidate");
    gff.add_field(stats, "Alignment", Value::Int(200))
        .expect("alignment");
    gff.add_field(stats, "Appearance", Value::Int(5))
        .expect("appearance");
    gff.add_field(progress, "Kills", Value::DWord(42))
        .expect("kills");
}

/// Adds a `Campaign` struct with chapter and play time below `root`.
pub(crate) fn add_campaign(gff: &mut Gff, root: usize, chapter: u32, time_played: u64) {
    let campaign = gff.push_struct(0);
    gff.add_field(root, "Campaign", Value::Struct(campaign))
        .expect("campaign");
    gff.add_field(campaign, "Chapter", Value::DWord(chapter))
        .expect("chapter");
    gff.add_field(campaign, "TimePlayed", Value::DWord64(time_played))
        .expect("time played");
}

/// The character that [`character_gff`] stores.
pub(crate) fn character() -> Character {
    Character {
        name: "Hero".to_owned(),
        level: 7,
        experience: 1234,
        gold: Some(99),
        attribute_points: 2,
        health: 120.0,
        health_max: 150,
        chi: 80.0,
        chi_max: 100,
        focus: 20.0,
        focus_max: 30.0,
        body: 5,
        mind: 6,
        spirit: 7,
        charm: 1,
        intuition: 2,
        intimidate: 3,
        alignment: 200,
        appearance: 5,
        chapter: 2,
        time_played: 90_000_000_000,
        kills: 42,
    }
}

/// Index of the `PlayerCreature` struct in [`character_gff`].
pub(crate) fn player(gff: &Gff) -> usize {
    gff.get("PlayerCreature")
        .expect("player field")
        .value
        .as_struct()
        .expect("player struct")
}

/// Index of the `PlayerCreature.Stats` struct in [`character_gff`].
pub(crate) fn stats(gff: &Gff) -> usize {
    gff.get("PlayerCreature.Stats")
        .expect("stats field")
        .value
        .as_struct()
        .expect("stats struct")
}

/// Replaces a field even when the new value has a different type tag.
pub(crate) fn replace_field(gff: &mut Gff, path: &str, value: Value) {
    let _ = gff.remove(path).expect("remove field");
    gff.add(path, value).expect("add field");
}

/// Wraps a block in a one-block savegame container.
pub(crate) fn container(gff: &Gff) -> Vec<u8> {
    container_with(&[("save.sav", gff)])
}

/// Wraps blocks in a savegame container, in the given order.
pub(crate) fn container_with(blocks: &[(&str, &Gff)]) -> Vec<u8> {
    let mut bytes = Vec::new();
    let count = u32::try_from(blocks.len()).expect("block count");
    bytes.extend_from_slice(&count.to_le_bytes());
    for (name, gff) in blocks {
        let payload = gff.to_bytes().expect("serialize block");
        let name_length = u32::try_from(name.len() + 1).expect("name length");
        bytes.extend_from_slice(&name_length.to_le_bytes());
        bytes.extend_from_slice(name.as_bytes());
        bytes.push(0);
        let size = u32::try_from(payload.len()).expect("block size");
        bytes.extend_from_slice(&size.to_le_bytes());
        bytes.extend_from_slice(&payload);
    }
    bytes
}

/// Parses a block into a savegame without touching the file system.
pub(crate) fn save(gff: &Gff) -> SaveGame {
    SaveGame::parse(PathBuf::from("test.sac"), container(gff)).expect("parse savegame")
}

/// Writes a block as `save.sac` into `folder` and returns the file path.
pub(crate) fn write_save(folder: &Path, gff: &Gff) -> PathBuf {
    let path = folder.join("save.sac");
    std::fs::write(&path, container(gff)).expect("write savegame");
    path
}

/// A temporary directory that removes itself when the test ends.
pub(crate) struct TempDir {
    path: PathBuf,
}

impl TempDir {
    /// Creates a unique directory below the system temp folder.
    pub(crate) fn new(prefix: &str) -> Self {
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |duration| duration.as_nanos());
        let path =
            std::env::temp_dir().join(format!("jadeforge-{prefix}-{}-{stamp}", std::process::id()));
        std::fs::create_dir_all(&path).expect("create temp directory");
        Self { path }
    }

    /// The directory path.
    pub(crate) fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.path);
    }
}
