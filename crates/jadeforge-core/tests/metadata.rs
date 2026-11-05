#![allow(clippy::expect_used)] // tests fail loudly; expects are assertions

//! The metadata fast path must agree with a full character parse.

use std::path::{Path, PathBuf};

use jadeforge_core::SaveGame;

fn save_dir() -> Option<PathBuf> {
    std::env::var_os("JADEFORGE_SAVE_DIR")
        .map(PathBuf::from)
        .filter(|path| path.is_dir())
}

fn savegames(dir: &Path) -> Vec<PathBuf> {
    let mut saves: Vec<PathBuf> = std::fs::read_dir(dir)
        .into_iter()
        .flatten()
        .flatten()
        .map(|entry| entry.path().join("save.sac"))
        .filter(|path| path.is_file())
        .collect();
    saves.sort();
    saves
}

#[test]
fn metadata_matches_the_full_character_parse() {
    let Some(dir) = save_dir() else {
        eprintln!("skipping: set JADEFORGE_SAVE_DIR");
        return;
    };
    let saves = savegames(&dir);
    if saves.is_empty() {
        eprintln!("skipping: no savegames in {}", dir.display());
        return;
    }
    for path in saves {
        let metadata = SaveGame::metadata(&path).expect("metadata");
        let save = SaveGame::open(&path).expect("open");
        let character = save.character().expect("character");
        assert_eq!(metadata.name, character.name, "{} name", path.display());
        assert_eq!(metadata.level, character.level, "{} level", path.display());
        assert_eq!(
            metadata.time_played,
            character.time_played,
            "{} time played",
            path.display()
        );
    }
}
