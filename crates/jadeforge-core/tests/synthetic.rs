#![allow(clippy::expect_used)] // tests fail loudly; expects are assertions

//! Tests that build savegames from scratch.
//!
//! These run on any machine (including CI) because they do not need a copy of
//! the game: the blocks are assembled programmatically, written, read back and
//! compared.

use jadeforge_core::gff::Value;
use jadeforge_core::{Gff, LocString, SaveGame};

/// Builds a small block with nested structs, a list, every scalar type and
/// text fields.
fn sample_block() -> Gff {
    let mut gff = Gff {
        version: *b"V3.2",
        labels: vec!["Tag".to_owned(), "Position".to_owned()],
        structs: vec![],
        root: 0,
    };

    // Reserve the labels first: labels are stored by index.
    let label_tag = gff.intern_label("Tag").expect("label");
    let label_position = gff.intern_label("Position").expect("label");
    let label_stats = gff.intern_label("Stats").expect("label");
    let label_items = gff.intern_label("ItemList").expect("label");
    let label_health = gff.intern_label("Health").expect("label");
    let label_name = gff.intern_label("Name").expect("label");
    let label_count = gff.intern_label("Count").expect("label");
    let label_note = gff.intern_label("MapNote").expect("label");
    let label_orientation = gff.intern_label("Orientation").expect("label");
    let label_blob = gff.intern_label("Blob").expect("label");
    let label_big = gff.intern_label("Big").expect("label");

    let root = gff.push_struct(u32::MAX);
    let stats = gff.push_struct(0);
    let item_a = gff.push_struct(0);
    let item_b = gff.push_struct(0);

    gff.add_field(root, "Tag", Value::String("PLAYER".to_owned()))
        .expect("tag");
    gff.add_field(root, "Position", Value::Vector3([1.5, -2.25, 0.0]))
        .expect("position");
    gff.add_field(root, "Orientation", Value::Quaternion([0.0, 0.0, 0.0, 1.0]))
        .expect("orientation");
    gff.add_field(root, "Stats", Value::Struct(stats))
        .expect("stats");
    gff.add_field(root, "ItemList", Value::List(vec![item_a, item_b]))
        .expect("items");
    gff.add_field(
        root,
        "MapNote",
        Value::LocString(LocString::from_text("hello")),
    )
    .expect("locstring");
    gff.add_field(root, "Blob", Value::Void(vec![1, 2, 3, 4, 5]))
        .expect("blob");
    gff.add_field(root, "Big", Value::DWord64(0x1122_3344_5566_7788))
        .expect("dword64");

    gff.add_field(stats, "Health", Value::Float(123.5))
        .expect("health");
    gff.add_field(stats, "Name", Value::ResRef("j01_beach".to_owned()))
        .expect("name");

    gff.add_field(item_a, "Count", Value::Byte(3))
        .expect("count");
    gff.add_field(item_b, "Count", Value::Word(1000))
        .expect("count");

    let _ = (
        label_tag,
        label_position,
        label_stats,
        label_items,
        label_health,
    );
    let _ = (
        label_name,
        label_count,
        label_note,
        label_orientation,
        label_blob,
        label_big,
    );
    gff
}

#[test]
fn synthetic_block_round_trips() {
    let gff = sample_block();
    gff.validate("test").expect("valid block");
    let bytes = gff.to_bytes().expect("serialize");
    let parsed = Gff::parse("test", &bytes).expect("parse");
    assert_eq!(parsed.labels, gff.labels);
    assert_eq!(parsed.structs, gff.structs);
    assert_eq!(parsed.root, gff.root);

    // Serializing the parsed block again must produce the same bytes: the
    // writer is deterministic.
    let again = parsed.to_bytes().expect("serialize again");
    assert_eq!(bytes, again);
}

#[test]
fn single_field_structs_are_stored_inline() {
    let mut gff = sample_block();
    let index = gff.push_struct(9);
    gff.add_field(index, "Count", Value::Byte(7))
        .expect("count");
    let bytes = gff.to_bytes().expect("serialize");
    let parsed = Gff::parse("test", &bytes).expect("parse");
    assert_eq!(
        parsed.structs[index].fields.len(),
        1,
        "struct should have a single field"
    );
    parsed.validate("test").expect("valid");
}

#[test]
fn editing_a_field_survives_a_round_trip() {
    let mut gff = sample_block();
    gff.set("Stats.Health", Value::Float(500.0)).expect("set");
    let bytes = gff.to_bytes().expect("serialize");
    let parsed = Gff::parse("test", &bytes).expect("parse");
    let field = parsed.get("Stats.Health").expect("field");
    assert_eq!(field.value, Value::Float(500.0));
}

#[test]
fn fields_can_be_added_and_removed() {
    let mut gff = sample_block();
    gff.add("Stats.UnassignedPts", Value::DWord(5))
        .expect("add");
    let bytes = gff.to_bytes().expect("serialize");
    let mut parsed = Gff::parse("test", &bytes).expect("parse");
    assert_eq!(
        parsed
            .get("Stats.UnassignedPts")
            .expect("field")
            .value
            .as_u64(),
        Some(5)
    );
    let removed = parsed.remove("Stats.UnassignedPts").expect("remove");
    assert_eq!(removed.value.as_u64(), Some(5));
    assert!(parsed.get("Stats.UnassignedPts").is_err());
}

#[test]
fn wrong_types_are_rejected() {
    let mut gff = sample_block();
    let error = gff
        .set("Stats.Health", Value::DWord(1))
        .expect_err("type mismatch");
    assert!(error.to_string().contains("expected"), "{error}");
}

#[test]
fn paths_reach_nested_lists() {
    let gff = sample_block();
    assert_eq!(
        gff.get("ItemList[0].Count").expect("path").value.as_u64(),
        Some(3)
    );
    assert_eq!(
        gff.get("ItemList[1].Count").expect("path").value.as_u64(),
        Some(1000)
    );
    assert!(gff.get("ItemList[9].Count").is_err());
    assert!(gff.get("Nope").is_err());
}

#[test]
fn save_container_round_trips() {
    let block = sample_block();
    let bytes = block.to_bytes().expect("serialize");
    let mut container = Vec::new();
    container.extend_from_slice(&1_u32.to_le_bytes());
    container.extend_from_slice(&9_u32.to_le_bytes());
    container.extend_from_slice(b"save.sav\0");
    container.extend_from_slice(&(bytes.len() as u32).to_le_bytes());
    container.extend_from_slice(&bytes);

    let save = SaveGame::parse("test.sac".into(), container.clone()).expect("parse container");
    assert_eq!(save.blocks().len(), 1);
    assert_eq!(save.blocks()[0].name, "save.sav");
    assert_eq!(save.to_bytes().expect("serialize"), container);
}

#[test]
fn broken_files_are_rejected() {
    // Empty file.
    assert!(SaveGame::parse("x".into(), Vec::new()).is_err());

    // Zero blocks.
    assert!(SaveGame::parse("x".into(), 0_u32.to_le_bytes().to_vec()).is_err());

    // Truncated block name.
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&1_u32.to_le_bytes());
    bytes.extend_from_slice(&200_u32.to_le_bytes());
    bytes.extend_from_slice(b"short");
    let error = SaveGame::parse("x".into(), bytes).expect_err("truncated");
    assert!(error.to_string().contains("end of file"), "{error}");

    // A block with the wrong magic.
    let mut bytes = Vec::new();
    let payload = vec![0_u8; 64];
    bytes.extend_from_slice(&1_u32.to_le_bytes());
    bytes.extend_from_slice(&8_u32.to_le_bytes());
    bytes.extend_from_slice(b"save.sav");
    bytes.extend_from_slice(&(payload.len() as u32).to_le_bytes());
    bytes.extend_from_slice(&payload);
    let error = SaveGame::parse("x".into(), bytes).expect_err("bad magic");
    assert!(error.to_string().contains("magic"), "{error}");
}

#[test]
fn json_export_contains_the_fields() {
    let gff = sample_block();
    let json = jadeforge_core::json::to_json(&gff);
    let text = serde_json::to_string(&json).expect("json");
    assert!(text.contains("\"Tag\":\"PLAYER\""), "{text}");
    assert!(text.contains("ItemList"), "{text}");
    assert!(text.contains("hello"), "{text}");
}

#[test]
fn unicode_text_is_preserved() {
    let mut gff = sample_block();
    gff.set("Tag", Value::String("Jade Ödland".to_owned()))
        .expect("set");
    let bytes = gff.to_bytes().expect("serialize");
    let parsed = Gff::parse("test", &bytes).expect("parse");
    assert_eq!(
        parsed.get("Tag").expect("tag").value.as_str(),
        Some("Jade Ödland")
    );
}

#[test]
fn structures_keep_their_insertion_order() {
    let gff = sample_block();
    let labels: Vec<&str> = gff
        .structs
        .iter()
        .flat_map(|structure| {
            structure
                .fields
                .iter()
                .map(|field| gff.label_name(field.label).unwrap_or("?"))
        })
        .collect();
    assert_eq!(labels.first(), Some(&"Tag"));
    let bytes = gff.to_bytes().expect("serialize");
    let parsed = Gff::parse("test", &bytes).expect("parse");
    let parsed_labels: Vec<String> = parsed
        .structs
        .iter()
        .flat_map(|structure| {
            structure
                .fields
                .iter()
                .map(|field| parsed.label_name(field.label).unwrap_or("?").to_owned())
        })
        .collect();
    let labels: Vec<String> = labels.iter().map(|label| (*label).to_owned()).collect();
    assert_eq!(labels, parsed_labels);
}
