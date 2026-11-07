//! Listing and restoring the backups the editor keeps next to a savegame.
//!
//! [`SaveGame::save`](crate::SaveGame::save) writes a copy of the previous file
//! as `save.sac.jadeforge-backup-<seconds>.<nanos>.<counter>` before replacing
//! it. The file name carries the creation instant in UTC, so a backup can be
//! listed with its timestamp even when the filesystem metadata is unavailable.
//!
//! Restoring a backup is itself a save: the current file is read and verified,
//! a new backup of it is created first, and the chosen backup is verified before
//! it is written atomically. A restore can therefore always be undone by
//! restoring the backup that it creates.

use std::fs;
use std::io::{self, Write as _};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use crate::error::{Error, Result};
use crate::save::{SaveGame, create_backup, parent_directory, resolve_write_target, sync_parent};

/// Marker the editor adds to a backup file name.
pub const BACKUP_MARKER: &str = ".jadeforge-backup-";

/// One backup of a savegame.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BackupEntry {
    /// Absolute or caller-relative path of the backup file.
    pub path: PathBuf,
    /// Creation instant from the file name, falling back to the file metadata.
    pub created: SystemTime,
    /// The savegame this backup belongs to.
    pub original: PathBuf,
    /// Size of the backup in bytes.
    pub size: u64,
}

/// Lists the backups of `original`, newest first.
///
/// The directory is read once. Entries whose name does not match the backup
/// pattern are ignored, and so are entries of a different savegame in the same
/// folder. A missing savegame (or a missing folder) yields an empty list rather
/// than an error: the screen also works before the first save.
///
/// # Errors
///
/// Returns an I/O error when the folder exists but cannot be read, or when a
/// matching entry's metadata cannot be read.
pub fn list_backups(original: &Path) -> Result<Vec<BackupEntry>> {
    let Some(folder) = parent_directory(original) else {
        return Ok(Vec::new());
    };
    let Some(name) = original.file_name().and_then(std::ffi::OsStr::to_str) else {
        return Ok(Vec::new());
    };
    let prefix = format!("{name}{BACKUP_MARKER}");
    let entries = match fs::read_dir(folder) {
        Ok(entries) => entries,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => return Err(Error::io(folder, error)),
    };
    let mut backups = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|error| Error::io(folder, error))?;
        let file_name = entry.file_name();
        let Some(file_name) = file_name.to_str() else {
            continue;
        };
        let Some(suffix) = file_name.strip_prefix(&prefix) else {
            continue;
        };
        let metadata = entry
            .metadata()
            .map_err(|error| Error::io(entry.path(), error))?;
        if !metadata.is_file() {
            continue;
        }
        backups.push(BackupEntry {
            path: entry.path(),
            created: parse_stamp(suffix)
                .or_else(|| metadata.modified().ok())
                .unwrap_or(UNIX_EPOCH),
            original: original.to_path_buf(),
            size: metadata.len(),
        });
    }
    backups.sort_by(|left, right| {
        right
            .created
            .cmp(&left.created)
            .then_with(|| right.path.cmp(&left.path))
    });
    Ok(backups)
}

/// Parses the `<seconds>.<nanos>.<counter>` suffix into a UTC instant.
fn parse_stamp(suffix: &str) -> Option<SystemTime> {
    let mut parts = suffix.split('.');
    let seconds: u64 = parts.next()?.parse().ok()?;
    let nanos: u32 = parts.next()?.parse().ok()?;
    parts.next()?;
    if parts.next().is_some() || nanos >= 1_000_000_000 {
        return None;
    }
    UNIX_EPOCH.checked_add(Duration::new(seconds, nanos))
}

/// Restores `backup` over `original`.
///
/// The backup is parsed and fully verified first, the current file (when it
/// exists) is read and verified, and a new backup of it is written before the
/// rename. The returned path is that safety backup, so the caller can tell the
/// user how to undo the restore.
///
/// A symlinked `original` is written through to its target and the link is
/// kept. The safety backup stays next to the caller's path so the backup
/// browser and the reported path use the same visible name.
///
/// # Errors
///
/// Returns an error when the backup is not a valid savegame, when the current
/// file cannot be read or backed up, or when the atomic replacement fails. The
/// current file is never touched in those cases.
pub fn restore_backup(backup: &Path, original: &Path) -> Result<PathBuf> {
    let restored = SaveGame::open(backup)?;
    restored.verify()?;
    let restored_bytes = fs::read(backup).map_err(|error| Error::io(backup, error))?;
    let write_target = resolve_write_target(original)?;

    let safety_backup = if original.exists() {
        let current = SaveGame::open(original)?;
        current.verify()?;
        Some(create_backup(original)?)
    } else {
        None
    };

    let (temporary, mut file) = create_restore_file(&write_target)?;
    let result = (|| -> Result<()> {
        file.write_all(&restored_bytes)
            .map_err(|error| Error::io(&temporary, error))?;
        file.sync_all()
            .map_err(|error| Error::io(&temporary, error))?;
        if let Ok(metadata) = fs::metadata(&write_target) {
            fs::set_permissions(&temporary, metadata.permissions())
                .map_err(|error| Error::io(&temporary, error))?;
            file.sync_all()
                .map_err(|error| Error::io(&temporary, error))?;
        }
        drop(file);
        fs::rename(&temporary, &write_target).map_err(|error| Error::io(&write_target, error))?;
        sync_parent(&write_target)?;
        Ok(())
    })();
    if let Err(error) = result {
        let _ = fs::remove_file(&temporary);
        return Err(error);
    }

    Ok(safety_backup.unwrap_or_else(|| backup.to_path_buf()))
}

/// Creates an exclusive temporary sibling for the restore write.
fn create_restore_file(target: &Path) -> Result<(PathBuf, fs::File)> {
    let name = target.file_name().map_or_else(
        || "save.sac".to_owned(),
        |name| name.to_string_lossy().into_owned(),
    );
    for attempt in 0..64_u32 {
        let path = target.with_file_name(format!(
            ".{name}.jadeforge-restore-{}-{attempt}",
            std::process::id()
        ));
        match fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
        {
            Ok(file) => return Ok((path, file)),
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {}
            Err(error) => return Err(Error::io(&path, error)),
        }
    }
    Err(Error::Corrupt(format!(
        "could not create an exclusive restore file next to {}",
        target.display()
    )))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn backup_names_carry_a_utc_instant() {
        // Windows SystemTime has 100 ns resolution; use a value that survives
        // the round trip on every supported platform.
        let stamp = parse_stamp("1700000000.123456700.7").expect("stamp");
        assert_eq!(
            stamp.duration_since(UNIX_EPOCH).expect("after epoch"),
            Duration::new(1_700_000_000, 123_456_700)
        );
        assert!(parse_stamp("not-a-stamp").is_none());
        assert!(parse_stamp("1.2").is_none());
        assert!(parse_stamp("1.1000000000.3").is_none());
        assert!(parse_stamp("1.2.3.4").is_none());
    }
}
