#![allow(clippy::expect_used)] // tests fail loudly; expects are assertions

//! Rewrite tests: every block of every available savegame is serialized from
//! the parsed tree again and must survive a full round trip.

use std::path::{Path, PathBuf};

use jadeforge_core::SaveGame;

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

#[test]
fn rewriting_all_blocks_keeps_the_tree() {
    let Some(dir) = save_dir() else {
        eprintln!("skipping: no savegames found (set JADEFORGE_SAVE_DIR)");
        return;
    };
    let files = save_files(&dir);
    if files.is_empty() {
        eprintln!("skipping: no save.sac files in {}", dir.display());
        return;
    }

    let mut blocks = 0_usize;
    for path in &files {
        let mut save = SaveGame::open(path).expect("open savegame");
        let expected: Vec<_> = save
            .blocks()
            .iter()
            .map(|block| block.gff().clone())
            .collect();
        for block in save.blocks_mut() {
            block.mark_modified();
        }
        let bytes = save.to_bytes().expect("serialize");
        let reparsed = SaveGame::parse(path.clone(), bytes).expect("re-parse");
        assert_eq!(reparsed.blocks().len(), expected.len());
        for (index, (block, expected)) in reparsed.blocks().iter().zip(&expected).enumerate() {
            assert_eq!(block.name, save.blocks()[index].name);
            assert_eq!(
                block.gff().structs.len(),
                expected.structs.len(),
                "{} block {} struct count",
                path.display(),
                block.name
            );
            assert_eq!(
                block.gff().labels,
                expected.labels,
                "{} block {} labels",
                path.display(),
                block.name
            );
            for (struct_index, (actual, expected)) in block
                .gff()
                .structs
                .iter()
                .zip(&expected.structs)
                .enumerate()
            {
                assert_eq!(
                    actual,
                    expected,
                    "{} block {} struct {}",
                    path.display(),
                    block.name,
                    struct_index
                );
            }
            blocks += 1;
        }
        save.verify().expect("verify");
    }
    eprintln!("rewrote {blocks} blocks");
}

#[test]
fn rewritten_blocks_are_compact() {
    let Some(dir) = save_dir() else {
        eprintln!("skipping: no savegames found (set JADEFORGE_SAVE_DIR)");
        return;
    };
    // A canonical rewrite may differ in size (identical payloads are stored
    // once, table order is normalised), but it must never explode.
    for path in save_files(&dir).into_iter().take(10) {
        let mut save = SaveGame::open(&path).expect("open savegame");
        let before: usize = save
            .blocks()
            .iter()
            .map(|block| block.gff().structs.len())
            .sum();
        for block in save.blocks_mut() {
            block.mark_modified();
        }
        let written = save.to_bytes().expect("serialize");
        let original = std::fs::metadata(&path).expect("metadata").len() as usize;
        assert!(before > 0);
        assert!(
            written.len() < original + 1024,
            "{} grew from {original} to {} bytes",
            path.display(),
            written.len()
        );
    }
}
