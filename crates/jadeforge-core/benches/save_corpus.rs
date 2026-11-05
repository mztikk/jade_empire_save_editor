#![allow(clippy::expect_used)] // a broken benchmark fixture should fail loudly

//! A dependency-free benchmark for the hot savegame operations.
//!
//! With `JADEFORGE_SAVE_DIR` set, this opens and round-trips the smallest save
//! in the corpus; without it, a synthetic character save is used. It is a
//! `harness = false` benchmark so it does not need Criterion:
//!
//! ```sh
//! cargo bench -p jadeforge-core --bench save_corpus
//! ```

use std::path::PathBuf;
use std::time::Instant;

use jadeforge_core::gff::{Gff, Value};
use jadeforge_core::{SaveGame, SaveOptions};

fn synthetic() -> Gff {
    let mut gff = Gff {
        version: *b"V3.2",
        labels: Vec::new(),
        structs: Vec::new(),
        root: 0,
    };
    let root = gff.push_struct(u32::MAX);
    let player = gff.push_struct(0);
    let stats = gff.push_struct(0);
    let campaign = gff.push_struct(0);
    gff.add_field(root, "PlayerCreature", Value::Struct(player))
        .expect("player");
    gff.add_field(root, "Campaign", Value::Struct(campaign))
        .expect("campaign");
    gff.add_field(player, "FirstName", Value::String("Benchmark".to_owned()))
        .expect("name");
    gff.add_field(player, "Level", Value::Byte(7))
        .expect("level");
    gff.add_field(player, "CurrentXP", Value::DWord(1234))
        .expect("xp");
    gff.add_field(player, "Stats", Value::Struct(stats))
        .expect("stats");
    gff.add_field(stats, "Gold", Value::DWord(99))
        .expect("gold");
    gff.add_field(stats, "Health", Value::Float(120.0))
        .expect("health");
    gff.add_field(campaign, "Chapter", Value::DWord(2))
        .expect("chapter");
    gff.add_field(campaign, "TimePlayed", Value::DWord64(90_000_000_000))
        .expect("time");
    gff
}

fn corpus() -> Option<PathBuf> {
    let dir = std::env::var_os("JADEFORGE_SAVE_DIR").map(PathBuf::from)?;
    let mut saves: Vec<(u64, PathBuf)> = std::fs::read_dir(dir)
        .ok()?
        .flatten()
        .map(|entry| entry.path().join("save.sac"))
        .filter(|path| path.is_file())
        .filter_map(|path| Some((std::fs::metadata(&path).ok()?.len(), path)))
        .collect();
    saves.sort();
    saves.into_iter().next().map(|(_, path)| path)
}

fn measure<F: FnMut()>(name: &str, mut operation: F) {
    // Warm up, then time ten iterations.
    for _ in 0..3 {
        operation();
    }
    let start = Instant::now();
    for _ in 0..10 {
        operation();
    }
    let elapsed = start.elapsed() / 10;
    println!("{name:<24} {elapsed:?}");
}

fn main() {
    let mut save = if let Some(path) = corpus() {
        SaveGame::open(&path).expect("open corpus save")
    } else {
        let block = synthetic();
        let bytes = block.to_bytes().expect("serialize synthetic block");
        let mut container = Vec::new();
        container.extend_from_slice(&1_u32.to_le_bytes());
        container.extend_from_slice(&9_u32.to_le_bytes());
        container.extend_from_slice(b"save.sav\0");
        container.extend_from_slice(
            &u32::try_from(bytes.len())
                .expect("block size")
                .to_le_bytes(),
        );
        container.extend_from_slice(&bytes);
        SaveGame::parse("synthetic.sac".into(), container).expect("parse synthetic save")
    };

    measure("open (in memory parse)", || {
        let _ = SaveGame::parse(save.path().to_path_buf(), save.to_bytes().expect("bytes"));
    });
    measure("to_bytes", || {
        let _ = save.to_bytes().expect("serialize");
    });
    measure("verify", || {
        save.verify().expect("verify");
    });
    let mut character = save.character().expect("character");
    character.level = character.level.saturating_add(1);
    measure("apply_character", || {
        save.apply_character(&character).expect("apply");
    });
    let _ = save.save_to("target/bench-save.sac", SaveOptions { backup: false });
}
