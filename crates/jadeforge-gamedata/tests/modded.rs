//! A modded installation: styles and their level tables shipped in `override/`.
//!
//! This is the supported way to add styles: the mod extends
//! `stylesuperlist.2da`, adds a row to `styledata.2da` and ships the new
//! `<Style>LV.2da` tables. The editor must resolve the upgrades exactly like it
//! does for the shipped tables.

use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use jadeforge_gamedata::GameData;

fn write_tlk(path: &Path, strings: &[&str]) {
    let mut data = Vec::new();
    data.extend_from_slice(b"TLK V4.0");
    data.extend_from_slice(&2_u32.to_le_bytes());
    data.extend_from_slice(&(strings.len() as u32).to_le_bytes());
    data.extend_from_slice(&24_u32.to_le_bytes());
    let data_offset = 24 + 10 * strings.len();
    data.extend_from_slice(&(data_offset as u32).to_le_bytes());
    let mut text = Vec::new();
    for string in strings {
        let bytes = string.as_bytes();
        data.extend_from_slice(&0_u32.to_le_bytes());
        // Entries store absolute file offsets.
        data.extend_from_slice(&((data_offset + text.len()) as u32).to_le_bytes());
        data.extend_from_slice(&(bytes.len() as u16).to_le_bytes());
        text.extend_from_slice(bytes);
    }
    data.extend_from_slice(&text);
    std::fs::write(path, data).expect("write talk table");
}

fn write(path: &Path, text: &str) {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).expect("create folder");
    }
    std::fs::write(path, text).expect("write table");
}

fn fixture() -> PathBuf {
    let root = std::env::temp_dir().join(format!(
        "jadeforge-modded-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |duration| duration.as_nanos())
    ));
    let override_dir = root.join("override");
    std::fs::create_dir_all(&override_dir).expect("create override");
    write_tlk(
        &root.join("dialog.tlk"),
        &["Mod Martial Style", "Mod Weapon Style", "Bought an upgrade"],
    );
    write(
        &override_dir.join("stylesuperlist.2da"),
        "2DA V2.0\n\n   Label       NameRef DescRef IconRef StyleData WeaponCount Weapon Type\n\
         0  Mod_Martial  0       2       ****    0         0           **** 0\n\
         1  Mod_Weapon   1       2       ****    1         1           mod_w 1\n",
    );
    write(
        &override_dir.join("styledata.2da"),
        "2DA V2.0\n\n   label      useexcel levelup2da   move2da advancement2da\n\
         0  modmartial 0        modmartiallv ****    ****\n\
         1  modweapon  0        modweaponlv  ****    ****\n",
    );
    // Upper case file name on purpose: the override lookup is case insensitive.
    write(
        &override_dir.join("ModMartialLV.2DA"),
        "2DA V2.0\n\n   Choices Label Choice_1 Choice_2 Choice_3 Choice_4 Choice_5\n\
         0  2       0     1        2        ****     ****     ****\n",
    );
    write(
        &override_dir.join("modweaponlv.2da"),
        "2DA V2.0\n\n   Choices Label Choice_1 Choice_2 Choice_3 Choice_4 Choice_5\n\
         0  1       0     10       ****     ****     ****     ****\n",
    );
    let mut advances =
        String::from("2DA V2.0\n\n   Label            DescRef CostToLevelUp Effect1 Param1\n");
    for row in 1..=12 {
        let _ = writeln!(
            advances,
            "{row}  Mod_Upgrade_{row} 2       {row}           72      0.25"
        );
    }
    write(&override_dir.join("styleadvance.2da"), &advances);
    root
}

#[test]
fn modded_styles_resolve_their_own_upgrade_tables() {
    let root = fixture();
    let game = GameData::open(&root).expect("load modded installation");
    assert_eq!(game.style_count(), 2);
    assert_eq!(
        game.style(0).map(|style| style.name.as_str()),
        Some("Mod Martial Style")
    );
    assert_eq!(game.styles().count(), 2);
    let martial = game.style_choices(0).expect("martial choices");
    assert_eq!(martial.len(), 2);
    assert_eq!((martial[0].slot, martial[0].advance), (1, 1));
    assert_eq!((martial[1].slot, martial[1].advance), (2, 2));
    let weapon = game.style_choices(1).expect("weapon choices");
    assert_eq!(weapon.len(), 1);
    assert_eq!((weapon[0].slot, weapon[0].advance), (1, 10));
    let damage = game.advance(1).expect("upgrade row 1");
    assert_eq!(damage.label, "Mod_Upgrade_1");
    assert_eq!(damage.cost, 1);
    assert_eq!(damage.name, "Bought an upgrade");
    assert!(game.advance(12).is_some());
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn a_style_without_styledata_falls_back_to_no_restriction() {
    let root = fixture();
    std::fs::remove_file(root.join("override").join("styledata.2da")).expect("remove");
    let game = GameData::open(&root).expect("load");
    assert!(game.styles().count() >= 2);
    assert!(game.style_choices(0).is_none());
    let _ = std::fs::remove_dir_all(root);
}
