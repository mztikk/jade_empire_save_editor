#![allow(clippy::expect_used)] // tests fail loudly; expects are assertions
#![allow(clippy::assert_is_empty)] // empty checks read better than comparing with an empty slice

//! Fighting styles, built from synthetic blocks.

mod common;

use jadeforge_core::{Error, Gff, MAX_STYLE_LEVEL, SaveGame, StyleIdentity, Value};

fn push_style(gff: &mut Gff, style_id: u16, level: Option<u8>, mask: Option<u64>) -> usize {
    let style = gff.push_struct(0);
    gff.add_field(style, "StyleId", Value::DWord(u32::from(style_id)))
        .expect("style id");
    if let Some(level) = level {
        gff.add_field(style, "StyleLevel", Value::Byte(level))
            .expect("style level");
    }
    if let Some(mask) = mask {
        gff.add_field(style, "StyleLevelInfo", Value::DWord64(mask))
            .expect("style mask");
    }
    style
}

fn add_styles(gff: &mut Gff, styles: Vec<usize>) {
    let stats = common::stats(gff);
    gff.add_field(stats, "Styles", Value::List(styles))
        .expect("styles");
}

fn style_level(save: &SaveGame, index: usize) -> (u8, u64) {
    let style = save
        .styles()
        .expect("styles")
        .into_iter()
        .find(|style| style.struct_index == index)
        .expect("style");
    (style.level, style.mask)
}

fn identity(style_id: u16) -> StyleIdentity {
    StyleIdentity {
        style_id,
        name_ref: 42,
        desc_ref: Some(99),
        icon: Some("I_NEW".to_owned()),
        weapon: Some("J00_NEW_WEAPON".to_owned()),
        weapon_count: 2,
    }
}

#[test]
fn missing_or_malformed_styles() {
    let save = common::save(&common::character_gff());
    assert!(save.styles().expect("styles").is_empty());

    let mut gff = common::character_gff();
    let stats = common::stats(&gff);
    gff.add_field(stats, "Styles", Value::Byte(1))
        .expect("wrong type");
    let save = common::save(&gff);
    assert!(matches!(save.styles(), Err(Error::TypeMismatch { .. })));

    let mut gff = common::character_gff();
    let style = push_style(&mut gff, 1, Some(1), Some(0));
    add_styles(&mut gff, vec![style]);
    let mut save = common::save(&gff);
    save.block_mut("save.sav")
        .expect("block")
        .gff_mut()
        .get_mut("PlayerCreature.Stats.Styles")
        .expect("styles")
        .value = Value::List(vec![999]);
    assert!(matches!(save.styles(), Err(Error::Corrupt(_))));
}

#[test]
fn style_level_defaults_from_the_mask_and_is_clamped() {
    let mut gff = common::character_gff();
    let empty = push_style(&mut gff, 1, None, None);
    let masked = push_style(&mut gff, 2, None, Some(0b111));
    let zero = push_style(&mut gff, 3, Some(0), Some(0));
    add_styles(&mut gff, vec![empty, masked, zero]);
    let save = common::save(&gff);
    let styles = save.styles().expect("styles");
    assert_eq!(styles[0].level, 1);
    assert_eq!(styles[0].mask, 0);
    assert_eq!(styles[1].level, 4);
    assert_eq!(styles[1].mask, 0b111);
    assert_eq!(styles[2].level, 1);
    assert_eq!(styles[0].weapon_count, 0);
    assert!(styles[0].weapon.is_none());
    assert!(styles[0].icon.is_none());
}

#[test]
fn style_identity_fields_are_read_from_the_savegame() {
    let mut gff = common::character_gff();
    let style = push_style(&mut gff, 7, Some(4), Some(0b11));
    gff.add_field(style, "IconRef", Value::ResRef("I_FIRE".to_owned()))
        .expect("icon");
    gff.add_field(style, "WeaponRef", Value::String("J00_STAFF".to_owned()))
        .expect("weapon");
    gff.add_field(style, "WeaponCount", Value::Word(2))
        .expect("weapon count");
    gff.add_field(style, "TimeSpentInStyle", Value::Float(125.5))
        .expect("time spent");
    add_styles(&mut gff, vec![style]);

    let save = common::save(&gff);
    let styles = save.styles().expect("styles");
    assert_eq!(styles[0].icon.as_deref(), Some("I_FIRE"));
    assert_eq!(styles[0].weapon.as_deref(), Some("J00_STAFF"));
    assert_eq!(styles[0].weapon_count, 2);
    assert!((styles[0].time_spent - 125.5).abs() < f32::EPSILON);
}

#[test]
fn styles_ignore_a_non_dword64_mask_when_reading() {
    let mut gff = common::character_gff();
    let style = push_style(&mut gff, 1, Some(3), None);
    gff.add_field(style, "StyleLevelInfo", Value::Word(0b111))
        .expect("wrong mask type");
    add_styles(&mut gff, vec![style]);
    let save = common::save(&gff);
    assert_eq!(style_level(&save, style), (3, 0));
}

#[test]
fn style_upgrades_reject_a_foreign_struct() {
    let mut gff = common::character_gff();
    let style = push_style(&mut gff, 1, Some(1), Some(0));
    add_styles(&mut gff, vec![style]);
    let foreign = gff.push_struct(0);
    let mut save = common::save(&gff);
    let before = save.to_bytes().expect("before");

    assert!(save.set_style_family_slot(foreign, &[1], 1).is_err());
    assert!(!save.is_modified());
    assert_eq!(save.to_bytes().expect("after"), before);
}

#[test]
fn a_family_buys_and_removes_a_gap_free_prefix() {
    let mut gff = common::character_gff();
    let style = push_style(&mut gff, 1, Some(1), Some(0));
    add_styles(&mut gff, vec![style]);
    let mut save = common::save(&gff);

    assert_eq!(
        save.set_style_family_slot(style, &[1, 2, 3], 2)
            .expect("buy two"),
        2
    );
    assert_eq!(style_level(&save, style), (3, 0b11));
    assert!(save.is_modified());

    assert_eq!(
        save.set_style_family_slot(style, &[1, 2, 3], 1)
            .expect("remove all"),
        2
    );
    assert_eq!(style_level(&save, style), (1, 0));
    save.to_bytes().expect("serialize");
}

#[test]
fn a_gap_in_a_family_is_rejected_without_changing_the_save() {
    let mut gff = common::character_gff();
    let style = push_style(&mut gff, 1, Some(3), Some(0b101));
    add_styles(&mut gff, vec![style]);
    let mut save = common::save(&gff);
    let before = save.to_bytes().expect("before");

    assert!(matches!(
        save.set_style_family_slot(style, &[1, 2, 3], 3),
        Err(Error::Corrupt(_))
    ));
    assert!(!save.is_modified());
    assert_eq!(save.to_bytes().expect("after"), before);
}

#[test]
fn a_wrong_mask_type_is_rejected_without_dirtying() {
    let mut gff = common::character_gff();
    let style = push_style(&mut gff, 1, Some(1), None);
    gff.add_field(style, "StyleLevelInfo", Value::Word(0))
        .expect("wrong mask type");
    add_styles(&mut gff, vec![style]);
    let mut save = common::save(&gff);
    let before = save.to_bytes().expect("before");

    assert!(matches!(
        save.set_style_family_slot(style, &[1, 2], 1),
        Err(Error::TypeMismatch { .. })
    ));
    assert!(!save.is_modified());
    assert_eq!(save.to_bytes().expect("after"), before);
}

#[test]
fn family_slot_validation_rejects_bad_arguments() {
    let mut gff = common::character_gff();
    let style = push_style(&mut gff, 1, Some(1), Some(0));
    add_styles(&mut gff, vec![style]);
    let mut save = common::save(&gff);

    assert!(save.set_style_family_slot(999, &[1], 1).is_err());
    assert!(matches!(
        save.set_style_family_slot(style, &[], 1),
        Err(Error::Corrupt(_))
    ));
    assert!(matches!(
        save.set_style_family_slot(style, &[1, 1], 1),
        Err(Error::Corrupt(_))
    ));
    assert!(matches!(
        save.set_style_family_slot(style, &[0], 0),
        Err(Error::OutOfRange { .. })
    ));
    assert!(matches!(
        save.set_style_family_slot(style, &[1], 2),
        Err(Error::Corrupt(_))
    ));
    assert!(!save.is_modified());
}

#[test]
fn adding_a_style_completes_identity_and_clears_resistances() {
    let mut gff = common::character_gff();
    let template = push_style(&mut gff, 1, Some(1), Some(0));
    let resist = gff.push_struct(0);
    gff.add_field(template, "ResistList", Value::List(vec![resist]))
        .expect("resist list");
    add_styles(&mut gff, vec![template]);
    let mut save = common::save(&gff);

    let added = save.add_style(template, &identity(2)).expect("add style");
    let gff = save.block("save.sav").expect("block").gff();
    let added_style = gff.struct_at(added).expect("added style");
    let field = |name: &str| {
        added_style
            .fields
            .iter()
            .find(|field| gff.label_name(field.label) == Some(name))
            .expect(name)
    };
    assert_eq!(field("StyleId").type_id, 4);
    assert_eq!(field("StyleId").value, Value::DWord(2));
    assert_eq!(field("NameRef").type_id, 5);
    assert_eq!(field("NameRef").value, Value::Int(42));
    assert_eq!(field("DescRef").type_id, 5);
    assert_eq!(field("DescRef").value, Value::Int(99));
    assert_eq!(field("IconRef").type_id, 10);
    assert_eq!(field("IconRef").value, Value::String("I_NEW".to_owned()));
    assert_eq!(field("WeaponRef").type_id, 10);
    assert_eq!(
        field("WeaponRef").value,
        Value::String("J00_NEW_WEAPON".to_owned())
    );
    assert_eq!(field("WeaponCount").type_id, 5);
    assert_eq!(field("WeaponCount").value, Value::Int(2));
    assert_eq!(
        field("ResistList").value,
        Value::List(Vec::new()),
        "the learned style inherited the template resistances"
    );

    let style = save
        .styles()
        .expect("styles")
        .into_iter()
        .find(|style| style.struct_index == added)
        .expect("new style");
    assert_eq!(style.weapon.as_deref(), Some("J00_NEW_WEAPON"));
    assert_eq!(style.weapon_count, 2);
    save.to_bytes().expect("serialize");
}

#[test]
fn adding_a_style_preserves_types_and_uses_the_requested_identity() {
    let mut gff = common::character_gff();
    let template = push_style(&mut gff, 1, Some(4), Some(0b1111));
    gff.add_field(template, "NameRef", Value::Int(7))
        .expect("name ref");
    gff.add_field(template, "DescRef", Value::Int(8))
        .expect("description ref");
    gff.add_field(template, "IconRef", Value::String("template".to_owned()))
        .expect("icon ref");
    gff.add_field(template, "WeaponRef", Value::String("template".to_owned()))
        .expect("weapon ref");
    gff.add_field(template, "WeaponCount", Value::Int(9))
        .expect("weapon count");
    gff.add_field(template, "TimeSpentInStyle", Value::Float(12.5))
        .expect("time");
    gff.add_field(template, "FirstLoad", Value::Int(1))
        .expect("first load");
    add_styles(&mut gff, vec![template]);
    let mut save = common::save(&gff);

    let added = save.add_style(template, &identity(2)).expect("add style");
    let styles = save.styles().expect("styles");
    assert_eq!(styles.len(), 2);
    let new_style = styles
        .iter()
        .find(|style| style.struct_index == added)
        .expect("added style");
    assert_eq!(new_style.style_id, 2);
    assert_eq!(new_style.name_ref, Some(42));
    assert_eq!(new_style.desc_ref, Some(99));
    assert_eq!(new_style.icon.as_deref(), Some("I_NEW"));
    assert_eq!(new_style.level, 1);
    assert_eq!(new_style.mask, 0);
    assert!((new_style.time_spent - 0.0).abs() < f32::EPSILON);

    let gff = save.block("save.sav").expect("block").gff();
    let added_style = gff.struct_at(added).expect("new style");
    let field = |name: &str| {
        added_style
            .fields
            .iter()
            .find(|field| gff.label_name(field.label) == Some(name))
            .expect(name)
    };
    assert_eq!(field("NameRef").type_id, 5);
    assert_eq!(field("NameRef").value, Value::Int(42));
    assert_eq!(field("FirstLoad").type_id, 5);
    assert_eq!(field("FirstLoad").value, Value::Int(0));
    assert_eq!(field("WeaponCount").type_id, 5);
    assert_eq!(
        field("WeaponRef").value,
        Value::String("J00_NEW_WEAPON".to_owned())
    );
    assert_eq!(field("StyleId").value, Value::DWord(2));
    assert_eq!(field("StyleLevel").value, Value::Byte(1));

    assert!(save.remove_style(added).expect("remove"));
    assert!(!save.remove_style(added).expect("second remove"));
    assert_eq!(save.styles().expect("styles").len(), 1);
    save.to_bytes().expect("serialize");
}

#[test]
fn add_style_rejects_a_missing_or_foreign_template() {
    let mut gff = common::character_gff();
    let style = push_style(&mut gff, 1, Some(1), Some(0));
    add_styles(&mut gff, vec![style]);
    let mut save = common::save(&gff);
    assert!(matches!(
        save.add_style(999, &identity(2)),
        Err(Error::Corrupt(_))
    ));
    assert!(!save.is_modified());

    let mut gff = common::character_gff();
    let style = push_style(&mut gff, 1, Some(1), Some(0));
    add_styles(&mut gff, vec![style]);
    let other = gff.push_struct(0);
    let mut save = common::save(&gff);
    assert!(matches!(
        save.add_style(other, &identity(2)),
        Err(Error::Corrupt(_))
    ));
    assert!(!save.is_modified());
}

#[test]
fn the_highest_family_slot_cannot_push_the_level_past_the_cap() {
    let mut gff = common::character_gff();
    let style = push_style(&mut gff, 1, Some(1), Some(0));
    add_styles(&mut gff, vec![style]);
    let mut save = common::save(&gff);
    let slots: Vec<u16> = (1..=64).collect();

    let changed = save
        .set_style_family_slot(style, &slots, 63)
        .expect("buy up to 63");
    assert_eq!(changed, 63);
    assert_eq!(
        style_level(&save, style),
        (MAX_STYLE_LEVEL, (1_u64 << 63) - 1)
    );
    let before = save.to_bytes().expect("before");
    assert!(matches!(
        save.set_style_family_slot(style, &slots, 64),
        Err(Error::OutOfRange { .. })
    ));
    assert_eq!(save.to_bytes().expect("after"), before);
}

#[test]
fn remove_style_rejects_a_non_list_styles_field() {
    let mut gff = common::character_gff();
    let stats = common::stats(&gff);
    gff.add_field(stats, "Styles", Value::Byte(1))
        .expect("wrong type");
    let mut save = common::save(&gff);
    assert!(matches!(
        save.remove_style(0),
        Err(Error::TypeMismatch { .. })
    ));
    assert!(!save.is_modified());
}
