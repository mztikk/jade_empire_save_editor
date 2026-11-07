#![allow(clippy::expect_used)] // tests fail loudly; expects are assertions
#![allow(clippy::assert_is_empty)] // empty checks read better than comparing with an empty slice

//! Character editing without real savegames: range checks, type-preserving
//! writes and the legacy `polyplayer.sav` fallback.

mod common;

use jadeforge_core::save::POLYPLAYER_BLOCK;
use jadeforge_core::{
    Character, Error, Gff, MAX_ALIGNMENT, MAX_APPEARANCE, MAX_ATTRIBUTE_POINTS, MAX_CHAPTER,
    MAX_EXPERIENCE, MAX_LEVEL, MAX_STAT, MIN_ALIGNMENT, SaveGame, Value, locate_player,
};

fn changed(base: &Character, apply: impl FnOnce(&mut Character)) -> Character {
    let mut character = base.clone();
    apply(&mut character);
    character
}

fn value(gff: &Gff, path: &str) -> Value {
    gff.get(path).expect("field").value.clone()
}

fn assert_rejected(cases: &[(&str, Character)]) {
    let mut save = common::save(&common::character_gff());
    for (field, character) in cases {
        let error = save
            .apply_character(character)
            .expect_err("out of range character was accepted");
        assert!(error.to_string().contains(field), "{field}: {error}");
        assert!(!save.is_modified(), "{field} dirtied the savegame");
    }
}

#[test]
fn rejects_out_of_range_progression_values() {
    let base = common::character();
    assert_rejected(&[
        ("level", changed(&base, |character| character.level = 0)),
        (
            "level",
            changed(&base, |character| character.level = MAX_LEVEL + 1),
        ),
        (
            "experience",
            changed(&base, |character| {
                character.experience = MAX_EXPERIENCE + 1;
            }),
        ),
        (
            "gold",
            changed(&base, |character| character.gold = Some(MAX_STAT + 1)),
        ),
        (
            "attribute points",
            changed(&base, |character| {
                character.attribute_points = MAX_ATTRIBUTE_POINTS + 1;
            }),
        ),
        (
            "alignment",
            changed(&base, |character| character.alignment = MAX_ALIGNMENT + 1),
        ),
        (
            "alignment",
            changed(&base, |character| character.alignment = MIN_ALIGNMENT - 1),
        ),
        (
            "appearance",
            changed(&base, |character| character.appearance = -1),
        ),
        (
            "appearance",
            changed(&base, |character| {
                character.appearance = MAX_APPEARANCE + 1;
            }),
        ),
        (
            "chapter",
            changed(&base, |character| character.chapter = MAX_CHAPTER + 1),
        ),
        (
            "kills",
            changed(&base, |character| character.kills = MAX_STAT + 1),
        ),
    ]);
}

#[test]
fn rejects_out_of_range_vitals_and_skills() {
    let base = common::character();
    assert_rejected(&[
        ("body", changed(&base, |character| character.body = 1001)),
        ("mind", changed(&base, |character| character.mind = 1001)),
        (
            "spirit",
            changed(&base, |character| character.spirit = 1001),
        ),
        (
            "health",
            changed(&base, |character| character.health_max = 0),
        ),
        (
            "health",
            changed(&base, |character| character.health = f32::NAN),
        ),
        (
            "health",
            changed(&base, |character| character.health = 1_000_001.0),
        ),
        ("chi", changed(&base, |character| character.chi_max = 0)),
        ("chi", changed(&base, |character| character.chi = -1.0)),
        (
            "focus",
            changed(&base, |character| character.focus = f32::INFINITY),
        ),
        (
            "focus max",
            changed(&base, |character| character.focus_max = -1.0),
        ),
        (
            "focus max",
            changed(&base, |character| character.focus_max = 1_000_001.0),
        ),
        (
            "charm",
            changed(&base, |character| character.charm = 1_000_001),
        ),
        (
            "intuition",
            changed(&base, |character| character.intuition = -1_000_001),
        ),
        (
            "intimidate",
            changed(&base, |character| character.intimidate = 1_000_001),
        ),
        (
            "name",
            changed(&base, |character| character.name = "Bad\nName".to_owned()),
        ),
    ]);
}

#[test]
fn accepts_the_largest_values_the_game_tables_can_load() {
    let mut save = common::save(&common::character_gff());
    let mut character = save.character().expect("character");
    character.level = MAX_LEVEL;
    character.experience = MAX_EXPERIENCE;
    character.gold = Some(MAX_STAT);
    character.attribute_points = MAX_ATTRIBUTE_POINTS;
    character.kills = MAX_STAT;
    character.alignment = MAX_ALIGNMENT;
    character.appearance = MAX_APPEARANCE;
    character.chapter = MAX_CHAPTER;

    save.apply_character(&character)
        .expect("the highest loadable values must be writable");

    let reread = save.character().expect("character");
    assert_eq!(reread.level, MAX_LEVEL);
    assert_eq!(reread.experience, MAX_EXPERIENCE);
    assert_eq!(reread.gold, Some(MAX_STAT));
    assert_eq!(reread.attribute_points, MAX_ATTRIBUTE_POINTS);
    assert_eq!(reread.kills, MAX_STAT);
    assert_eq!(reread.alignment, MAX_ALIGNMENT);
    assert_eq!(reread.appearance, MAX_APPEARANCE);
    assert_eq!(reread.chapter, MAX_CHAPTER);
}

#[test]
fn an_unsigned_stat_value_still_opens_for_repair() {
    let mut gff = common::character_gff();
    common::replace_field(
        &mut gff,
        "PlayerCreature.Stats.Gold",
        Value::DWord(u32::MAX),
    );
    let mut save = common::save(&gff);
    let mut character = save.character().expect("character");
    assert_eq!(
        character.gold,
        Some(u32::MAX),
        "the raw value must stay readable so the user can repair it"
    );
    assert!(
        save.apply_character(&character).is_err(),
        "an out-of-range value must not be written back"
    );
    assert!(
        !save.is_modified(),
        "the rejected write dirtied the savegame"
    );

    character.gold = Some(MAX_STAT);
    save.apply_character(&character)
        .expect("repair the gold value");
    assert_eq!(save.character().expect("character").gold, Some(MAX_STAT));
}

#[test]
fn values_that_crash_the_game_still_open_for_repair() {
    let mut gff = common::character_gff();
    common::replace_field(&mut gff, "PlayerCreature.Level", Value::Byte(255));
    common::replace_field(&mut gff, "PlayerCreature.CurrentXP", Value::DWord(u32::MAX));
    common::replace_field(
        &mut gff,
        "PlayerCreature.Stats.UnassignedPts",
        Value::DWord(u32::MAX),
    );
    common::replace_field(
        &mut gff,
        "PlayerCreature.Stats.Appearance",
        Value::Int(1_000_000),
    );

    let mut save = common::save(&gff);
    let mut character = save.character().expect("character");
    assert_eq!(character.level, 255);
    assert_eq!(character.experience, u32::MAX);
    assert_eq!(character.attribute_points, u32::MAX);
    assert_eq!(character.appearance, 1_000_000);
    assert!(
        save.apply_character(&character).is_err(),
        "the crashing values must not be written back unchanged"
    );
    assert!(!save.is_modified());

    character.level = MAX_LEVEL;
    character.experience = MAX_EXPERIENCE;
    character.attribute_points = MAX_ATTRIBUTE_POINTS;
    character.appearance = MAX_APPEARANCE;
    save.apply_character(&character)
        .expect("repair every crashing value");

    let reread = save.character().expect("character");
    assert_eq!(reread.level, MAX_LEVEL);
    assert_eq!(reread.experience, MAX_EXPERIENCE);
    assert_eq!(reread.attribute_points, MAX_ATTRIBUTE_POINTS);
    assert_eq!(reread.appearance, MAX_APPEARANCE);
}

#[test]
fn appearance_updates_the_original_appearance_copy() {
    let mut save = common::save(&common::character_gff());
    let mut character = save.character().expect("character");
    character.appearance = 12;

    save.apply_character(&character)
        .expect("apply the appearance change");

    let gff = save.block("save.sav").expect("block").gff();
    assert_eq!(
        value(gff, "PlayerCreature.Stats.Appearance"),
        Value::Int(12)
    );
    assert_eq!(
        value(gff, "PlayerCreature.PlayerOrigApp"),
        Value::Int(12),
        "the game loads the model from PlayerOrigApp"
    );
}

#[test]
fn appearance_still_writes_when_the_original_copy_is_absent() {
    let mut gff = common::character_gff();
    gff.remove("PlayerCreature.PlayerOrigApp")
        .expect("remove original appearance");
    let mut save = common::save(&gff);
    let mut character = save.character().expect("character");
    character.appearance = 12;

    save.apply_character(&character)
        .expect("apply without the original copy");
    assert_eq!(
        value(
            save.block("save.sav").expect("block").gff(),
            "PlayerCreature.Stats.Appearance"
        ),
        Value::Int(12)
    );
}

#[test]
fn writes_preserve_existing_field_types() {
    let mut gff = common::character_gff();
    common::replace_field(&mut gff, "PlayerCreature.Level", Value::Word(7));
    common::replace_field(&mut gff, "PlayerCreature.CurrentXP", Value::Int64(1234));
    common::replace_field(&mut gff, "PlayerCreature.Stats.Gold", Value::DWord64(99));
    common::replace_field(
        &mut gff,
        "PlayerCreature.Stats.UnassignedPts",
        Value::Int(2),
    );
    common::replace_field(
        &mut gff,
        "PlayerCreature.Stats.HealthBase",
        Value::Short(150),
    );

    let mut character = Character::read(&gff, "PlayerCreature.").expect("read character");
    assert_eq!(character.level, 7);
    assert_eq!(character.experience, 1234);
    assert_eq!(character.gold, Some(99));
    assert_eq!(character.attribute_points, 2);
    assert_eq!(character.health_max, 150);

    character.level = 9;
    character.experience = 2000;
    character.gold = Some(100);
    character.attribute_points = 5;
    character.health_max = 200;
    character.health = 130.5;

    let changes = character
        .write(&mut gff, "PlayerCreature.")
        .expect("write character");
    assert!(
        changes
            .iter()
            .any(|change| change.field == "PlayerCreature.Level"),
        "level change missing: {changes:?}"
    );
    assert_eq!(value(&gff, "PlayerCreature.Level"), Value::Word(9));
    assert_eq!(value(&gff, "PlayerCreature.CurrentXP"), Value::Int64(2000));
    assert_eq!(
        value(&gff, "PlayerCreature.Stats.Gold"),
        Value::DWord64(100)
    );
    assert_eq!(
        value(&gff, "PlayerCreature.Stats.UnassignedPts"),
        Value::Int(5)
    );
    assert_eq!(
        value(&gff, "PlayerCreature.Stats.HealthBase"),
        Value::Short(200)
    );
    assert_eq!(
        value(&gff, "PlayerCreature.Stats.Health"),
        Value::Float(130.5)
    );

    // The same types have to survive a serialization round trip.
    let bytes = gff.to_bytes().expect("serialize");
    let parsed = Gff::parse("types", &bytes).expect("parse");
    assert_eq!(
        value(&parsed, "PlayerCreature.CurrentXP"),
        Value::Int64(2000)
    );
    let reread = Character::read(&parsed, "PlayerCreature.").expect("read back");
    assert_eq!(reread.level, 9);
    assert_eq!(reread.experience, 2000);
    assert_eq!(reread.gold, Some(100));
    assert_eq!(reread.attribute_points, 5);
    assert_eq!(reread.health_max, 200);
    assert!((reread.health - 130.5).abs() < f32::EPSILON);
}

#[test]
fn wrong_field_types_are_rejected() {
    let mut character = common::character();

    let mut gff = common::character_gff();
    common::replace_field(
        &mut gff,
        "PlayerCreature.Level",
        Value::String("seven".to_owned()),
    );
    let error = character
        .write(&mut gff, "PlayerCreature.")
        .expect_err("string level was accepted");
    assert!(matches!(error, Error::TypeMismatch { .. }), "{error}");

    let mut gff = common::character_gff();
    common::replace_field(&mut gff, "PlayerCreature.Stats.Health", Value::Word(120));
    character.health = 130.0;
    let error = character
        .write(&mut gff, "PlayerCreature.")
        .expect_err("word health was accepted");
    assert!(matches!(error, Error::TypeMismatch { .. }), "{error}");

    let mut gff = common::character_gff();
    common::replace_field(&mut gff, "PlayerCreature.FirstName", Value::Byte(1));
    let error = character
        .write(&mut gff, "PlayerCreature.")
        .expect_err("byte name was accepted");
    assert!(matches!(error, Error::TypeMismatch { .. }), "{error}");
}

#[test]
fn missing_fields_are_left_alone_except_unspent_points() {
    let mut gff = common::empty_gff();
    let root = gff.push_struct(u32::MAX);
    let player = gff.push_struct(0);
    gff.add_field(root, "PlayerCreature", Value::Struct(player))
        .expect("player");

    let mut character = common::character();
    character.gold = None;
    character.attribute_points = 0;
    character
        .write(&mut gff, "PlayerCreature.")
        .expect("write into an empty player");
    assert!(
        gff.struct_at(player).expect("player").fields.is_empty(),
        "a missing field must not be created"
    );

    // Unspent points are the one field that is created when it is missing.
    let mut gff = common::character_gff();
    let _ = gff
        .remove("PlayerCreature.Stats.UnassignedPts")
        .expect("remove unspent points");
    let mut character = common::character();
    character.attribute_points = 3;
    character
        .write(&mut gff, "PlayerCreature.")
        .expect("write unspent points");
    assert_eq!(
        value(&gff, "PlayerCreature.Stats.UnassignedPts"),
        Value::DWord(3)
    );
}

#[test]
fn a_save_without_a_string_first_name_has_no_player() {
    let mut gff = common::empty_gff();
    let root = gff.push_struct(u32::MAX);
    let player = gff.push_struct(0);
    gff.add_field(root, "PlayerCreature", Value::Struct(player))
        .expect("player");
    gff.add_field(player, "FirstName", Value::Byte(1))
        .expect("wrong name type");

    let save = common::save(&gff);
    assert!(save.player_location().is_err());
    assert!(!save.is_modified());
}

#[test]
fn polyplayer_saves_merge_the_campaign_block() {
    let mut polyplayer = common::empty_gff();
    let root = polyplayer.push_struct(u32::MAX);
    common::add_character_fields(&mut polyplayer, root);

    let mut global = common::empty_gff();
    let global_root = global.push_struct(u32::MAX);
    common::add_campaign(&mut global, global_root, 3, 42_000_000);

    let bytes = common::container_with(&[("polyplayer.sav", &polyplayer), ("save.sav", &global)]);
    let mut save = SaveGame::parse("polyplayer.sac".into(), bytes).expect("parse");

    let location = locate_player(&save).expect("player location");
    assert_eq!(location.block, POLYPLAYER_BLOCK);
    assert_eq!(location.prefix, "");

    let character = save.character().expect("character");
    assert_eq!(character.name, "Hero");
    assert_eq!(character.chapter, 3);
    assert_eq!(character.time_played, 42_000_000);

    let mut edited = character.clone();
    edited.body = 8;
    edited.chapter = 4;
    edited.time_played = 43_000_000;
    let changes = save.apply_character(&edited).expect("apply");
    assert!(!changes.is_empty());
    let reread = save.character().expect("read back");
    assert_eq!(reread.body, 8);
    assert_eq!(reread.chapter, 4);
    assert_eq!(reread.time_played, 43_000_000);

    // The edited save has to serialize and reparse.
    let bytes = save.to_bytes().expect("serialize");
    let reread_save = SaveGame::parse("polyplayer.sac".into(), bytes).expect("reparse");
    assert_eq!(reread_save.character().expect("character"), reread);
}

#[test]
fn an_unchanged_polyplayer_edit_does_not_dirty_the_save() {
    let mut polyplayer = common::empty_gff();
    let root = polyplayer.push_struct(u32::MAX);
    common::add_character_fields(&mut polyplayer, root);

    let mut global = common::empty_gff();
    let global_root = global.push_struct(u32::MAX);
    common::add_campaign(&mut global, global_root, 1, 1000);

    let bytes = common::container_with(&[("polyplayer.sav", &polyplayer), ("save.sav", &global)]);
    let mut save = SaveGame::parse("polyplayer.sac".into(), bytes).expect("parse");
    let character = save.character().expect("character");
    let changes = save.apply_character(&character).expect("apply");
    assert!(changes.is_empty(), "unchanged edit produced {changes:?}");
    assert!(!save.is_modified());
}

#[test]
fn wider_numeric_fields_are_rejected_instead_of_truncated() {
    let mut gff = common::character_gff();
    common::replace_field(
        &mut gff,
        "PlayerCreature.Stats.Gold",
        Value::DWord64(5_000_000_000),
    );
    let save = common::save(&gff);
    assert!(save.character().is_err());

    let mut gff = common::character_gff();
    common::replace_field(
        &mut gff,
        "PlayerCreature.CurrentXP",
        Value::Int64(5_000_000_000),
    );
    let save = common::save(&gff);
    assert!(save.character().is_err());
}

#[test]
fn a_missing_gold_field_is_created_when_the_user_edits_it() {
    let mut gff = common::character_gff();
    let _ = gff
        .remove("PlayerCreature.Stats.Gold")
        .expect("remove gold");
    let mut save = common::save(&gff);
    let mut character = save.character().expect("character");
    assert_eq!(character.gold, None);
    character.gold = Some(123);
    let changes = save.apply_character(&character).expect("apply gold");
    assert!(!changes.is_empty());
    assert_eq!(
        save.block("save.sav")
            .expect("block")
            .gff()
            .get("PlayerCreature.Stats.Gold")
            .expect("gold field")
            .value,
        Value::DWord(123)
    );
}

#[test]
fn a_failing_campaign_write_leaves_the_player_block_untouched() {
    let mut gff = common::character_gff();
    let mut character = Character::read(&gff, "PlayerCreature.").expect("character");
    common::replace_field(
        &mut gff,
        "Campaign.Chapter",
        Value::String("not a number".to_owned()),
    );
    let mut save = common::save(&gff);
    character.level = 8;

    assert!(matches!(
        save.apply_character(&character),
        Err(Error::TypeMismatch { .. })
    ));
    assert!(!save.is_modified());
    assert_eq!(
        save.block("save.sav")
            .expect("block")
            .gff()
            .get("PlayerCreature.Level")
            .expect("level")
            .value,
        Value::Byte(7)
    );
}

#[test]
fn names_are_limited_by_characters_not_utf8_bytes() {
    let mut gff = common::character_gff();
    let mut character = Character::read(&gff, "PlayerCreature.").expect("read");
    character.name = "é".repeat(64);
    character
        .write(&mut gff, "PlayerCreature.")
        .expect("64 multi-byte characters fit");

    character.name = "é".repeat(65);
    assert!(matches!(
        character.write(&mut gff, "PlayerCreature."),
        Err(Error::OutOfRange { .. })
    ));
}
