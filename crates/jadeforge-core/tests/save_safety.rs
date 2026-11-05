#![allow(clippy::expect_used)] // tests fail loudly; expects are assertions
#![allow(clippy::panic)] // tests use panics when a save path is wrong

//! Save-path safety: unique backups, permissions, symlinks and conflict
//! detection. Everything runs on synthetic savegames in temporary folders.

mod common;

use std::fs;
use std::path::Path;

use jadeforge_core::{Error, SaveGame, SaveOptions};

fn save_with_level(path: &Path, level: u8) {
    let mut save = SaveGame::open(path).expect("open savegame");
    let mut character = save.character().expect("character");
    character.level = level;
    save.apply_character(&character).expect("apply level");
    save.save(SaveOptions { backup: true }).expect("save");
}

fn backups(folder: &Path) -> Vec<std::path::PathBuf> {
    let mut paths: Vec<_> = fs::read_dir(folder)
        .expect("read folder")
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.contains(".jadeforge-backup-"))
        })
        .collect();
    paths.sort();
    paths
}

#[test]
fn two_saves_in_the_same_second_keep_two_backups() {
    let temp = common::TempDir::new("backups");
    let path = common::write_save(temp.path(), &common::character_gff());
    let original = fs::read(&path).expect("read original");

    save_with_level(&path, 8);
    save_with_level(&path, 9);

    let found = backups(temp.path());
    assert_eq!(found.len(), 2, "each save must keep its own backup");
    let contents: Vec<Vec<u8>> = found
        .iter()
        .map(|backup| fs::read(backup).expect("read backup"))
        .collect();
    assert!(
        contents.iter().any(|content| content == &original),
        "the pristine pre-edit savegame must survive in a backup"
    );
    let mut reloaded = SaveGame::open(&path).expect("reopen");
    assert_eq!(
        reloaded.character().expect("character").level,
        9,
        "the live savegame has the final level"
    );
    let mut character = reloaded.character().expect("character");
    character.level = 10;
    reloaded.apply_character(&character).expect("apply");
    reloaded
        .save(SaveOptions { backup: true })
        .expect("save again");
    assert_eq!(backups(temp.path()).len(), 3);
}

#[cfg(unix)]
#[test]
fn saving_preserves_the_file_mode() {
    use std::os::unix::fs::PermissionsExt as _;

    let temp = common::TempDir::new("permissions");
    let path = common::write_save(temp.path(), &common::character_gff());
    fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).expect("chmod");
    save_with_level(&path, 8);
    let mode = fs::metadata(&path).expect("metadata").permissions().mode() & 0o777;
    assert_eq!(mode, 0o600, "the savegame mode must survive a save");
    for backup in backups(temp.path()) {
        let mode = fs::metadata(&backup)
            .expect("backup metadata")
            .permissions()
            .mode()
            & 0o777;
        assert_eq!(mode, 0o600, "the backup mode must match the original");
    }
}

#[cfg(unix)]
#[test]
fn saving_through_a_symlink_keeps_the_link() {
    use std::os::unix::fs::symlink;

    let temp = common::TempDir::new("symlink");
    let real = temp.path().join("real.sac");
    let link = temp.path().join("save.sac");
    fs::write(&real, common::container(&common::character_gff())).expect("write real");
    symlink("real.sac", &link).expect("symlink");
    let before = fs::read(&real).expect("read real");

    let mut save = SaveGame::open(&link).expect("open through link");
    let mut character = save.character().expect("character");
    character.level = 8;
    save.apply_character(&character).expect("apply");
    save.save(SaveOptions { backup: true }).expect("save");

    assert!(
        fs::symlink_metadata(&link)
            .expect("link metadata")
            .file_type()
            .is_symlink(),
        "saving must not replace the symlink"
    );
    let after = fs::read(&real).expect("read real");
    assert_ne!(before, after, "the symlink target must receive the save");
    SaveGame::open(&real).expect("target parses");
}

#[test]
fn an_external_change_is_not_overwritten() {
    let temp = common::TempDir::new("conflict");
    let path = common::write_save(temp.path(), &common::character_gff());
    let mut save = SaveGame::open(&path).expect("open");
    let mut character = save.character().expect("character");
    character.level = 8;
    save.apply_character(&character).expect("apply");

    fs::write(&path, b"external change").expect("external write");
    let error = save
        .save(SaveOptions { backup: true })
        .expect_err("conflict");
    assert!(matches!(error, Error::Conflict { .. }), "{error}");
    assert_eq!(fs::read(&path).expect("read"), b"external change");
}

#[test]
fn a_save_without_backup_writes_no_backup_file() {
    let temp = common::TempDir::new("no-backup");
    let path = common::write_save(temp.path(), &common::character_gff());
    let mut save = SaveGame::open(&path).expect("open");
    let mut character = save.character().expect("character");
    character.level = 8;
    save.apply_character(&character).expect("apply");
    save.save(SaveOptions { backup: false }).expect("save");
    assert_eq!(backups(temp.path()).len(), 0);
}

#[cfg(unix)]
#[test]
fn a_failed_save_leaves_no_temporary_file() {
    use std::os::unix::fs::PermissionsExt as _;

    let temp = common::TempDir::new("readonly");
    let path = common::write_save(temp.path(), &common::character_gff());
    let mut save = SaveGame::open(&path).expect("open");
    let mut character = save.character().expect("character");
    character.level = 8;
    save.apply_character(&character).expect("apply");
    fs::set_permissions(temp.path(), fs::Permissions::from_mode(0o555)).expect("chmod");

    let result = save.save(SaveOptions { backup: true });
    // Root can still write into a 0555 directory; do not fail the test for
    // that environment.
    if result.is_err() {
        let temporary: Vec<_> = fs::read_dir(temp.path())
            .expect("read folder")
            .flatten()
            .filter(|entry| {
                entry
                    .file_name()
                    .to_string_lossy()
                    .contains(".jadeforge-tmp-")
            })
            .collect();
        assert!(temporary.is_empty(), "a failed save left a temp file");
    }
    fs::set_permissions(temp.path(), fs::Permissions::from_mode(0o755)).expect("restore chmod");
}
