#![allow(clippy::expect_used)] // tests fail loudly; expects are assertions

//! Backup listing and restore, using savegames built from scratch.

use std::path::{Path, PathBuf};

use jadeforge_core::gff::Value;
use jadeforge_core::{Gff, SaveGame, SaveOptions, list_backups, restore_backup};

fn sample_block(tag: &str) -> Gff {
    let mut gff = Gff {
        version: *b"V3.2",
        labels: vec![],
        structs: vec![],
        root: 0,
    };
    let root = gff.push_struct(u32::MAX);
    gff.add_field(root, "Tag", Value::String(tag.to_owned()))
        .expect("tag");
    gff
}

fn container(block: &Gff) -> Vec<u8> {
    let payload = block.to_bytes().expect("serialize block");
    let name = b"save.sav\0";
    let mut out = Vec::new();
    out.extend_from_slice(&1_u32.to_le_bytes());
    out.extend_from_slice(
        &u32::try_from(name.len())
            .expect("name length")
            .to_le_bytes(),
    );
    out.extend_from_slice(name);
    out.extend_from_slice(
        &u32::try_from(payload.len())
            .expect("payload length")
            .to_le_bytes(),
    );
    out.extend_from_slice(&payload);
    out
}

struct TempDir(PathBuf);

impl TempDir {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "jadeforge-backup-test-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_or(0, |duration| duration.as_nanos())
        ));
        std::fs::create_dir_all(&path).expect("create temp dir");
        Self(path)
    }

    fn save(&self, tag: &str) -> PathBuf {
        let path = self.0.join("save.sac");
        std::fs::write(&path, container(&sample_block(tag))).expect("write savegame");
        path
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn backups_of(path: &Path) -> Vec<jadeforge_core::BackupEntry> {
    list_backups(path).expect("list backups")
}

#[test]
fn a_save_creates_a_listable_backup_with_its_original_path() {
    let dir = TempDir::new();
    let path = dir.save("first");
    let mut save = SaveGame::open(&path).expect("open");
    save.save(SaveOptions { backup: true }).expect("save");

    let backups = backups_of(&path);
    assert_eq!(backups.len(), 1, "one save must create one backup");
    assert_eq!(backups[0].original, path);
    assert!(backups[0].path.exists());
    assert!(backups[0].size > 0);
}

#[test]
fn backups_are_listed_newest_first_and_other_savegames_are_ignored() {
    let dir = TempDir::new();
    let path = dir.save("first");
    for _ in 0..2 {
        let mut save = SaveGame::open(&path).expect("open");
        save.save(SaveOptions { backup: true }).expect("save");
        std::thread::sleep(std::time::Duration::from_millis(2));
    }
    std::fs::write(
        dir.0
            .join("other.sac.jadeforge-backup-1700000000.000000001.1"),
        b"not ours",
    )
    .expect("write other backup");

    let backups = backups_of(&path);
    assert_eq!(backups.len(), 2);
    assert!(backups[0].created >= backups[1].created);
}

#[test]
fn restoring_a_backup_writes_it_atomically_and_keeps_a_safety_copy() {
    let dir = TempDir::new();
    let path = dir.save("original");
    let original_bytes = std::fs::read(&path).expect("read original");
    let mut save = SaveGame::open(&path).expect("open");
    save.save(SaveOptions { backup: true }).expect("save");
    let backup = backups_of(&path).remove(0).path;

    // Change the file so the restore has something visible to undo.
    std::fs::write(&path, container(&sample_block("changed"))).expect("change");
    let safety = restore_backup(&backup, &path).expect("restore");
    assert_eq!(std::fs::read(&path).expect("read restored"), original_bytes);
    assert!(safety.exists(), "the current file must be backed up first");
    assert_eq!(backups_of(&path).len(), 2);
}

#[test]
fn restoring_a_broken_backup_never_touches_the_savegame() {
    let dir = TempDir::new();
    let path = dir.save("current");
    let before = std::fs::read(&path).expect("read");
    let broken = dir.0.join("save.sac.jadeforge-backup-1.000000001.1");
    std::fs::write(&broken, b"not a savegame").expect("write broken backup");
    assert!(restore_backup(&broken, &path).is_err());
    assert_eq!(std::fs::read(&path).expect("read"), before);
}
