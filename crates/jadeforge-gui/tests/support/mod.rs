#![allow(clippy::panic)] // a broken in-test fixture should fail loudly
//! A self-contained synthetic installation for the headless UI tests.
//!
//! CI has no game installation and no savegames, so the UI tests build a small
//! but complete savegame and the tables needed to resolve its names here. The
//! fixture stays in a temporary directory and is removed with it.

use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use jadeforge_core::{Gff, Value};

const VIPER: u32 = 0;
const VIPER_DESC: u32 = 1;
const STYLE: u32 = 2;
const STYLE_DESC: u32 = 3;
const DAMAGE: u32 = 4;
const GEM: u32 = 5;
const GEM_DESC: u32 = 6;
const GENERIC_ITEM: u32 = 7;
const ADVANCE_DESC: u32 = 8;
const GENERIC_ITEM_DESC: u32 = 9;

/// Builds the installation and one `Test Save` below `root`.
pub fn synthetic(root: &Path) -> Option<(PathBuf, PathBuf)> {
    let game = root.join("Game");
    let save_dir = game.join("Save");
    let save_folder = save_dir.join("Test Save");
    std::fs::create_dir_all(&save_folder).ok()?;
    let override_dir = game.join("override");
    std::fs::create_dir_all(&override_dir).ok()?;

    let strings = [
        "Viper",
        "A poisoned blade.",
        "Test Style",
        "A test style.",
        "Damage",
        "Test Gem",
        "A test gem.",
        "Item",
        "This upgrade makes every strike hit harder and adds enough words that \
         the hover detail wraps like a real upgrade instead of collapsing \
         into a narrow column.",
        "A generic test item.",
    ];
    write_tlk(&game.join("dialog.tlk"), &strings)?;
    write(&override_dir.join("items.2da"), items_2da().as_bytes())?;
    write(
        &override_dir.join("stylesuperlist.2da"),
        styles_2da().as_bytes(),
    )?;
    write(
        &override_dir.join("styledata.2da"),
        style_data_2da().as_bytes(),
    )?;
    write(&override_dir.join("testlv.2da"), test_lv_2da().as_bytes())?;
    write(
        &override_dir.join("StyleAdvance.2da"),
        advances_2da().as_bytes(),
    )?;
    write(&override_dir.join("effects.2da"), effects_2da().as_bytes())?;
    write(&override_dir.join("gems.2da"), gems_2da().as_bytes())?;

    let save = save_folder.join("save.sac");
    let block = block();
    write(&save, &container(&block)?)?;
    write(
        &save_folder.join(jadeforge_core::screenshot::SAVE_IMAGE_NAME),
        &screenshot(64, 64),
    )?;
    Some((save, game))
}

fn items_2da() -> String {
    let mut table = format!(
        "2DA V2.0\n\n   LABEL STRREF_NAME STRREF_DESC TYPE COST ICON\n\
         42 Viper {VIPER} {VIPER_DESC} weapons 100 icon_viper\n\
         43 Gem_Test {GEM} {GEM_DESC} gems 50 ****\n"
    );
    for id in 0..60_u16 {
        if id == 42 || id == 43 {
            continue;
        }
        let _ = writeln!(
            table,
            "{id} Item_{id} {GENERIC_ITEM} {GENERIC_ITEM_DESC} weapons 1 ****"
        );
    }
    table
}

fn styles_2da() -> String {
    format!(
        "2DA V2.0\n\n   Label NameRef DescRef IconRef StyleData WeaponCount Weapon Type\n\
         21 Martial_Test {STYLE} {STYLE_DESC} icon_style 0 0 **** HAND\n\
         22 Other_Style {STYLE} {STYLE_DESC} **** 1 0 **** HAND\n\
         23 Third_Style {STYLE} {STYLE_DESC} **** 1 0 **** HAND\n"
    )
}

fn style_data_2da() -> String {
    String::from(
        "2DA V2.0\n\n   label useexcel levelup2da move2da advancement2da\n\
         0 test 0 testlv **** ****\n\
         1 other 0 testlv **** ****\n",
    )
}

fn test_lv_2da() -> String {
    format!(
        "2DA V2.0\n\n   Label Choice_1 Choice_2 Choice_3 Choice_4 Choice_5\n0 {DAMAGE} 1 2 3 4 5\n"
    )
}

/// Long advance names plus a short first effect word keep the hover detail
/// wide, which reproduces the squeeze the old single-row layout produced.
fn advances_2da() -> String {
    let mut table = String::from("2DA V2.0\n\n   Label DescRef CostToLevelUp Effect1 Param1\n");
    for row in 1..=5 {
        let _ = writeln!(table, "{row} Increase_Damage_{row} {ADVANCE_DESC} 1 84 0.5");
    }
    table
}

fn effects_2da() -> String {
    String::from("2DA V2.0\n\n   Label\n62 EFFECT_MODIFYBODYBASE\n")
}

fn gems_2da() -> String {
    String::from(
        "2DA V2.0\n\n   Label Effect0 Parameter0 Duration0 Effect1 Parameter1 Effect2 Parameter2\n\
         0 Gem_Test 62 2.0 0.0 **** **** **** ****\n",
    )
}

fn write_tlk(path: &Path, strings: &[&str]) -> Option<()> {
    let mut data = Vec::new();
    data.extend_from_slice(b"TLK V4.0");
    data.extend_from_slice(&2_u32.to_le_bytes());
    data.extend_from_slice(&u32::try_from(strings.len()).ok()?.to_le_bytes());
    data.extend_from_slice(&24_u32.to_le_bytes());
    let data_offset = 24 + 10 * strings.len();
    data.extend_from_slice(&u32::try_from(data_offset).ok()?.to_le_bytes());
    let mut text = Vec::new();
    for string in strings {
        let bytes = string.as_bytes();
        data.extend_from_slice(&0_u32.to_le_bytes());
        data.extend_from_slice(&u32::try_from(data_offset + text.len()).ok()?.to_le_bytes());
        data.extend_from_slice(&u16::try_from(bytes.len()).ok()?.to_le_bytes());
        text.extend_from_slice(bytes);
    }
    data.extend_from_slice(&text);
    write(path, &data)
}

fn write(path: &Path, bytes: &[u8]) -> Option<()> {
    std::fs::write(path, bytes).ok()
}

fn container(gff: &Gff) -> Option<Vec<u8>> {
    let block = gff.to_bytes().ok()?;
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&1_u32.to_le_bytes());
    bytes.extend_from_slice(&9_u32.to_le_bytes());
    bytes.extend_from_slice(b"save.sav\0");
    bytes.extend_from_slice(&u32::try_from(block.len()).ok()?.to_le_bytes());
    bytes.extend_from_slice(&block);
    Some(bytes)
}

/// A small 24-bit uncompressed TGA.
fn screenshot(width: u16, height: u16) -> Vec<u8> {
    let mut tga = vec![0_u8; 18];
    tga[2] = 2;
    tga[12..14].copy_from_slice(&width.to_le_bytes());
    tga[14..16].copy_from_slice(&height.to_le_bytes());
    tga[16] = 24;
    tga[17] = 0x20;
    tga.resize(18 + usize::from(width) * usize::from(height) * 3, 0x40);
    tga
}

fn add(gff: &mut Gff, structure: usize, label: &str, value: Value) {
    assert!(
        gff.add_field(structure, label, value).is_ok(),
        "fixture field {label} could not be added"
    );
}

#[allow(clippy::too_many_lines)] // one field per line keeps the fixture readable
fn block() -> Gff {
    let mut gff = Gff {
        version: *b"V3.2",
        labels: Vec::new(),
        structs: Vec::new(),
        root: 0,
    };
    let root = gff.push_struct(u32::MAX);
    let player = gff.push_struct(0);
    let stats = gff.push_struct(0);
    let focus = gff.push_struct(0);
    let progress = gff.push_struct(0);
    let campaign = gff.push_struct(0);
    let item = gff.push_struct(0);
    let other_item = gff.push_struct(0);
    let style = gff.push_struct(0);
    let other_style = gff.push_struct(0);
    let henchmen = gff.push_struct(0);
    let hench = gff.push_struct(0);
    let gem = gff.push_struct(0);
    let effect = gff.push_struct(0);
    let gem_entry = gff.push_struct(0);
    let amulet = gff.push_struct(0);
    let slots: Vec<usize> = (0..7).map(|_| gff.push_struct(0)).collect();

    add(&mut gff, root, "PlayerCreature", Value::Struct(player));
    add(&mut gff, root, "Campaign", Value::Struct(campaign));
    add(&mut gff, campaign, "Chapter", Value::DWord(2));
    add(
        &mut gff,
        campaign,
        "TimePlayed",
        Value::DWord64(90_000_000_000),
    );
    add(
        &mut gff,
        player,
        "FirstName",
        Value::String("Test Hero".to_owned()),
    );
    add(&mut gff, player, "Level", Value::Byte(7));
    add(&mut gff, player, "CurrentXP", Value::DWord(1234));
    add(&mut gff, player, "Stats", Value::Struct(stats));
    add(&mut gff, player, "Progress", Value::Struct(progress));
    add(&mut gff, stats, "Gold", Value::DWord(99));
    add(&mut gff, stats, "UnassignedPts", Value::DWord(2));
    add(&mut gff, stats, "Health", Value::Float(120.0));
    add(&mut gff, stats, "HealthBase", Value::DWord(150));
    add(&mut gff, stats, "Chi", Value::Float(80.0));
    add(&mut gff, stats, "ChiBase", Value::DWord(100));
    add(&mut gff, stats, "Focus", Value::Struct(focus));
    add(&mut gff, focus, "STAT_CUR", Value::Float(20.0));
    add(&mut gff, focus, "STAT_MAX", Value::Float(30.0));
    add(&mut gff, stats, "Body", Value::DWord(5));
    add(&mut gff, stats, "Mind", Value::DWord(6));
    add(&mut gff, stats, "Spirit", Value::DWord(7));
    add(&mut gff, stats, "Charm", Value::Int(1));
    add(&mut gff, stats, "Intuition", Value::Int(2));
    add(&mut gff, stats, "Intimidate", Value::Int(3));
    add(&mut gff, stats, "Alignment", Value::Int(200));
    add(&mut gff, stats, "Appearance", Value::Int(4));
    add(&mut gff, progress, "Kills", Value::DWord(42));

    add(
        &mut gff,
        player,
        "ItemList",
        Value::List(vec![item, other_item]),
    );
    add(&mut gff, item, "Count", Value::Byte(5));
    add(&mut gff, item, "ItemId", Value::Word(42));
    add(&mut gff, item, "Type", Value::Byte(6));
    add(&mut gff, other_item, "Count", Value::Byte(1));
    add(&mut gff, other_item, "ItemId", Value::Word(0));
    add(&mut gff, other_item, "Type", Value::Byte(5));

    add(
        &mut gff,
        stats,
        "Styles",
        Value::List(vec![style, other_style]),
    );
    add(&mut gff, style, "StyleId", Value::DWord(21));
    add(&mut gff, style, "StyleLevel", Value::Byte(1));
    add(&mut gff, style, "StyleLevelInfo", Value::DWord64(0));
    add(
        &mut gff,
        style,
        "NameRef",
        Value::Int(i32::try_from(STYLE).unwrap_or(0)),
    );
    add(
        &mut gff,
        style,
        "DescRef",
        Value::Int(i32::try_from(STYLE_DESC).unwrap_or(0)),
    );
    add(
        &mut gff,
        style,
        "IconRef",
        Value::String("icon_style".to_owned()),
    );
    add(&mut gff, style, "WeaponCount", Value::Int(0));
    add(&mut gff, style, "WeaponRef", Value::String(String::new()));
    add(&mut gff, style, "TimeSpentInStyle", Value::Float(0.0));
    add(&mut gff, style, "FirstLoad", Value::Int(0));
    add(
        &mut gff,
        style,
        "VisualEffectNode",
        Value::String(String::new()),
    );

    add(&mut gff, other_style, "StyleId", Value::DWord(22));
    add(&mut gff, other_style, "StyleLevel", Value::Byte(3));
    add(
        &mut gff,
        other_style,
        "StyleLevelInfo",
        Value::DWord64(0b11),
    );
    add(
        &mut gff,
        other_style,
        "NameRef",
        Value::Int(i32::try_from(STYLE).unwrap_or(0)),
    );
    add(
        &mut gff,
        other_style,
        "DescRef",
        Value::Int(i32::try_from(STYLE_DESC).unwrap_or(0)),
    );
    add(
        &mut gff,
        other_style,
        "IconRef",
        Value::String("icon_style".to_owned()),
    );
    add(&mut gff, other_style, "WeaponCount", Value::Int(0));
    add(
        &mut gff,
        other_style,
        "WeaponRef",
        Value::String(String::new()),
    );
    add(&mut gff, other_style, "TimeSpentInStyle", Value::Float(0.0));
    add(&mut gff, other_style, "FirstLoad", Value::Int(0));
    add(
        &mut gff,
        other_style,
        "VisualEffectNode",
        Value::String(String::new()),
    );

    add(&mut gff, player, "Henchmen", Value::Struct(henchmen));
    add(&mut gff, henchmen, "HenchList", Value::List(vec![hench]));
    add(
        &mut gff,
        hench,
        "BaseResref",
        Value::String("j00h_ho".to_owned()),
    );
    add(&mut gff, hench, "Level", Value::Byte(3));
    add(&mut gff, hench, "Available", Value::Byte(1));
    add(&mut gff, hench, "Available2", Value::Byte(1));
    add(&mut gff, hench, "State", Value::Byte(0));
    add(&mut gff, hench, "CombatTime", Value::Float(0.0));

    add(&mut gff, player, "Gems", Value::List(vec![gem_entry]));
    add(&mut gff, gem_entry, "Gem", Value::Struct(gem));
    add(&mut gff, gem, "ItemID", Value::Word(43));
    add(&mut gff, gem, "GemsEffectList", Value::List(vec![effect]));
    add(&mut gff, effect, "GemsEffectType", Value::DWord(62));
    add(&mut gff, effect, "GemsEffectValue", Value::DWord(2));
    add(&mut gff, effect, "GemsEffectCurVal", Value::DWord(2));
    add(&mut gff, effect, "GemsEffectDurati", Value::Float(0.0));

    add(&mut gff, player, "Amulet", Value::Struct(amulet));
    add(&mut gff, amulet, "Type", Value::Byte(2));
    add(&mut gff, amulet, "OpenGemsList", Value::List(slots));
    gff
}
