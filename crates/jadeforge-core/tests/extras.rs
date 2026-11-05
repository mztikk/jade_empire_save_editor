#![allow(clippy::expect_used)] // tests fail loudly; expects are assertions

//! Tests for the newer editing surfaces: styles, henchmen and screenshots.

use std::path::{Path, PathBuf};

use jadeforge_core::{SaveGame, SaveImage, SaveOptions};

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
    let path = std::env::temp_dir().join(format!(
        "jadeforge-extras-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |duration| duration.as_nanos())
    ));
    std::fs::create_dir_all(&path).expect("create temp dir");
    path
}

#[test]
fn styles_keep_their_mask_in_sync_with_the_level() {
    let Some(dir) = save_dir() else {
        eprintln!("skipping: no savegames found");
        return;
    };
    let files = save_files(&dir);
    if files.is_empty() {
        eprintln!("skipping: no savegames");
        return;
    }
    let save = SaveGame::open(&files[0]).expect("open");
    let styles = save.styles().expect("styles");
    if styles.is_empty() {
        eprintln!("skipping: savegame has no styles");
        return;
    }
    for style in styles {
        assert_eq!(
            u32::from(style.level),
            1 + style.mask.count_ones(),
            "style {} has an inconsistent level and mask",
            style.style_id
        );
    }
}

#[test]
fn adding_and_removing_a_style_survives_a_save() {
    let Some(dir) = save_dir() else {
        eprintln!("skipping: no savegames found");
        return;
    };
    let files = save_files(&dir);
    if files.is_empty() {
        eprintln!("skipping: no savegames");
        return;
    }
    let temp = temporary_dir();
    let target = temp.join("save.sac");
    std::fs::copy(&files[0], &target).expect("copy");

    let mut save = SaveGame::open(&target).expect("open");
    let styles = save.styles().expect("styles");
    if styles.is_empty() {
        eprintln!("skipping: savegame has no styles");
        return;
    }
    let before = styles.len();
    let template = styles[0].struct_index;
    let identity = jadeforge_core::StyleIdentity {
        style_id: 7,
        name_ref: 18_444,
        desc_ref: None,
        icon: None,
        weapon: None,
        weapon_count: 0,
    };
    let index = save.add_style(template, &identity).expect("add style");
    assert_eq!(save.styles().expect("styles").len(), before + 1);
    save.set_style_family_slot(index, &[1, 2, 3, 4], 3)
        .expect("upgrade");
    assert_eq!(
        save.styles()
            .expect("styles")
            .into_iter()
            .find(|style| style.struct_index == index)
            .expect("added")
            .level,
        4
    );
    save.save(SaveOptions { backup: false }).expect("save");

    let mut reloaded = SaveGame::open(&target).expect("reopen");
    let styles = reloaded.styles().expect("styles");
    let added = styles
        .iter()
        .find(|style| style.struct_index == index)
        .expect("added style present");
    assert_eq!(added.style_id, 7);
    assert_eq!(added.level, 4);

    assert!(reloaded.remove_style(index).expect("remove"));
    reloaded.save(SaveOptions { backup: false }).expect("save");
    let final_save = SaveGame::open(&target).expect("reopen");
    assert!(
        !final_save
            .styles()
            .expect("styles")
            .iter()
            .any(|style| style.struct_index == index)
    );
}

#[test]
fn henchman_levels_can_be_changed() {
    let Some(dir) = save_dir() else {
        eprintln!("skipping: no savegames found");
        return;
    };
    let files = save_files(&dir);
    if files.is_empty() {
        eprintln!("skipping: no savegames");
        return;
    }
    let mut save = SaveGame::open(&files[0]).expect("open");
    let henchmen = save.henchmen().expect("henchmen");
    if henchmen.is_empty() {
        eprintln!("skipping: savegame has no henchmen");
        return;
    }
    let target = &henchmen[0];
    save.set_henchman_level(target.struct_index, 12)
        .expect("level");
    save.set_henchman_available(target.struct_index, true)
        .expect("available");
    let updated = save
        .henchmen()
        .expect("henchmen")
        .into_iter()
        .find(|henchman| henchman.struct_index == target.struct_index)
        .expect("henchman");
    assert_eq!(updated.level, 12);
    assert!(updated.available);
    assert!(updated.available2);
    assert!(save.set_henchman_level(target.struct_index, 0).is_err());
}

#[test]
fn screenshots_are_decoded() {
    let Some(dir) = save_dir() else {
        eprintln!("skipping: no savegames found");
        return;
    };
    let files = save_files(&dir);
    let mut decoded = 0_usize;
    for file in files.iter().take(5) {
        match SaveImage::load(file) {
            Ok(image) => {
                assert!(image.width > 0 && image.height > 0);
                assert_eq!(
                    image.rgba.len(),
                    image.width as usize * image.height as usize * 4
                );
                decoded += 1;
            }
            Err(error) => eprintln!("cannot load screenshot of {}: {error}", file.display()),
        }
    }
    assert!(decoded > 0, "no screenshot could be decoded");
}
