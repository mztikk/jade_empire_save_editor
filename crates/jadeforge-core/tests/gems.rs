#![allow(clippy::expect_used)] // tests fail loudly; expects are assertions

//! Gem reading, moving and removal against real savegames.

use std::path::PathBuf;

use jadeforge_core::{SaveGame, SaveOptions};

fn save_dir() -> Option<PathBuf> {
    let dir = std::env::var_os("JADEFORGE_SAVE_DIR").map(PathBuf::from)?;
    dir.is_dir().then_some(dir)
}

fn first_save(dir: &std::path::Path) -> Option<PathBuf> {
    let mut files: Vec<PathBuf> = std::fs::read_dir(dir)
        .into_iter()
        .flatten()
        .flatten()
        .map(|entry| entry.path().join("save.sac"))
        .filter(|path| path.is_file())
        .collect();
    files.sort();
    files.into_iter().next()
}

#[test]
fn gems_move_between_amulet_and_inventory() {
    let Some(dir) = save_dir() else {
        eprintln!("skipping: no savegames found");
        return;
    };
    let Some(path) = first_save(&dir) else {
        eprintln!("skipping: no savegames");
        return;
    };
    let temp = std::env::temp_dir().join(format!("jadeforge-gems-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&temp);
    std::fs::create_dir_all(&temp).expect("temp dir");
    let target = temp.join("save.sac");
    std::fs::copy(&path, &target).expect("copy");

    let mut save = SaveGame::open(&target).expect("open");
    let gems = save.gems().expect("gems");
    if gems.is_empty() {
        eprintln!("skipping: savegame has no gems");
        let _ = std::fs::remove_dir_all(temp);
        return;
    }
    let gem = &gems[0];
    let before = gem.location;
    let moved = save.move_gem(gem.struct_index).expect("move");
    assert_ne!(moved, before);
    let after = save
        .gems()
        .expect("gems")
        .into_iter()
        .find(|candidate| candidate.struct_index == gem.struct_index)
        .expect("gem");
    assert_eq!(after.location, moved);
    assert_eq!(after.item_id, gem.item_id);

    save.remove_gem(gem.struct_index).expect("remove");
    assert!(
        save.gems()
            .expect("gems")
            .iter()
            .all(|candidate| candidate.struct_index != gem.struct_index)
    );

    // Add a predefined gem with a synthetic effect list.
    let added = save
        .add_gem(434, &[(62, 2), (65, 1)], 0.0)
        .expect("add gem");
    let added_gem = save
        .gems()
        .expect("gems")
        .into_iter()
        .find(|candidate| candidate.struct_index == added)
        .expect("added gem");
    assert_eq!(added_gem.item_id, 434);
    assert_eq!(added_gem.location, jadeforge_core::GemLocation::Inventory);
    assert_eq!(added_gem.effects.len(), 2);
    assert_eq!(added_gem.effects[0].kind, 62);
    assert_eq!(added_gem.effects[0].value, 2);

    // The modified savegame still has to serialize and verify.
    save.save(SaveOptions::default()).expect("save");
    let _ = std::fs::remove_dir_all(temp);
}

#[test]
fn amulet_sockets_cover_every_socketed_gem() {
    let Some(dir) = save_dir() else {
        eprintln!("skipping: no savegames found");
        return;
    };
    let mut checked = 0;
    for entry in std::fs::read_dir(&dir).into_iter().flatten().flatten() {
        let path = entry.path().join("save.sac");
        if !path.is_file() {
            continue;
        }
        let save = SaveGame::open(&path).expect("open");
        let amulet = save.amulet().expect("amulet");
        let Some(amulet) = amulet else {
            continue;
        };
        assert!(
            matches!(amulet.sockets, 3 | 5 | 7),
            "{}: unexpected socket count {}",
            path.display(),
            amulet.sockets
        );
        let gems = save.gems().expect("gems");
        let socketed = gems
            .iter()
            .filter(|gem| {
                gem.location == jadeforge_core::GemLocation::Amulet
                    && (gem.item_id != 0 || !gem.effects.is_empty())
            })
            .count();
        assert!(
            socketed <= usize::from(amulet.sockets),
            "{}: {socketed} gems in {} sockets",
            path.display(),
            amulet.sockets
        );
        checked += 1;
    }
    assert!(checked > 0, "no savegames were checked");
}
