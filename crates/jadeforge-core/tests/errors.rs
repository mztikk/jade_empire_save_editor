#![allow(clippy::expect_used)] // tests fail loudly; expects are assertions

//! Boundary and error-path tests for the savegame layer.
//!
//! Everything here is built from scratch, so the tests run without the game or
//! real savegames.

use std::path::PathBuf;

use jadeforge_core::gff::Value;
use jadeforge_core::{DEFAULT_ALIGNMENT, Gff, SaveGame, SaveOptions};

fn player_block(item: bool) -> (Gff, Option<usize>) {
    let mut gff = Gff {
        version: *b"V3.2",
        labels: Vec::new(),
        structs: Vec::new(),
        root: 0,
    };
    let root = gff.push_struct(u32::MAX);
    let player = gff.push_struct(0);
    let stats = gff.push_struct(0);
    gff.add_field(root, "PlayerCreature", Value::Struct(player))
        .expect("player");
    gff.add_field(player, "FirstName", Value::String("Boundary".to_owned()))
        .expect("name");
    gff.add_field(player, "Level", Value::Byte(3))
        .expect("level");
    gff.add_field(player, "Stats", Value::Struct(stats))
        .expect("stats");
    gff.add_field(stats, "Gold", Value::DWord(5)).expect("gold");
    gff.add_field(stats, "Health", Value::Float(50.0))
        .expect("health");
    gff.add_field(stats, "HealthBase", Value::DWord(100))
        .expect("health base");
    gff.add_field(stats, "Chi", Value::Float(25.0))
        .expect("chi");
    gff.add_field(stats, "ChiBase", Value::DWord(50))
        .expect("chi base");
    let focus = gff.push_struct(0);
    gff.add_field(stats, "Focus", Value::Struct(focus))
        .expect("focus");
    gff.add_field(focus, "STAT_CUR", Value::Float(10.0))
        .expect("focus cur");
    gff.add_field(focus, "STAT_MAX", Value::Float(20.0))
        .expect("focus max");

    let item_index = if item {
        let item = gff.push_struct(0);
        gff.add_field(item, "Count", Value::Byte(1)).expect("count");
        gff.add_field(item, "ItemId", Value::Word(42)).expect("id");
        gff.add_field(item, "Type", Value::Byte(6)).expect("type");
        gff.add_field(player, "ItemList", Value::List(vec![item]))
            .expect("items");
        Some(item)
    } else {
        None
    };
    (gff, item_index)
}

fn container(gff: &Gff) -> Vec<u8> {
    let block = gff.to_bytes().expect("serialize block");
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&1_u32.to_le_bytes());
    bytes.extend_from_slice(&9_u32.to_le_bytes());
    bytes.extend_from_slice(b"save.sav\0");
    bytes.extend_from_slice(
        &u32::try_from(block.len())
            .expect("block size")
            .to_le_bytes(),
    );
    bytes.extend_from_slice(&block);
    bytes
}

fn save_with_item() -> (SaveGame, usize) {
    let (gff, item) = player_block(true);
    let item = item.expect("item was requested");
    (
        SaveGame::parse(PathBuf::from("boundary.sac"), container(&gff)).expect("parse"),
        item,
    )
}

#[test]
fn inventory_boundaries() {
    let (mut save, item) = save_with_item();
    assert_eq!(save.items().expect("items").len(), 1);

    save.set_item_count(item, 255).expect("maximum count");
    assert_eq!(save.items().expect("items")[0].count, 255);
    assert!(
        save.set_item_count(item, 0).is_err(),
        "zero count is invalid"
    );
    assert!(
        save.set_item_count(999, 1).is_err(),
        "missing struct is invalid"
    );
    assert!(save.add_item(43, 6, 0).is_err(), "zero stack is invalid");
    let added = save.add_item(43, 6, 2).expect("add item");
    assert_eq!(save.items().expect("items").len(), 2);

    assert!(save.remove_item(added).expect("remove added item"));
    assert!(!save.remove_item(added).expect("second remove"));
    assert_eq!(save.items().expect("items").len(), 1);
}

#[test]
fn optional_character_fields_are_created_on_write() {
    let (gff, _) = player_block(false);
    let mut save = SaveGame::parse(PathBuf::from("optional.sac"), container(&gff)).expect("parse");
    let character = save.character().expect("character");
    assert_eq!(character.gold, Some(5));
    assert_eq!(
        character.alignment,
        i32::try_from(DEFAULT_ALIGNMENT).expect("fits i32")
    );
    assert_eq!(character.attribute_points, 0);

    let mut changed = character;
    changed.gold = Some(77);
    changed.attribute_points = 4;
    changed.alignment = 200;
    save.apply_character(&changed).expect("apply character");

    let reread = save.character().expect("character");
    assert_eq!(reread.gold, Some(77));
    assert_eq!(reread.attribute_points, 4);
    // Alignment is only written when the field already exists: a missing field
    // means "never changed", so it must stay balanced.
    assert_eq!(
        reread.alignment,
        i32::try_from(DEFAULT_ALIGNMENT).expect("fits i32")
    );
    let gff = save.block("save.sav").expect("block").gff();
    assert_eq!(
        gff.get("PlayerCreature.Stats.Gold").expect("gold").value,
        Value::DWord(77)
    );
    assert_eq!(
        gff.get("PlayerCreature.Stats.UnassignedPts")
            .expect("unspent points")
            .value,
        Value::DWord(4)
    );
    assert!(gff.get("PlayerCreature.Stats.Alignment").is_err());
}

#[test]
fn malformed_containers_are_rejected() {
    assert!(SaveGame::parse("x".into(), Vec::new()).is_err());
    assert!(SaveGame::parse("x".into(), 0_u32.to_le_bytes().to_vec()).is_err());

    // Truncated name.
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&1_u32.to_le_bytes());
    bytes.extend_from_slice(&100_u32.to_le_bytes());
    bytes.extend_from_slice(b"save.sav");
    assert!(SaveGame::parse("x".into(), bytes).is_err());

    // Block size beyond the file.
    let (gff, _) = player_block(false);
    let mut bytes = container(&gff);
    let size_offset = 4 + 4 + 9;
    let size = u32::from_le_bytes(bytes[size_offset..size_offset + 4].try_into().expect("4"));
    bytes[size_offset..size_offset + 4].copy_from_slice(&(size + 100).to_le_bytes());
    assert!(SaveGame::parse("x".into(), bytes).is_err());

    // Wrong block magic and version.
    let (gff, _) = player_block(false);
    let mut bytes = container(&gff);
    bytes[17] = b'X';
    assert!(SaveGame::parse("x".into(), bytes).is_err());
    let mut bytes = container(&gff);
    bytes[21..25].copy_from_slice(b"V9.9");
    assert!(SaveGame::parse("x".into(), bytes).is_err());

    // A block whose payload is not a GFF tree.
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&1_u32.to_le_bytes());
    bytes.extend_from_slice(&9_u32.to_le_bytes());
    bytes.extend_from_slice(b"save.sav\0");
    bytes.extend_from_slice(&56_u32.to_le_bytes());
    bytes.extend_from_slice(&[0_u8; 56]);
    assert!(SaveGame::parse("x".into(), bytes).is_err());

    assert!(SaveGame::open("/definitely/not/a/save.sac").is_err());
}

#[test]
fn backups_and_unwritable_targets() {
    let dir = std::env::temp_dir().join(format!(
        "jadeforge-errors-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |duration| duration.as_nanos())
    ));
    std::fs::create_dir_all(&dir).expect("temp dir");
    let target = dir.join("save.sac");
    let (gff, item) = player_block(true);
    std::fs::write(&target, container(&gff)).expect("write save");

    let mut save = SaveGame::open(&target).expect("open");
    save.set_item_count(item.expect("item"), 2)
        .expect("edit item");
    save.save(SaveOptions::default()).expect("save with backup");
    let backups = std::fs::read_dir(&dir)
        .expect("dir")
        .flatten()
        .filter(|entry| {
            entry
                .file_name()
                .to_string_lossy()
                .contains("jadeforge-backup")
        })
        .count();
    assert_eq!(backups, 1, "expected exactly one backup");

    let copy = dir.join("copy.sac");
    save.save_to(&copy, SaveOptions { backup: false })
        .expect("save without backup");
    assert!(
        !std::fs::read_dir(&dir)
            .expect("dir")
            .flatten()
            .any(|entry| entry
                .file_name()
                .to_string_lossy()
                .contains("copy.sac.jadeforge"))
    );

    // A directory as target cannot be written.
    assert!(save.save_to(&dir, SaveOptions { backup: false }).is_err());

    let _ = std::fs::remove_dir_all(&dir);
}
