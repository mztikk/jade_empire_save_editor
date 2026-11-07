#![allow(clippy::expect_used)] // tests fail loudly; expects are assertions
#![allow(clippy::assert_is_empty)] // empty checks read better than comparing with an empty slice

//! Companions, built from synthetic blocks.

mod common;

use jadeforge_core::{Error, Gff, SaveGame, Value};

fn push_henchman(gff: &mut Gff, fields: &[(&str, Value)]) -> usize {
    let henchman = gff.push_struct(0);
    for (label, value) in fields {
        gff.add_field(henchman, label, value.clone())
            .expect("henchman field");
    }
    henchman
}

fn add_henchmen(gff: &mut Gff, members: Vec<usize>) {
    let player = common::player(gff);
    let henchmen = gff.push_struct(0);
    gff.add_field(player, "Henchmen", Value::Struct(henchmen))
        .expect("henchmen");
    gff.add_field(henchmen, "HenchList", Value::List(members))
        .expect("hench list");
}

fn field_value(save: &SaveGame, path: &str) -> Value {
    save.block("save.sav")
        .expect("block")
        .gff()
        .get(path)
        .expect("field")
        .value
        .clone()
}

#[test]
fn missing_or_malformed_henchman_lists() {
    let save = common::save(&common::character_gff());
    assert!(save.henchmen().expect("henchmen").is_empty());

    let mut gff = common::character_gff();
    let player = common::player(&gff);
    let henchmen_struct = gff.push_struct(0);
    gff.add_field(player, "Henchmen", Value::Struct(henchmen_struct))
        .expect("henchmen");
    gff.add_field(henchmen_struct, "HenchList", Value::Byte(1))
        .expect("wrong list type");
    let save = common::save(&gff);
    assert!(matches!(save.henchmen(), Err(Error::TypeMismatch { .. })));

    let mut gff = common::character_gff();
    let member = push_henchman(&mut gff, &[("Level", Value::Byte(1))]);
    add_henchmen(&mut gff, vec![member]);
    let mut save = common::save(&gff);
    save.block_mut("save.sav")
        .expect("block")
        .gff_mut()
        .get_mut("PlayerCreature.Henchmen.HenchList")
        .expect("list")
        .value = Value::List(vec![999]);
    assert!(matches!(save.henchmen(), Err(Error::Corrupt(_))));
}

#[test]
fn missing_henchman_fields_get_defaults() {
    let mut gff = common::character_gff();
    let member = push_henchman(&mut gff, &[]);
    add_henchmen(&mut gff, vec![member]);
    let save = common::save(&gff);
    let rows = save.henchmen().expect("henchmen");
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].resref, "");
    assert_eq!(rows[0].level, 1);
    assert!(!rows[0].available);
    assert!(!rows[0].available2);
    assert_eq!(rows[0].state, 0);
    assert!((rows[0].combat_time - 0.0).abs() < f32::EPSILON);
}

#[test]
fn henchman_fields_are_read() {
    let mut gff = common::character_gff();
    let member = push_henchman(
        &mut gff,
        &[
            ("BaseResref", Value::String("j00h_ho".to_owned())),
            ("Level", Value::Word(12)),
            ("Available", Value::Byte(1)),
            ("Available2", Value::Byte(0)),
            ("State", Value::Byte(2)),
            ("CombatTime", Value::Float(3.5)),
        ],
    );
    add_henchmen(&mut gff, vec![member]);
    let save = common::save(&gff);
    let rows = save.henchmen().expect("henchmen");
    assert_eq!(rows[0].resref, "j00h_ho");
    assert_eq!(rows[0].level, 12);
    assert!(rows[0].available);
    assert!(!rows[0].available2);
    assert_eq!(rows[0].state, 2);
    assert!((rows[0].combat_time - 3.5).abs() < f32::EPSILON);
}

#[test]
fn out_of_range_henchman_levels_are_rejected_when_read() {
    let mut gff = common::character_gff();
    let zero = push_henchman(&mut gff, &[("Level", Value::Byte(0))]);
    add_henchmen(&mut gff, vec![zero]);
    let save = common::save(&gff);
    assert!(save.henchmen().is_err());

    let mut gff = common::character_gff();
    let huge = push_henchman(&mut gff, &[("Level", Value::Word(999))]);
    add_henchmen(&mut gff, vec![huge]);
    let save = common::save(&gff);
    assert!(save.henchmen().is_err());
}

#[test]
fn henchman_setters_reject_a_foreign_struct() {
    let mut gff = common::character_gff();
    let member = push_henchman(&mut gff, &[("Level", Value::Byte(3))]);
    add_henchmen(&mut gff, vec![member]);
    let foreign = push_henchman(&mut gff, &[("Level", Value::Byte(1))]);
    let mut save = common::save(&gff);

    assert!(save.set_henchman_level(foreign, 12).is_err());
    assert!(!save.is_modified());
    assert!(save.set_henchman_available(foreign, true).is_err());
    assert!(!save.is_modified());
}

#[test]
fn set_henchman_level_keeps_field_types() {
    let cases = [
        (Value::Byte(3), Value::Byte(12)),
        (Value::Word(3), Value::Word(12)),
        (Value::Short(3), Value::Short(12)),
        (Value::Int(3), Value::Int(12)),
        (Value::DWord(3), Value::DWord(12)),
        (Value::DWord64(3), Value::DWord64(12)),
    ];
    for (stored, expected) in cases {
        let mut gff = common::character_gff();
        let henchman = push_henchman(&mut gff, &[("Level", stored)]);
        add_henchmen(&mut gff, vec![henchman]);
        let mut save = common::save(&gff);
        save.set_henchman_level(henchman, 12).expect("set level");
        assert_eq!(
            field_value(&save, "PlayerCreature.Henchmen.HenchList[0].Level"),
            expected
        );
    }

    let mut gff = common::character_gff();
    let henchman = push_henchman(&mut gff, &[]);
    add_henchmen(&mut gff, vec![henchman]);
    let mut save = common::save(&gff);
    save.set_henchman_level(henchman, 12).expect("add level");
    assert_eq!(
        field_value(&save, "PlayerCreature.Henchmen.HenchList[0].Level"),
        Value::DWord(12)
    );
}

#[test]
fn set_henchman_level_rejects_out_of_range_levels_and_unknown_structs() {
    let mut gff = common::character_gff();
    let henchman = push_henchman(&mut gff, &[("Level", Value::Byte(1))]);
    add_henchmen(&mut gff, vec![henchman]);
    let mut save = common::save(&gff);
    assert!(matches!(
        save.set_henchman_level(henchman, 0),
        Err(Error::OutOfRange { .. })
    ));
    assert!(matches!(
        save.set_henchman_level(henchman, 61),
        Err(Error::OutOfRange { .. })
    ));
    assert!(matches!(
        save.set_henchman_level(999, 12),
        Err(Error::Corrupt(_))
    ));
}

#[test]
fn availability_toggles_both_flags() {
    let mut gff = common::character_gff();
    let member = push_henchman(
        &mut gff,
        &[
            ("Available", Value::Byte(0)),
            ("Available2", Value::Byte(0)),
        ],
    );
    add_henchmen(&mut gff, vec![member]);
    let mut save = common::save(&gff);
    save.set_henchman_available(member, true)
        .expect("make available");
    let rows = save.henchmen().expect("henchmen");
    assert!(rows[0].available);
    assert!(rows[0].available2);
    save.set_henchman_available(member, false)
        .expect("make unavailable");
    let rows = save.henchmen().expect("henchmen");
    assert!(!rows[0].available);
    assert!(!rows[0].available2);
}

#[test]
fn availability_can_create_missing_fields() {
    let mut gff = common::character_gff();
    let henchman = push_henchman(&mut gff, &[]);
    add_henchmen(&mut gff, vec![henchman]);
    let mut save = common::save(&gff);
    save.set_henchman_available(henchman, true)
        .expect("add flags");
    assert_eq!(
        field_value(&save, "PlayerCreature.Henchmen.HenchList[0].Available"),
        Value::DWord(1)
    );
    assert!(matches!(
        save.set_henchman_available(999, true),
        Err(Error::Corrupt(_))
    ));
}

#[test]
fn combat_mode_is_validated_and_keeps_the_type() {
    let mut gff = common::character_gff();
    let henchman = push_henchman(&mut gff, &[("CombatMode", Value::Byte(1))]);
    add_henchmen(&mut gff, vec![henchman]);
    let mut save = common::save(&gff);
    save.set_henchman_combat_mode(henchman, 2)
        .expect("set mode");
    assert_eq!(
        field_value(&save, "PlayerCreature.Henchmen.HenchList[0].CombatMode"),
        Value::Byte(2)
    );
    save.set_henchman_combat_mode(henchman, 0)
        .expect("reset mode");
    assert_eq!(
        field_value(&save, "PlayerCreature.Henchmen.HenchList[0].CombatMode"),
        Value::Byte(0)
    );
    assert!(matches!(
        save.set_henchman_combat_mode(henchman, 3),
        Err(Error::OutOfRange { .. })
    ));
    assert!(matches!(
        save.set_henchman_combat_mode(999, 1),
        Err(Error::Corrupt(_))
    ));
}

#[test]
fn combat_mode_can_create_a_missing_field() {
    let mut gff = common::character_gff();
    let henchman = push_henchman(&mut gff, &[]);
    add_henchmen(&mut gff, vec![henchman]);
    let mut save = common::save(&gff);
    save.set_henchman_combat_mode(henchman, 2)
        .expect("add mode");
    assert_eq!(
        field_value(&save, "PlayerCreature.Henchmen.HenchList[0].CombatMode"),
        Value::DWord(2)
    );
}
