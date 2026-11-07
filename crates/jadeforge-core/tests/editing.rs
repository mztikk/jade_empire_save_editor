#![allow(clippy::expect_used)] // tests fail loudly; expects are assertions

//! Editing tests against real savegames: values are changed, the file is
//! written to a temporary directory and read back.

use std::path::{Path, PathBuf};

use jadeforge_core::{MAX_CHAPTER, SaveGame, SaveOptions, StyleIdentity};

fn save_dir() -> Option<PathBuf> {
    let dir = std::env::var_os("JADEFORGE_SAVE_DIR").map(PathBuf::from)?;
    dir.is_dir().then_some(dir)
}

fn save_files(dir: &Path) -> Vec<PathBuf> {
    let mut files: Vec<PathBuf> = std::fs::read_dir(dir)
        .into_iter()
        .flatten()
        .flatten()
        .map(|entry| entry.path().join("save.sac"))
        .filter(|candidate| candidate.is_file())
        .collect();
    files.sort();
    files
}

fn temporary_dir() -> PathBuf {
    let mut path = std::env::temp_dir();
    path.push(format!(
        "jadeforge-test-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |duration| duration.as_nanos())
    ));
    std::fs::create_dir_all(&path).expect("create temp dir");
    path
}

#[test]
fn editing_the_character_survives_a_save() {
    let Some(dir) = save_dir() else {
        eprintln!("skipping: no savegames found (set JADEFORGE_SAVE_DIR)");
        return;
    };
    let files = save_files(&dir);
    if files.is_empty() {
        eprintln!("skipping: no savegames");
        return;
    }
    let source = files
        .iter()
        .max_by_key(|path| std::fs::metadata(path).map_or(0, |meta| meta.len()))
        .expect("at least one save");
    let temp = temporary_dir();
    let target = temp.join("save.sac");
    std::fs::copy(source, &target).expect("copy savegame");

    let mut save = SaveGame::open(&target).expect("open copy");
    let original = save.character().expect("read character");
    let mut expected = original.clone();
    expected.name = format!("{} II", original.name);
    expected.level = original.level.saturating_add(1);
    expected.experience = original.experience.saturating_add(1234);
    expected.gold = Some(original.gold.unwrap_or(0).saturating_add(4242));
    expected.health_max = original.health_max + 10;
    expected.health = expected.health_max as f32;
    expected.body = original.body.saturating_add(1);
    expected.attribute_points = original.attribute_points + 3;
    expected.chapter = original.chapter;

    let changes = save.apply_character(&expected).expect("apply character");
    assert!(!changes.is_empty(), "nothing changed");
    assert!(save.is_modified());
    save.save(SaveOptions { backup: false }).expect("save");

    let reloaded = SaveGame::open(&target).expect("reopen");
    let actual = reloaded.character().expect("read back");
    assert_eq!(actual, expected, "character did not survive the round trip");

    // The file must still be a valid savegame with the same block layout.
    assert_eq!(reloaded.blocks().len(), save.blocks().len());
    reloaded.verify().expect("verify");
}

#[test]
fn editing_chapter_adds_the_missing_field_on_a_real_save() {
    let Some(dir) = save_dir() else {
        eprintln!("skipping: no savegames found (set JADEFORGE_SAVE_DIR)");
        return;
    };
    for source in save_files(&dir) {
        let Ok(save) = SaveGame::open(&source) else {
            continue;
        };
        let Some(block) = save.block("save.sav") else {
            continue;
        };
        let gff = block.gff();
        if gff.get("Campaign").is_err() || gff.get("Campaign.Chapter").is_ok() {
            continue;
        }

        let temp = temporary_dir();
        let target = temp.join("save.sac");
        std::fs::copy(&source, &target).expect("copy savegame");
        let mut save = SaveGame::open(&target).expect("open copy");
        let mut character = save.character().expect("character");
        character.chapter = if character.chapter < MAX_CHAPTER {
            character.chapter + 1
        } else {
            0
        };
        save.apply_character(&character).expect("apply chapter");
        save.save(SaveOptions { backup: false }).expect("save");
        let reloaded = SaveGame::open(&target).expect("reopen");
        assert_eq!(
            reloaded.character().expect("character").chapter,
            character.chapter
        );
        return;
    }
    eprintln!("skipping: no real savegame lacks Campaign.Chapter");
}

#[test]
fn learning_a_style_from_a_sparse_real_template_keeps_its_identity() {
    let Some(dir) = save_dir() else {
        eprintln!("skipping: no savegames found (set JADEFORGE_SAVE_DIR)");
        return;
    };
    for source in save_files(&dir) {
        let Ok(mut save) = SaveGame::open(&source) else {
            continue;
        };
        let Ok(location) = save.player_location() else {
            continue;
        };
        let Some(template) = ({
            let Some(block) = save.block(location.block) else {
                continue;
            };
            let gff = block.gff();
            let Ok(styles) = save.styles() else {
                continue;
            };
            styles
                .iter()
                .find(|style| {
                    gff.get(format!(
                        "{}Stats.Styles[{}].WeaponRef",
                        location.prefix, style.struct_index
                    ))
                    .is_err()
                })
                .map(|style| style.struct_index)
        }) else {
            continue;
        };
        let identity = StyleIdentity {
            style_id: 2,
            name_ref: 42,
            desc_ref: Some(99),
            icon: Some("I_REAL".to_owned()),
            weapon: Some("J00_REAL".to_owned()),
            weapon_count: 2,
        };
        let added = save.add_style(template, &identity).expect("add style");
        let added_style = save
            .styles()
            .expect("styles")
            .into_iter()
            .find(|style| style.struct_index == added)
            .expect("added style");
        assert_eq!(added_style.weapon.as_deref(), Some("J00_REAL"));
        assert_eq!(added_style.weapon_count, 2);
        let bytes = save.to_bytes().expect("serialize");
        SaveGame::parse("real-style.sac".into(), bytes).expect("reparse");
        return;
    }
    eprintln!("skipping: no real style lacks WeaponRef");
}

#[test]
fn rejected_edits_leave_the_savegame_untouched() {
    let Some(dir) = save_dir() else {
        eprintln!("skipping: no savegames found (set JADEFORGE_SAVE_DIR)");
        return;
    };
    let files = save_files(&dir);
    if files.is_empty() {
        eprintln!("skipping: no savegames");
        return;
    }
    let mut save = SaveGame::open(&files[0]).expect("open");
    let before = save.to_bytes().expect("serialize");
    let mut character = save.character().expect("character");
    character.level = 0; // out of range
    let error = save
        .apply_character(&character)
        .expect_err("level 0 must be rejected");
    assert!(error.to_string().contains("level"), "{error}");
    assert!(
        !save.is_modified(),
        "a rejected edit must not dirty the save"
    );

    // A rejected edit must not change anything else either.
    let mut character = save.character().expect("character");
    character.name = "x".repeat(65);
    assert!(save.apply_character(&character).is_err());
    assert!(!save.is_modified());
    assert_eq!(save.to_bytes().expect("serialize"), before);
}

#[test]
fn items_can_be_added_and_removed() {
    let Some(dir) = save_dir() else {
        eprintln!("skipping: no savegames found (set JADEFORGE_SAVE_DIR)");
        return;
    };
    let files = save_files(&dir);
    if files.is_empty() {
        eprintln!("skipping: no savegames");
        return;
    }
    let source = &files[0];
    let temp = temporary_dir();
    let target = temp.join("save.sac");
    std::fs::copy(source, &target).expect("copy savegame");

    let mut save = SaveGame::open(&target).expect("open copy");
    let before = save.items().expect("items");
    let new_struct = save.add_item(1234, 6, 3).expect("add item");
    let after = save.items().expect("items");
    assert_eq!(after.len(), before.len() + 1);
    let added = after
        .iter()
        .find(|item| item.struct_index == new_struct)
        .expect("added item present");
    assert_eq!(added.item_id, 1234);
    assert_eq!(added.count, 3);
    assert_eq!(added.kind, 6);

    save.set_item_count(new_struct, 9).expect("set count");
    assert_eq!(
        save.items()
            .expect("items")
            .iter()
            .find(|item| item.struct_index == new_struct)
            .expect("item")
            .count,
        9
    );

    save.save(SaveOptions { backup: false }).expect("save");
    let mut reloaded = SaveGame::open(&target).expect("reopen");
    let items = reloaded.items().expect("items");
    let added = items
        .iter()
        .find(|item| item.item_id == 1234)
        .expect("added item survived");
    assert_eq!(added.count, 9);

    assert!(reloaded.remove_item(added.struct_index).expect("remove"));
    reloaded.save(SaveOptions { backup: false }).expect("save");
    let final_save = SaveGame::open(&target).expect("reopen");
    assert!(
        !final_save
            .items()
            .expect("items")
            .iter()
            .any(|item| item.item_id == 1234)
    );
}
