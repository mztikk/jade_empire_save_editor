#![allow(clippy::expect_used)] // tests fail loudly; expects are assertions
#![allow(clippy::assert_is_empty)] // empty checks read better than comparing with an empty slice

//! Inventory parsing and editing without real savegames.

mod common;

use jadeforge_core::inventory::read_items;
use jadeforge_core::{Error, Gff, SaveGame, Value};

fn push_item(gff: &mut Gff, count: Value, item_id: u16, kind: u8) -> usize {
    let item = gff.push_struct(0);
    gff.add_field(item, "Count", count).expect("count");
    gff.add_field(item, "ItemId", Value::Word(item_id))
        .expect("item id");
    gff.add_field(item, "Type", Value::Byte(kind))
        .expect("item type");
    item
}

fn with_item(count: Value) -> (Gff, usize) {
    let mut gff = common::character_gff();
    let player = common::player(&gff);
    let item = push_item(&mut gff, count, 42, 6);
    gff.add_field(player, "ItemList", Value::List(vec![item]))
        .expect("item list");
    (gff, item)
}

fn set_value(save: &mut SaveGame, path: &str, value: Value) {
    save.block_mut("save.sav")
        .expect("save block")
        .gff_mut()
        .get_mut(path)
        .expect("field")
        .value = value;
}

#[test]
fn missing_or_malformed_item_lists() {
    let save = common::save(&common::character_gff());
    assert!(save.items().expect("items").is_empty());

    let mut gff = common::character_gff();
    let player = common::player(&gff);
    gff.add_field(player, "ItemList", Value::Byte(1))
        .expect("wrong list type");
    let save = common::save(&gff);
    assert!(matches!(save.items(), Err(Error::TypeMismatch { .. })));

    let (gff, _) = with_item(Value::Byte(1));
    let mut save = common::save(&gff);
    set_value(&mut save, "PlayerCreature.ItemList", Value::List(vec![999]));
    assert!(matches!(save.items(), Err(Error::Corrupt(_))));
}

#[test]
fn items_use_defaults_for_missing_fields() {
    let mut gff = common::character_gff();
    let player = common::player(&gff);
    let item = gff.push_struct(0);
    gff.add_field(player, "ItemList", Value::List(vec![item]))
        .expect("item list");
    let save = common::save(&gff);
    let items = save.items().expect("items");
    assert_eq!(items.len(), 1);
    assert_eq!(items[0].struct_index, item);
    assert_eq!(items[0].item_id, 0);
    assert_eq!(items[0].count, 1);
    assert_eq!(items[0].kind, 0);
}

#[test]
fn set_item_count_keeps_word_fields_and_rejects_other_types() {
    let (gff, item) = with_item(Value::Word(1));
    let mut save = common::save(&gff);
    save.set_item_count(item, 200).expect("set word count");
    assert_eq!(
        save.block("save.sav")
            .expect("block")
            .gff()
            .get("PlayerCreature.ItemList[0].Count")
            .expect("count")
            .value,
        Value::Word(200)
    );
    assert_eq!(save.items().expect("items")[0].count, 200);

    let (gff, item) = with_item(Value::DWord(1));
    let mut save = common::save(&gff);
    assert!(matches!(
        save.set_item_count(item, 2),
        Err(Error::TypeMismatch { .. })
    ));

    let (mut gff, item) = with_item(Value::Byte(1));
    let _ = gff
        .remove("PlayerCreature.ItemList[0].Count")
        .expect("remove");
    let mut save = common::save(&gff);
    assert!(matches!(
        save.set_item_count(item, 2),
        Err(Error::FieldNotFound { .. })
    ));
    assert!(matches!(
        save.set_item_count(999, 2),
        Err(Error::Corrupt(_))
    ));
}

#[test]
fn failed_removal_does_not_dirty_a_clean_save() {
    let (gff, _) = with_item(Value::Byte(3));
    let mut save = common::save(&gff);
    assert!(!save.is_modified());
    assert!(!save.remove_item(999).expect("remove missing item"));
    assert!(!save.is_modified());

    set_value(&mut save, "PlayerCreature.ItemList", Value::Byte(1));
    assert!(matches!(
        save.remove_item(0),
        Err(Error::TypeMismatch { .. })
    ));
}

#[test]
fn add_item_rejects_a_non_list_item_list() {
    let mut gff = common::character_gff();
    let player = common::player(&gff);
    gff.add_field(player, "ItemList", Value::Byte(1))
        .expect("wrong list type");
    let mut save = common::save(&gff);
    assert!(matches!(
        save.add_item(1234, 6, 1),
        Err(Error::TypeMismatch { .. })
    ));
}

#[test]
fn read_items_reads_and_reports_errors() {
    let mut gff = common::empty_gff();
    let root = gff.push_struct(u32::MAX);
    let item = push_item(&mut gff, Value::Byte(3), 42, 6);
    gff.add_field(root, "Items", Value::List(vec![item]))
        .expect("items");
    let items = read_items(&gff, "Items").expect("read items");
    assert_eq!(items.len(), 1);
    assert_eq!(items[0].item_id, 42);
    assert_eq!(items[0].count, 3);
    assert_eq!(items[0].kind, 6);

    let mut default_item = common::empty_gff();
    let default_root = default_item.push_struct(u32::MAX);
    let entry = default_item.push_struct(0);
    default_item
        .add_field(default_root, "Items", Value::List(vec![entry]))
        .expect("items");
    let defaults = read_items(&default_item, "Items").expect("defaults");
    assert_eq!(defaults[0].item_id, 0);
    assert_eq!(defaults[0].count, 1);
    assert_eq!(defaults[0].kind, 0);

    let mut non_list = common::empty_gff();
    let non_list_root = non_list.push_struct(u32::MAX);
    non_list
        .add_field(non_list_root, "Items", Value::Byte(1))
        .expect("wrong type");
    assert!(matches!(
        read_items(&non_list, "Items"),
        Err(Error::TypeMismatch { .. })
    ));
    assert!(matches!(
        read_items(&non_list, "Missing"),
        Err(Error::FieldNotFound { .. })
    ));

    let mut dangling = common::empty_gff();
    let dangling_root = dangling.push_struct(u32::MAX);
    dangling
        .add_field(dangling_root, "Items", Value::List(vec![999]))
        .expect("dangling list");
    assert!(matches!(
        read_items(&dangling, "Items"),
        Err(Error::Corrupt(_))
    ));
}
