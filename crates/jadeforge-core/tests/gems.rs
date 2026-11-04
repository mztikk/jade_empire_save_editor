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

    // The modified savegame still has to serialize and verify.
    save.save(SaveOptions::default()).expect("save");
    let _ = std::fs::remove_dir_all(temp);
}
