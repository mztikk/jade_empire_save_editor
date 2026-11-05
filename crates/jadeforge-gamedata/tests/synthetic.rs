#![allow(clippy::expect_used)] // tests fail loudly; expects are assertions
#![allow(clippy::assert_is_empty)] // empty checks read better than an empty-slice comparison

//! A complete synthetic installation: talk table, text tables in `override/`
//! and a binary table in a RIM archive.

mod common;

use common::{TempDir, binary_2da, rim, write, write_text, write_tlk};
use jadeforge_gamedata::gamedata::family_and_tier;
use jadeforge_gamedata::rim::TYPE_2DA;
use jadeforge_gamedata::{
    GameData, RimArchive, StyleKind, TalkTable, is_game_root, pretty_effect_name,
};
use std::fmt::Write as _;

const STRINGS: &[&str] = &[
    "Iron Sword",
    "A sturdy blade.",
    "Martial Strike",
    "A basic martial style.",
    "Bought an upgrade",
    "Gem of Body",
    "A gem that raises Body.",
    "Damage",
    "Upgrades label",
    "Body",
    "Damage armor",
    "Temp Item",
    "Cut Item",
    "****",
    "Gem of Mind",
    "A gem that raises Mind.",
    "Weapon Style",
    "Weapon style description.",
    "Upgrade One",
    "",
    "Second Sword",
    "No Category",
    "Mystery Item",
    "Empty Label",
];

fn items_2da() -> String {
    String::from(
        "2DA V2.0\n\n   LABEL STRREF_NAME STRREF_DESC TYPE COST ICON\n\
         0  Iron_Sword 0 1 weapons 100 icon_sword\n\
         1  Gem_Body 5 6 gems 50 ****\n\
         2  Gem_Mind 14 15 gems 75 ****\n\
         3  Temp_Item 11 **** weapons 1 ****\n\
         4  Cut_Item 12 **** plot_items 1 ****\n\
         5  Missing_Name **** 1 weapons 1 ****\n\
         6  Star_Name 13 1 weapons 1 ****\n\
         7  Bad_Ref abc 1 weapons 1 ****\n\
         8  No_Category 21 1 **** 1 ****\n\
         9  Mystery 22 1 mystery 1 ****\n\
         10 **** 23 1 weapons 1 ****\n\
         11 Second_Sword 20 1 weapons 200 ****\n\
         12 zzz_Item 11 **** weapons 1 ****\n\
         13 Test_Item 11 **** weapons 1 ****\n\
         70000 Too_Big 0 1 weapons 1 ****\n",
    )
}

fn styles_2da() -> String {
    String::from(
        "2DA V2.0\n\n   Label NameRef DescRef IconRef StyleData WeaponCount Weapon Type\n\
         0  Martial_Strike 2 3 icon_martial 0 1 wpn_martial HAND\n\
         1  Weapon_Test 16 17 **** 1 2 wpn_sword MONSTER\n\
         2  Bad_NameRef abc 3 **** 0 0 **** HAND\n\
         3  No_Name 999 3 **** 0 0 **** HAND\n\
         4  Star_Name 13 3 **** 0 0 **** HAND\n\
         5  Huge_Ref 4294967296 3 **** 0 0 **** HAND\n\
         6  Empty_Name 19 3 **** 0 0 **** HAND\n\
         7  Support_Test 2 3 **** 2 0 **** Support\n\
         8  No_Table 2 3 **** 2 0 **** HAND\n\
         9  Missing_Data 2 3 **** 99 0 **** HAND\n\
         10 Empty_LevelTable 2 3 **** 3 0 **** HAND\n\
         11 No_Choices 2 3 **** 4 0 **** HAND\n\
         12 Empty_TableName 2 3 **** 5 0 **** HAND\n\
         70000 Big_Style 2 3 **** 0 0 **** HAND\n",
    )
}

fn style_data_2da() -> String {
    String::from(
        "2DA V2.0\n\n   label useexcel levelup2da move2da advancement2da\n\
         0 martial 0 martiallv **** ****\n\
         1 big 0 biglv **** ****\n\
         2 none 0 **** **** ****\n\
         3 missing 0 missinglv **** ****\n\
         4 nocolumns 0 nocolumnslv **** ****\n         5 empty 0 \"\" **** ****\n",
    )
}

fn martial_lv_2da() -> String {
    String::from(
        "2DA V2.0\n\n   Choices Label Choice_1 Choice_2 Choice_3 Choice_4 Choice_5\n\
         0 2 7 1 2 **** **** ****\n\
         1 0 **** 0 70000 3 **** ****\n\
         2 5 18 4 5 6 7 8\n",
    )
}

fn big_lv_2da() -> String {
    let mut table =
        String::from("2DA V2.0\n\n   Choices Label Choice_1 Choice_2 Choice_3 Choice_4 Choice_5\n");
    for row in 0..15 {
        let _ = writeln!(table, "{row} 0 **** 1 2 3 4 5");
    }
    table
}

fn advances_2da() -> String {
    let mut table = String::from("2DA V2.0\n\n   Label DescRef CostToLevelUp Effect1 Param1\n");
    for (row, effect, param) in [
        (1, 72, 0.25),
        (2, 72, 0.5),
        (3, 85, 0.5),
        (4, 86, 0.25),
        (5, 87, 2.5),
        (6, 69, -0.1),
        (7, 70, 0.15),
        (8, 84, 0.2),
        (9, 81, 0.1),
        (10, 99, 1.5),
    ] {
        let _ = writeln!(table, "{row} Increase_Upgrade_{row} 4 1 {effect} {param}");
    }
    table.push_str(
        "11 **** 4 1 72 0.1\n\
         12 INVALID 4 1 72 0.1\n\
         13 **** 4 1 72 0.1\n\
         15 No_Tier 4 1 72 0.1\n\
         16 Increase_Damage_x 4 1 72 0.1\n\
         70000 Too_Big 4 1 72 0.1\n",
    );
    table
}

fn effects_2da() -> String {
    String::from(
        "2DA V2.0\n\n   Label\n\
         60 EFFECT_MODIFYBODYBASE\n\
         61 EFFECT_INVALID\n\
         62 ****\n\
         63 EFFECT_DAMAGEARMOR\n\
         64 EFFECT_MODIFYMIND\n",
    )
}

fn gems_2da() -> String {
    String::from(
        "2DA V2.0\n\n   Label Effect0 Parameter0 Duration0 Effect1 Parameter1 Effect2 Parameter2\n\
         0 Gem_Body 62 2.5 3.0 65 -1.0 0 1.0\n\
         1 Gem_Mind 63 1.0 0.0 **** **** **** ****\n\
         2 No_Item 62 1.0 0.0 **** **** **** ****\n",
    )
}

fn installation() -> TempDir {
    let dir = TempDir::new("synthetic");
    write_tlk(&dir.join("dialog.tlk"), STRINGS);
    let override_dir = dir.join("override");
    write_text(&override_dir.join("items.2da"), &items_2da());
    write_text(&override_dir.join("stylesuperlist.2da"), &styles_2da());
    write_text(&override_dir.join("styledata.2da"), &style_data_2da());
    write_text(&override_dir.join("martiallv.2da"), &martial_lv_2da());
    write_text(&override_dir.join("biglv.2da"), &big_lv_2da());
    write_text(
        &override_dir.join("nocolumnslv.2da"),
        "2DA V2.0\n\n   Label Choice_1\n0 0 1\n",
    );
    write_text(&override_dir.join("styleadvance.2da"), &advances_2da());
    write_text(&override_dir.join("effects.2da"), &effects_2da());
    write_text(&override_dir.join("gems.2da"), &gems_2da());
    write_text(&override_dir.join("notes.txt"), "ignored");
    dir
}

fn rim_installation(invalid_override: bool) -> TempDir {
    let dir = TempDir::new("rim");
    write_tlk(&dir.join("dialog.tlk"), &["Iron Sword", "A sturdy blade."]);
    let override_dir = dir.join("override");
    if invalid_override {
        write_text(&override_dir.join("items.2da"), "this is not a 2DA table");
    } else {
        write_text(
            &override_dir.join("items.2da"),
            "2DA V2.0\n\n   LABEL STRREF_NAME STRREF_DESC TYPE COST ICON\n\
             0 Text_Sword 0 1 weapons 5 ****\n",
        );
    }
    let table = binary_2da(
        &[
            "LABEL",
            "STRREF_NAME",
            "STRREF_DESC",
            "TYPE",
            "COST",
            "ICON",
        ],
        &[("0", vec!["Rim_Sword", "0", "1", "weapons", "5", ""])],
    );
    write(
        &dir.join("data").join("miniglobal-a.rim"),
        &rim(&[("items", TYPE_2DA, &table)]),
    );
    dir
}

#[test]
fn resolves_items_and_playable_state() {
    let dir = installation();
    let game = GameData::open(dir.path()).expect("load installation");
    assert_eq!(game.root(), dir.path());
    assert_eq!(game.item_count(), 11);
    assert_eq!(game.playable_item_count(), 7);
    assert_eq!(game.items().count(), 11);

    let sword = game.item(0).expect("sword");
    assert_eq!(sword.name, "Iron Sword");
    assert_eq!(sword.label, "Iron_Sword");
    assert_eq!(sword.description.as_deref(), Some("A sturdy blade."));
    assert_eq!(sword.category.as_deref(), Some("weapons"));
    assert_eq!(sword.save_type, Some(5));
    assert_eq!(sword.cost, 100);
    assert_eq!(sword.icon.as_deref(), Some("icon_sword"));

    let gem = game.item(1).expect("gem item");
    assert_eq!(gem.name, "Gem of Body");
    assert_eq!(gem.category.as_deref(), Some("gems"));
    assert_eq!(gem.save_type, Some(1));
    assert_eq!(gem.icon, None);
    assert_eq!(game.item(8).expect("no category").category, None);
    assert_eq!(game.item(8).expect("no category").save_type, None);
    assert_eq!(
        game.item(9).expect("mystery").category.as_deref(),
        Some("mystery")
    );
    assert_eq!(game.item(9).expect("mystery").save_type, None);
    assert_eq!(game.item(10).expect("empty label").label, "");
    assert_eq!(game.item(11).expect("second").name, "Second Sword");
    assert_eq!(game.item(3).expect("temp").name, "Temp Item");
    assert!(game.item(5).is_none());
    assert!(game.item(6).is_none());
    assert!(game.item(7).is_none());
    assert!(game.item(u16::MAX).is_none());
}

#[test]
fn resolves_styles_and_advances() {
    let dir = installation();
    let game = GameData::open(dir.path()).expect("load installation");

    assert_eq!(game.style_count(), 8);
    assert_eq!(game.styles().count(), 8);
    let martial = game.style(0).expect("martial");
    assert_eq!(martial.name, "Martial Strike");
    assert_eq!(martial.name_ref, 2);
    assert_eq!(martial.label, "Martial_Strike");
    assert_eq!(martial.kind, StyleKind::Martial);
    assert_eq!(martial.weapon_count, 1);
    assert_eq!(martial.weapon.as_deref(), Some("wpn_martial"));
    assert_eq!(martial.icon.as_deref(), Some("icon_martial"));
    assert_eq!(
        martial.description.as_deref(),
        Some("A basic martial style.")
    );
    let weapon = game.style(1).expect("weapon");
    assert_eq!(weapon.kind, StyleKind::Weapon);
    assert_eq!(weapon.weapon_count, 2);
    assert_eq!(weapon.icon, None);
    assert_eq!(game.style(7).expect("support").kind, StyleKind::Support);
    assert!(game.style(2).is_none());
    assert!(game.style(5).is_none());

    assert_eq!(game.advances().count(), 12);
    let damage = game.advance(1).expect("damage");
    assert_eq!(damage.name, "Bought an upgrade");
    assert_eq!(damage.cost, 1);
    assert_eq!(damage.effect, 72);
    assert_eq!(damage.family, "Increase_Upgrade");
    assert_eq!(damage.tier, 1);
    assert_eq!(damage.effect_text(), "Damage +25%");
    assert_eq!(
        game.advance(2).expect("damage 2").effect_text(),
        "Damage +50%"
    );
    assert_eq!(game.advance(3).expect("range").effect_text(), "Range +50%");
    assert_eq!(
        game.advance(4).expect("duration").effect_text(),
        "Duration +25%"
    );
    assert_eq!(
        game.advance(5).expect("resistance").effect_text(),
        "Resistance +2.5"
    );
    assert_eq!(game.advance(6).expect("chi").effect_text(), "Chi cost -10%");
    assert_eq!(
        game.advance(7).expect("focus").effect_text(),
        "Focus cost +15%"
    );
    assert_eq!(
        game.advance(8).expect("chi strike").effect_text(),
        "Chi strike +20%"
    );
    assert_eq!(game.advance(9).expect("speed").effect_text(), "Speed +10%");
    assert_eq!(
        game.advance(10).expect("unknown").effect_text(),
        "Effect 99 (1.5)"
    );
    assert_eq!(game.advance(15).expect("no tier").tier, 0);
    assert_eq!(game.advance(15).expect("no tier").family, "No_Tier");
    assert_eq!(game.advance(16).expect("bad tier").tier, 0);
    assert_eq!(
        game.advance(16).expect("bad tier").family,
        "Increase_Damage_x"
    );
    assert!(game.advance(11).is_none());
    assert!(game.advance(12).is_none());
    assert!(game.advance(13).is_none());
    assert!(game.advance(14).is_none());
    assert!(game.advance(u16::MAX).is_none());

    assert_eq!(game.effect_name(60), Some("Body"));
    assert_eq!(game.effect_name(63), Some("Damage armor"));
    assert_eq!(game.effect_name(64), Some("Mind"));
    assert_eq!(game.effect_name(61), None);
    assert_eq!(game.effect_name(62), None);
    assert_eq!(game.effect_name(999), None);
}

#[test]
fn resolves_predefined_gems() {
    let dir = installation();
    let game = GameData::open(dir.path()).expect("load installation");
    assert_eq!(game.gems().len(), 2);
    assert_eq!(game.gems()[0].name, "Gem of Body");
    assert_eq!(game.gems()[1].name, "Gem of Mind");
    let body = game.gem(1).expect("body gem");
    assert_eq!(body.label, "Gem_Body");
    assert_eq!(body.effects, vec![(62, 2), (65, 0)]);
    assert!((body.duration - 3.0).abs() < f32::EPSILON);
    let mind = game.gem(2).expect("mind gem");
    assert_eq!(mind.effects, vec![(63, 1)]);
    assert!((mind.duration - 0.0).abs() < f32::EPSILON);
    assert!(game.gem(0).is_none());
    assert!(game.gem(999).is_none());
}

#[test]
fn styles_resolve_their_level_up_tables() {
    let dir = installation();
    let game = GameData::open(dir.path()).expect("load installation");

    let choices = game.style_choices(0).expect("martial choices");
    assert_eq!(choices.len(), 8);
    assert_eq!(
        choices.iter().map(|choice| choice.slot).collect::<Vec<_>>(),
        [1, 2, 3, 4, 5, 6, 7, 8]
    );
    assert_eq!(
        choices
            .iter()
            .map(|choice| choice.advance)
            .collect::<Vec<_>>(),
        [1, 2, 3, 4, 5, 6, 7, 8]
    );
    assert_eq!(choices[0].family, "Damage");
    assert_eq!(choices[1].family, "Damage");
    assert_eq!(choices[2].family, "Upgrades 2");
    assert_eq!(choices[3].family, "Upgrade One");
    assert_eq!(
        choices.iter().map(|choice| choice.tier).collect::<Vec<_>>(),
        [1, 2, 1, 1, 2, 3, 4, 5]
    );

    let big = game.style_choices(1).expect("big choices");
    assert_eq!(big.len(), 64);
    assert_eq!(big.last().expect("last").slot, 64);
    assert!(game.style_choices(8).is_none());
    assert!(game.style_choices(9).is_none());
    assert!(game.style_choices(10).is_none());
    let partial = game.style_choices(11).expect("partial columns");
    assert_eq!(partial.len(), 1);
    assert_eq!(partial[0].slot, 1);
    assert_eq!(partial[0].advance, 1);
    assert!(game.style_choices(12).is_none());
    assert!(game.style_choices(999).is_none());
}

#[test]
fn search_and_playable_filters_work() {
    let dir = installation();
    let game = GameData::open(dir.path()).expect("load installation");

    assert!(game.search_items("", 10).is_empty());
    assert!(game.search_items("   ", 10).is_empty());
    assert_eq!(game.search_items("iron", 10).len(), 1);
    let swords: Vec<&str> = game
        .search_items("sword", 10)
        .iter()
        .map(|item| item.name.as_str())
        .collect();
    assert_eq!(swords, ["Iron Sword", "Second Sword"]);
    assert_eq!(game.search_items("gem", 1).len(), 1);
    assert!(game.search_items("temp", 10).is_empty());
    assert!(game.search_items("cut", 10).is_empty());
    assert_eq!(game.search_items("body", 10).len(), 1);
    assert_eq!(
        game.playable_items()
            .map(|item| item.id)
            .collect::<Vec<_>>(),
        [0, 1, 2, 8, 9, 10, 11]
    );
}

#[test]
fn keyword_helpers_cover_all_effects() {
    assert_eq!(pretty_effect_name("EFFECT_MODIFYBODYBASE"), "Body");
    assert_eq!(pretty_effect_name("EFFECT_MODIFYMIND"), "Mind");
    assert_eq!(pretty_effect_name("EFFECT_MODIFYSPIRIT"), "Spirit");
    assert_eq!(pretty_effect_name("EFFECT_CHARM"), "Charm");
    assert_eq!(pretty_effect_name("EFFECT_INTUITION"), "Intuition");
    assert_eq!(pretty_effect_name("EFFECT_INTIMIDATE"), "Intimidate");
    assert_eq!(pretty_effect_name("EFFECT_FOCUS"), "Focus");
    assert_eq!(pretty_effect_name("EFFECT_HEALTH"), "Health");
    assert_eq!(pretty_effect_name("EFFECT_CHI"), "Chi");
    assert_eq!(pretty_effect_name("EFFECT_DAMAGEARMOR"), "Damage armor");
    assert_eq!(pretty_effect_name("no_prefix_here"), "No Prefix Here");

    assert_eq!(
        family_and_tier("Increase_Damage_3"),
        ("Increase_Damage".to_owned(), 3)
    );
    assert_eq!(
        family_and_tier("Increase_Damage_x"),
        ("Increase_Damage_x".to_owned(), 0)
    );
    assert_eq!(family_and_tier("NoTier"), ("NoTier".to_owned(), 0));
    assert_eq!(family_and_tier("_3"), ("_3".to_owned(), 0));
}

#[test]
fn an_installation_without_tables_still_opens() {
    let dir = TempDir::new("empty-tables");
    write_tlk(&dir.join("dialog.tlk"), &["only a talk table"]);
    let game = GameData::open(dir.path()).expect("load installation");
    assert_eq!(game.item_count(), 0);
    assert_eq!(game.style_count(), 0);
    assert_eq!(game.playable_item_count(), 0);
    assert!(game.items().next().is_none());
    assert!(game.styles().next().is_none());
    assert!(game.advances().next().is_none());
    assert!(game.gems().is_empty());
    assert!(game.search_items("anything", 10).is_empty());
    assert!(game.item(0).is_none());
    assert!(game.style(0).is_none());
    assert!(game.advance(1).is_none());
    assert!(game.style_choices(0).is_none());
    assert!(game.effect_name(0).is_none());
    assert!(game.gem(0).is_none());

    let empty = TempDir::new("not-a-game");
    assert!(GameData::open(empty.path()).is_err());
    assert!(!is_game_root(empty.path()));
}

#[test]
fn dialogf_tlk_is_used_when_dialog_tlk_is_absent() {
    let dir = TempDir::new("dialogf");
    write_tlk(&dir.join("dialogf.tlk"), &["Female Name"]);
    write_text(
        &dir.join("override").join("items.2da"),
        "2DA V2.0\n\n   LABEL STRREF_NAME\n0 Female_Item 0\n",
    );
    let game = GameData::open(dir.path()).expect("load dialogf");
    assert_eq!(game.item(0).expect("item").name, "Female Name");
}

#[test]
fn override_takes_precedence_and_falls_back_to_the_rim() {
    let dir = rim_installation(false);
    let game = GameData::open(dir.path()).expect("text override");
    assert_eq!(game.item(0).expect("item").label, "Text_Sword");

    let dir = rim_installation(true);
    let game = GameData::open(dir.path()).expect("rim fallback");
    assert_eq!(game.item(0).expect("item").label, "Rim_Sword");
    assert_eq!(game.item_count(), 1);
}

#[test]
fn open_reports_missing_and_corrupt_support_files() {
    let dir = TempDir::new("support-errors");
    let missing_rim = dir.join("missing.rim");
    assert!(RimArchive::open(&missing_rim).is_err());

    let missing_tlk = dir.join("missing.tlk");
    assert!(TalkTable::open(&missing_tlk).is_err());

    let bad_tlk = dir.join("bad.tlk");
    let mut bytes = Vec::new();
    bytes.extend_from_slice(b"XXXX");
    bytes.resize(32, 0);
    write(&bad_tlk, &bytes);
    assert!(TalkTable::open(&bad_tlk).is_err());

    let bad_rim = dir.join("bad.rim");
    write(&bad_rim, b"XXXX");
    assert!(RimArchive::open(&bad_rim).is_err());
}

#[test]
fn game_root_markers_are_recognised() {
    let dir = TempDir::new("markers");
    assert!(!is_game_root(dir.path()));
    write(&dir.join("chitin.key"), b"key");
    assert!(is_game_root(dir.path()));
    std::fs::remove_file(dir.join("chitin.key")).expect("remove key");
    std::fs::create_dir_all(dir.join("override")).expect("create override");
    assert!(is_game_root(dir.path()));
    write_tlk(&dir.join("dialogf.tlk"), &["name"]);
    assert!(is_game_root(dir.path()));
    write_tlk(&dir.join("dialog.tlk"), &["name"]);
    assert!(is_game_root(dir.path()));
}

#[cfg(unix)]
#[test]
fn non_utf8_override_names_are_skipped() {
    use std::ffi::OsString;
    use std::os::unix::ffi::OsStringExt;

    let dir = TempDir::new("non-utf8-override");
    write_tlk(&dir.join("dialog.tlk"), &["Name"]);
    std::fs::create_dir_all(dir.join("override")).expect("create override");
    let name = OsString::from_vec(vec![0xff, b'.', b'2', b'd', b'a']);
    let path = dir.join("override").join(name);
    if let Err(error) = std::fs::write(&path, b"junk") {
        // APFS rejects non-UTF-8 filenames. The loader behaviour under test is
        // only meaningful where the filesystem can represent such a name.
        eprintln!("skipping: the filesystem rejected a non-UTF-8 name: {error}");
        return;
    }
    let game = GameData::open(dir.path()).expect("load installation");
    assert_eq!(game.item_count(), 0);
}

#[test]
fn a_corrupt_dialog_tlk_falls_back_to_dialogf() {
    let dir = TempDir::new("dialogf-fallback");
    std::fs::write(dir.join("dialog.tlk"), b"not a talk table").expect("corrupt tlk");
    write_tlk(&dir.join("dialogf.tlk"), &["Female Name"]);
    write_text(
        &dir.join("override").join("items.2da"),
        "2DA V2.0\n\n   LABEL STRREF_NAME\n0 Female_Item 0\n",
    );
    let game = GameData::open(dir.path()).expect("load dialogf fallback");
    assert_eq!(game.item(0).expect("item").name, "Female Name");
}

#[test]
fn a_found_but_broken_table_is_reported() {
    let dir = TempDir::new("broken-table");
    write_tlk(&dir.join("dialog.tlk"), &["Name"]);
    write_text(&dir.join("override").join("items.2da"), "not a 2DA table");
    let game = GameData::open(dir.path()).expect("the installation still opens");
    assert!(
        game.warnings()
            .iter()
            .any(|warning| warning.contains("items.2da")),
        "{:?}",
        game.warnings()
    );
}
