#![allow(clippy::expect_used)] // tests fail loudly; expects are assertions

//! Round trip tests against real savegames.
//!
//! The tests look for a Jade Empire save folder in `JADEFORGE_SAVE_DIR`. When
//! it is unset or empty the tests report that they were skipped instead of
//! failing, which keeps the suite runnable on machines without the game.

use std::path::{Path, PathBuf};

use jadeforge_core::SaveGame;

fn save_dir() -> Option<PathBuf> {
    let dir = std::env::var_os("JADEFORGE_SAVE_DIR").map(PathBuf::from)?;
    dir.is_dir().then_some(dir)
}

fn save_files(dir: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    let Ok(entries) = std::fs::read_dir(dir) else {
        return files;
    };
    for entry in entries.flatten() {
        let candidate = entry.path().join("save.sac");
        if candidate.is_file() {
            files.push(candidate);
        }
    }
    files.sort();
    files
}

#[test]
fn every_available_savegame_round_trips_byte_for_byte() {
    let Some(dir) = save_dir() else {
        eprintln!("skipping: no savegames found (set JADEFORGE_SAVE_DIR)");
        return;
    };
    let files = save_files(&dir);
    if files.is_empty() {
        eprintln!("skipping: no save.sac files in {}", dir.display());
        return;
    }

    let mut checked = 0;
    for path in &files {
        let original = std::fs::read(path).expect("read savegame");
        let save = SaveGame::parse(path.clone(), original.clone())
            .unwrap_or_else(|error| panic!("failed to parse {}: {error}", path.display()));
        let written = save
            .to_bytes()
            .unwrap_or_else(|error| panic!("failed to write {}: {error}", path.display()));
        assert_eq!(
            written,
            original,
            "saving {} without changes must reproduce the original bytes",
            path.display()
        );
        save.verify()
            .unwrap_or_else(|error| panic!("verification of {} failed: {error}", path.display()));

        // The block chain must cover the file exactly.
        assert!(!save.blocks().is_empty());
        assert!(
            save.blocks().iter().any(|block| block.name == "save.sav"),
            "{} has no save.sav block",
            path.display()
        );
        checked += 1;
    }
    eprintln!("round tripped {checked} savegames");
}

#[test]
fn player_creature_is_present_in_every_savegame() {
    let Some(dir) = save_dir() else {
        eprintln!("skipping: no savegames found (set JADEFORGE_SAVE_DIR)");
        return;
    };
    for path in save_files(&dir) {
        let save = SaveGame::open(&path).expect("open savegame");
        let block = save.global_block().expect("save.sav block");
        let creature = block
            .gff()
            .get("PlayerCreature")
            .expect("player creature field")
            .value
            .as_struct()
            .expect("player creature struct");
        let structure = block.gff().struct_at(creature).expect("struct");
        assert!(
            structure
                .fields
                .iter()
                .any(|field| block.gff().label_name(field.label) == Some("Stats"))
        );
    }
}
