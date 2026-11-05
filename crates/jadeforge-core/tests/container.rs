#![allow(clippy::expect_used)] // tests fail loudly; expects are assertions
#![allow(clippy::assert_is_empty)] // empty checks read better than comparing with an empty slice

//! The savegame container API and the metadata fast path, without real files.

mod common;

use std::path::{Path, PathBuf};

use jadeforge_core::save::backup_path;
use jadeforge_core::{SaveGame, Value};

fn root_only() -> jadeforge_core::Gff {
    let mut gff = common::empty_gff();
    let _ = gff.push_struct(u32::MAX);
    gff
}

fn parse_blocks(blocks: &[(&str, &jadeforge_core::Gff)]) -> SaveGame {
    SaveGame::parse(PathBuf::from("fixture.sac"), common::container_with(blocks))
        .expect("parse savegame")
}

fn write_bytes(dir: &Path, bytes: &[u8]) -> PathBuf {
    let path = dir.join("save.sac");
    std::fs::write(&path, bytes).expect("write savegame");
    path
}

#[test]
fn container_helpers_report_and_change_state() {
    let save = render();
    let original = common::container_with(&[
        ("save.sav", &common::character_gff()),
        ("polyplayer.sav", &root_only()),
        ("raw", &root_only()),
    ]);
    let unchanged = parse_blocks(&[
        ("save.sav", &common::character_gff()),
        ("polyplayer.sav", &root_only()),
        ("raw", &root_only()),
    ]);
    assert_eq!(unchanged.to_bytes().expect("serialize"), original);
    assert_eq!(save.path(), Path::new("fixture.sac"));
    assert_eq!(save.block_names(), ["save.sav", "polyplayer.sav", "raw"]);
    assert_eq!(save.block("save.sav").expect("block").stem(), "save");
    assert_eq!(save.block("raw").expect("block").stem(), "raw");
    assert!(save.block("missing").is_none());
    assert!(!save.is_modified());
    assert!(save.modified_blocks().is_empty());
    assert_eq!(save.global_block().expect("global block").name, "save.sav");

    let mut save = save;
    assert_eq!(save.blocks_mut().len(), 3);
    save.blocks_mut()[2].mark_modified();
    assert!(save.is_modified());
    assert_eq!(save.modified_blocks(), ["raw"]);
    save.block_mut("raw").expect("raw block").mark_clean();
    assert!(!save.is_modified());

    assert!(save.remove_block("polyplayer.sav"));
    assert!(!save.remove_block("polyplayer.sav"));
    assert_eq!(save.block_names(), ["save.sav", "raw"]);

    save.remove_block("save.sav");
    assert!(save.global_block().is_err());
    assert!(save.global_block_mut().is_err());
    assert!(save.block_mut("save.sav").is_none());
}

fn render() -> SaveGame {
    parse_blocks(&[
        ("save.sav", &common::character_gff()),
        ("polyplayer.sav", &root_only()),
        ("raw", &root_only()),
    ])
}

#[test]
fn metadata_reads_the_fast_fields() {
    let dir = common::TempDir::new("metadata");
    let path = common::write_save(dir.path(), &common::character_gff());
    let metadata = SaveGame::metadata(&path).expect("metadata");
    assert_eq!(metadata.name, "Hero");
    assert_eq!(metadata.level, 7);
    assert_eq!(metadata.time_played, 90_000_000_000);
}

#[test]
fn metadata_falls_back_to_polyplayer_and_defaults_the_level() {
    let dir = common::TempDir::new("metadata-polyplayer");
    let mut polyplayer = common::empty_gff();
    let root = polyplayer.push_struct(u32::MAX);
    polyplayer
        .add_field(root, "FirstName", Value::String("Old Hero".to_owned()))
        .expect("first name");
    let path = write_bytes(
        dir.path(),
        &common::container_with(&[("polyplayer.sav", &polyplayer)]),
    );
    let metadata = SaveGame::metadata(&path).expect("metadata");
    assert_eq!(metadata.name, "Old Hero");
    assert_eq!(metadata.level, 1);
    assert_eq!(metadata.time_played, 0);
}

#[test]
fn metadata_falls_back_when_the_global_name_has_the_wrong_type() {
    let dir = common::TempDir::new("metadata-fallback");
    let mut global = common::empty_gff();
    let root = global.push_struct(u32::MAX);
    let player = global.push_struct(0);
    global
        .add_field(root, "PlayerCreature", Value::Struct(player))
        .expect("player");
    global
        .add_field(player, "FirstName", Value::Byte(1))
        .expect("wrong name type");

    let mut polyplayer = common::empty_gff();
    let poly_root = polyplayer.push_struct(u32::MAX);
    polyplayer
        .add_field(
            poly_root,
            "FirstName",
            Value::String("Poly Hero".to_owned()),
        )
        .expect("first name");
    polyplayer
        .add_field(poly_root, "Level", Value::Word(9))
        .expect("level");

    let path = write_bytes(
        dir.path(),
        &common::container_with(&[("save.sav", &global), ("polyplayer.sav", &polyplayer)]),
    );
    let metadata = SaveGame::metadata(&path).expect("metadata");
    assert_eq!(metadata.name, "Poly Hero");
    assert_eq!(metadata.level, 9);
}

#[test]
fn metadata_rejects_broken_containers() {
    let dir = common::TempDir::new("metadata-errors");
    let valid = common::container_with(&[("save.sav", &common::character_gff())]);

    let mut trailing = valid.clone();
    trailing.push(0);
    let mut bad_payload = Vec::new();
    bad_payload.extend_from_slice(&1_u32.to_le_bytes());
    bad_payload.extend_from_slice(&9_u32.to_le_bytes());
    bad_payload.extend_from_slice(b"save.sav\0");
    bad_payload.extend_from_slice(&56_u32.to_le_bytes());
    bad_payload.extend_from_slice(&[0_u8; 56]);

    let cases = [
        (Vec::new(), "unexpected end"),
        (0_u32.to_le_bytes().to_vec(), "does not contain any block"),
        (4097_u32.to_le_bytes().to_vec(), "implausible"),
        (vec![1, 0, 0, 0, 0, 0, 0, 0], "invalid name length"),
        (vec![1, 0, 0, 0, 5, 1, 0, 0], "invalid name length"),
        (vec![1, 0, 0, 0, 2, 0, 0, 0, 0xff, 0], "non UTF-8"),
        (vec![1, 0, 0, 0, 100, 0, 0, 0, b'x'], "end of file"),
        (trailing, "trailing bytes"),
        (bad_payload, "GFF block"),
        (
            common::container_with(&[("save.sav", &root_only())]),
            "player character",
        ),
    ];

    for (bytes, expected) in cases {
        let path = write_bytes(dir.path(), &bytes);
        let error = SaveGame::metadata(&path).expect_err("broken savegame was accepted");
        assert!(
            error.to_string().contains(expected),
            "expected {expected:?}, got {error}"
        );
    }

    assert!(SaveGame::metadata(dir.path().join("missing.sac")).is_err());
}

#[test]
fn parse_rejects_broken_containers() {
    let valid = common::container_with(&[("save.sav", &common::character_gff())]);
    let mut trailing = valid;
    trailing.push(0);

    let cases = [
        (Vec::new(), "end of file"),
        (0_u32.to_le_bytes().to_vec(), "does not contain any block"),
        (4097_u32.to_le_bytes().to_vec(), "implausible"),
        (vec![1, 0, 0, 0, 0, 0, 0, 0], "invalid name length"),
        (vec![1, 0, 0, 0, 100, 0, 0, 0, b'x'], "end of file"),
        (vec![1, 0, 0, 0, 2, 0, 0, 0, 0xff, 0], "non UTF-8"),
        (trailing, "trailing bytes"),
    ];

    for (bytes, expected) in cases {
        let error = SaveGame::parse(PathBuf::from("broken.sac"), bytes)
            .expect_err("broken container was accepted");
        assert!(
            error.to_string().contains(expected),
            "expected {expected:?}, got {error}"
        );
    }
}

#[test]
fn backup_paths_keep_the_target_name_and_handle_missing_names() {
    let backup = backup_path(Path::new("saves/save.sac"));
    let name = backup.file_name().expect("backup name").to_string_lossy();
    assert!(name.starts_with("save.sac.jadeforge-backup-"), "{name}");
    assert_eq!(backup.parent(), Some(Path::new("saves")));

    let root = backup_path(Path::new("/"));
    let name = root.file_name().expect("backup name").to_string_lossy();
    assert!(name.starts_with("save.sac.jadeforge-backup-"), "{name}");
}
