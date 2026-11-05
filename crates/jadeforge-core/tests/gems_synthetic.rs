#![allow(clippy::expect_used)] // tests fail loudly; expects are assertions
#![allow(clippy::assert_is_empty)] // empty checks read better than comparing with an empty slice

//! Gems and the amulet, built from synthetic blocks.

mod common;

use jadeforge_core::{Error, GemLocation, Gff, SaveGame, Value, sockets_for_tier};

fn push_effect(gff: &mut Gff, kind: u32, value: u32, duration: f32) -> usize {
    let effect = gff.push_struct(0);
    gff.add_field(effect, "GemsEffectType", Value::DWord(kind))
        .expect("effect type");
    gff.add_field(effect, "GemsEffectValue", Value::DWord(value))
        .expect("effect value");
    gff.add_field(effect, "GemsEffectCurVal", Value::DWord(value))
        .expect("effect current");
    gff.add_field(effect, "GemsEffectDurati", Value::Float(duration))
        .expect("effect duration");
    effect
}

fn push_gem(gff: &mut Gff, item_id: u16, effects: &[(u32, u32, f32)]) -> usize {
    let gem = gff.push_struct(0);
    gff.add_field(gem, "ItemID", Value::Word(item_id))
        .expect("item id");
    let effect_indices: Vec<usize> = effects
        .iter()
        .map(|(kind, value, duration)| push_effect(gff, *kind, *value, *duration))
        .collect();
    gff.add_field(gem, "GemsEffectList", Value::List(effect_indices))
        .expect("effect list");
    gem
}

fn push_entry(gff: &mut Gff, gem: usize) -> usize {
    let entry = gff.push_struct(0);
    gff.add_field(entry, "Gem", Value::Struct(gem))
        .expect("gem");
    entry
}

fn add_amulet(gff: &mut Gff, player: usize, tier: Option<u8>, active: bool) -> usize {
    let amulet = gff.push_struct(0);
    gff.add_field(player, "Amulet", Value::Struct(amulet))
        .expect("amulet");
    if let Some(tier) = tier {
        gff.add_field(amulet, "Type", Value::Byte(tier))
            .expect("amulet tier");
    }
    if active {
        gff.add_field(amulet, "Active", Value::Byte(1))
            .expect("amulet active");
    }
    amulet
}

fn seven_sockets(gff: &mut Gff, entries: &[(usize, usize)]) -> Vec<usize> {
    let mut slots: Vec<usize> = (0..7).map(|_| gff.push_struct(0)).collect();
    for (position, entry) in entries {
        slots[*position] = *entry;
    }
    slots
}

fn set_field(save: &mut SaveGame, path: &str, value: Value) {
    save.block_mut("save.sav")
        .expect("save block")
        .gff_mut()
        .get_mut(path)
        .expect("field")
        .value = value;
}

#[test]
fn sockets_follow_the_amulet_tier() {
    assert_eq!(sockets_for_tier(0), 3);
    assert_eq!(sockets_for_tier(1), 3);
    assert_eq!(sockets_for_tier(2), 5);
    assert_eq!(sockets_for_tier(3), 7);
    assert_eq!(sockets_for_tier(4), 7);
}

#[test]
fn the_amulet_needs_a_type_or_active_field() {
    let save = common::save(&common::character_gff());
    assert_eq!(save.amulet().expect("amulet"), None);

    let mut gff = common::character_gff();
    let player = common::player(&gff);
    add_amulet(&mut gff, player, None, true);
    let save = common::save(&gff);
    assert_eq!(
        save.amulet().expect("amulet"),
        Some(jadeforge_core::AmuletInfo {
            tier: 1,
            sockets: 3,
        })
    );

    let mut gff = common::character_gff();
    let player = common::player(&gff);
    add_amulet(&mut gff, player, Some(4), false);
    let save = common::save(&gff);
    assert_eq!(
        save.amulet().expect("amulet"),
        Some(jadeforge_core::AmuletInfo {
            tier: 4,
            sockets: 7,
        })
    );
}

#[test]
fn the_amulet_tier_is_clamped() {
    let mut gff = common::character_gff();
    let player = common::player(&gff);
    add_amulet(&mut gff, player, Some(1), false);
    let mut save = common::save(&gff);

    for tier in [0_u8, 1, 2, 3, 4, 5, 99, 255] {
        set_field(&mut save, "PlayerCreature.Amulet.Type", Value::Byte(tier));
        let amulet = save.amulet().expect("amulet").expect("present");
        assert_eq!(amulet.tier, tier.clamp(1, 4), "tier {tier}");
        assert_eq!(amulet.sockets, sockets_for_tier(amulet.tier));
    }
}

#[test]
fn gems_are_read_from_both_lists() {
    let mut gff = common::character_gff();
    let player = common::player(&gff);
    let inventory_gem = push_gem(&mut gff, 10, &[(62, 2, 0.0)]);
    let inventory_entry = push_entry(&mut gff, inventory_gem);
    gff.add_field(player, "Gems", Value::List(vec![inventory_entry]))
        .expect("inventory gems");

    let amulet = add_amulet(&mut gff, player, Some(2), false);
    let socketed_gem = push_gem(&mut gff, 11, &[(65, 4, 1.5)]);
    let socketed_entry = push_entry(&mut gff, socketed_gem);
    gff.add_field(amulet, "OpenGemsList", Value::List(vec![socketed_entry]))
        .expect("open gems");

    let save = common::save(&gff);
    let gems = save.gems().expect("gems");
    assert_eq!(gems.len(), 2);
    assert_eq!(gems[0].item_id, 10);
    assert_eq!(gems[0].location, GemLocation::Inventory);
    assert_eq!(gems[0].effects.len(), 1);
    assert_eq!(gems[0].effects[0].kind, 62);
    assert_eq!(gems[0].effects[0].value, 2);
    assert_eq!(gems[1].item_id, 11);
    assert_eq!(gems[1].location, GemLocation::Amulet);
    assert_eq!(gems[1].effects[0].kind, 65);
    assert!((gems[1].effects[0].duration - 1.5).abs() < f32::EPSILON);
}

#[test]
fn missing_and_malformed_gem_lists_do_not_fail() {
    let save = common::save(&common::character_gff());
    assert!(save.gems().expect("gems").is_empty());

    let mut gff = common::character_gff();
    let player = common::player(&gff);
    gff.add_field(player, "Gems", Value::Byte(1))
        .expect("wrong inventory type");
    let amulet = add_amulet(&mut gff, player, Some(1), false);
    gff.add_field(amulet, "OpenGemsList", Value::Byte(1))
        .expect("wrong socket type");
    let save = common::save(&gff);
    assert!(save.gems().expect("gems").is_empty());

    let mut gff = common::character_gff();
    let player = common::player(&gff);
    let gem = push_gem(&mut gff, 10, &[]);
    let entry = push_entry(&mut gff, gem);
    gff.add_field(player, "Gems", Value::List(vec![entry]))
        .expect("gems");
    let mut save = common::save(&gff);
    set_field(&mut save, "PlayerCreature.Gems", Value::List(vec![999]));
    assert!(save.gems().expect("gems").is_empty());
}

#[test]
fn a_gem_without_a_nested_gem_field_uses_the_entry() {
    let mut gff = common::character_gff();
    let player = common::player(&gff);
    let entry = gff.push_struct(0);
    gff.add_field(entry, "ItemID", Value::Word(12))
        .expect("item id");
    gff.add_field(player, "Gems", Value::List(vec![entry]))
        .expect("gems");
    let save = common::save(&gff);
    let gems = save.gems().expect("gems");
    assert_eq!(gems.len(), 1);
    assert_eq!(gems[0].item_id, 12);
    assert!(gems[0].effects.is_empty());
}

#[test]
fn read_effects_skips_missing_structs() {
    let mut gff = common::character_gff();
    let player = common::player(&gff);
    let gem = push_gem(&mut gff, 10, &[(62, 2, 0.0)]);
    let entry = push_entry(&mut gff, gem);
    gff.add_field(player, "Gems", Value::List(vec![entry]))
        .expect("gems");
    let mut save = common::save(&gff);
    set_field(
        &mut save,
        "PlayerCreature.Gems[0].Gem.GemsEffectList",
        Value::List(vec![999]),
    );
    let gems = save.gems().expect("gems");
    assert_eq!(gems.len(), 1);
    assert!(gems[0].effects.is_empty());
}

#[test]
fn add_gem_creates_the_inventory_list_and_effects() {
    let mut save = common::save(&common::character_gff());
    let entry = save
        .add_gem(434, &[(62, 2), (65, 1)], 2.5)
        .expect("add gem");
    let gems = save.gems().expect("gems");
    assert_eq!(gems.len(), 1);
    assert_eq!(gems[0].struct_index, entry);
    assert_eq!(gems[0].item_id, 434);
    assert_eq!(gems[0].location, GemLocation::Inventory);
    assert_eq!(gems[0].effects.len(), 2);
    assert_eq!(gems[0].effects[0].kind, 62);
    assert_eq!(gems[0].effects[0].value, 2);
    assert!((gems[0].effects[0].duration - 2.5).abs() < f32::EPSILON);

    save.to_bytes().expect("serialize after add");
}

#[test]
fn move_gem_swaps_entries_and_keeps_seven_sockets() {
    let mut gff = common::character_gff();
    let player = common::player(&gff);
    let gem = push_gem(&mut gff, 10, &[]);
    let entry = push_entry(&mut gff, gem);
    gff.add_field(player, "Gems", Value::List(vec![entry]))
        .expect("gems");
    let amulet = add_amulet(&mut gff, player, Some(4), false);
    let slots = seven_sockets(&mut gff, &[]);
    gff.add_field(amulet, "OpenGemsList", Value::List(slots))
        .expect("open gems");
    let mut save = common::save(&gff);

    assert_eq!(
        save.move_gem(entry).expect("move to amulet"),
        GemLocation::Amulet
    );
    let amulet_list = save
        .block("save.sav")
        .expect("block")
        .gff()
        .get("PlayerCreature.Amulet.OpenGemsList")
        .expect("list")
        .value
        .as_list()
        .expect("list value")
        .to_vec();
    assert_eq!(amulet_list.len(), 7);
    assert_eq!(amulet_list[0], entry);
    assert!(
        save.gems()
            .expect("gems")
            .iter()
            .any(|gem| { gem.struct_index == entry && gem.location == GemLocation::Amulet })
    );

    assert_eq!(
        save.move_gem(entry).expect("move back"),
        GemLocation::Inventory
    );
    let amulet_list = save
        .block("save.sav")
        .expect("block")
        .gff()
        .get("PlayerCreature.Amulet.OpenGemsList")
        .expect("list")
        .value
        .as_list()
        .expect("list value")
        .to_vec();
    assert_eq!(amulet_list.len(), 7);
    assert_ne!(amulet_list[0], entry);
    assert!(
        save.gems()
            .expect("gems")
            .iter()
            .any(|gem| { gem.struct_index == entry && gem.location == GemLocation::Inventory })
    );
    save.to_bytes().expect("serialize after move");
}

#[test]
fn moving_into_a_full_amulet_keeps_the_source_gem() {
    let mut gff = common::character_gff();
    let player = common::player(&gff);
    let gem = push_gem(&mut gff, 10, &[]);
    let entry = push_entry(&mut gff, gem);
    gff.add_field(player, "Gems", Value::List(vec![entry]))
        .expect("gems");
    let amulet = add_amulet(&mut gff, player, Some(1), false);
    let mut occupied = Vec::new();
    for _ in 0..3 {
        let gem = push_gem(&mut gff, 11, &[]);
        occupied.push(push_entry(&mut gff, gem));
    }
    let slots = seven_sockets(
        &mut gff,
        &occupied
            .iter()
            .enumerate()
            .map(|(index, entry)| (index, *entry))
            .collect::<Vec<_>>(),
    );
    gff.add_field(amulet, "OpenGemsList", Value::List(slots))
        .expect("open gems");
    let mut save = common::save(&gff);
    let before = save.to_bytes().expect("before");

    assert!(save.move_gem(entry).is_err());
    assert_eq!(save.to_bytes().expect("after"), before);
    assert!(!save.is_modified());
    assert!(
        save.gems()
            .expect("gems")
            .iter()
            .any(|gem| gem.struct_index == entry && gem.location == GemLocation::Inventory)
    );
}

#[test]
fn moving_without_an_amulet_is_rejected_and_keeps_the_gem() {
    let mut gff = common::character_gff();
    let player = common::player(&gff);
    let gem = push_gem(&mut gff, 10, &[]);
    let entry = push_entry(&mut gff, gem);
    gff.add_field(player, "Gems", Value::List(vec![entry]))
        .expect("gems");
    let mut save = common::save(&gff);
    let before = save.to_bytes().expect("before");

    assert!(save.move_gem(entry).is_err());
    assert_eq!(save.to_bytes().expect("after"), before);
    assert!(!save.is_modified());
}

#[test]
fn remove_gem_updates_the_list_and_reports_unknown_gems() {
    let mut gff = common::character_gff();
    let player = common::player(&gff);
    let gem = push_gem(&mut gff, 10, &[]);
    let entry = push_entry(&mut gff, gem);
    gff.add_field(player, "Gems", Value::List(vec![entry]))
        .expect("gems");
    let mut save = common::save(&gff);

    assert!(matches!(save.remove_gem(999), Err(Error::Corrupt(_))));
    save.remove_gem(entry).expect("remove gem");
    assert!(save.gems().expect("gems").is_empty());
    save.to_bytes().expect("serialize after remove");
}

#[test]
fn remove_gem_from_the_amulet() {
    let mut gff = common::character_gff();
    let player = common::player(&gff);
    let amulet = add_amulet(&mut gff, player, Some(2), false);
    let gem = push_gem(&mut gff, 10, &[]);
    let entry = push_entry(&mut gff, gem);
    let slots = seven_sockets(&mut gff, &[(0, entry)]);
    gff.add_field(amulet, "OpenGemsList", Value::List(slots))
        .expect("open gems");
    let mut save = common::save(&gff);

    assert!(
        save.gems()
            .expect("gems")
            .iter()
            .any(|gem| { gem.struct_index == entry && gem.location == GemLocation::Amulet })
    );
    save.remove_gem(entry).expect("remove socketed gem");
    let list = save
        .block("save.sav")
        .expect("block")
        .gff()
        .get("PlayerCreature.Amulet.OpenGemsList")
        .expect("list")
        .value
        .as_list()
        .expect("list value")
        .to_vec();
    assert_eq!(list.len(), 7);
    assert!(!list.contains(&entry));
    assert!(
        !save
            .gems()
            .expect("gems")
            .iter()
            .any(|gem| gem.item_id == 10)
    );
}
