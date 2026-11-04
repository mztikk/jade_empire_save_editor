//! End to end tests for the command line tool.
//!
//! A minimal savegame is built from scratch so the tests do not need the game
//! installed.

use std::path::PathBuf;
use std::process::Command;

use jadeforge_core::Gff;
use jadeforge_core::gff::Value;

fn player_block() -> Gff {
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
    gff.add_field(player, "FirstName", Value::String("Test Hero".to_owned()))
        .expect("name");
    gff.add_field(player, "Level", Value::Byte(7))
        .expect("level");
    gff.add_field(player, "CurrentXP", Value::DWord(4242))
        .expect("xp");
    gff.add_field(player, "Stats", Value::Struct(stats))
        .expect("stats");
    gff.add_field(stats, "Gold", Value::DWord(100))
        .expect("gold");
    gff.add_field(stats, "Health", Value::Float(50.0))
        .expect("health");
    gff.add_field(campaign, "Chapter", Value::DWord(2))
        .expect("chapter");
    gff
}

fn write_savegame(path: &PathBuf) {
    let bytes = player_block().to_bytes().expect("serialize");
    let mut container = Vec::new();
    container.extend_from_slice(&1_u32.to_le_bytes());
    let name = b"save.sav\0";
    container.extend_from_slice(&(name.len() as u32).to_le_bytes());
    container.extend_from_slice(name);
    container.extend_from_slice(&(bytes.len() as u32).to_le_bytes());
    container.extend_from_slice(&bytes);
    std::fs::write(path, container).expect("write savegame");
}

fn fixture() -> (PathBuf, PathBuf) {
    let root = std::env::temp_dir().join(format!(
        "jadeforge-cli-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |duration| duration.as_nanos())
    ));
    std::fs::create_dir_all(&root).expect("create dir");
    let save = root.join("save.sac");
    write_savegame(&save);
    (root, save)
}

fn jadeforge() -> Command {
    Command::new(env!("CARGO_BIN_EXE_jadeforge"))
}

#[test]
fn info_prints_the_character() {
    let (root, save) = fixture();
    let output = jadeforge().arg(&save).arg("info").output().expect("run");
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Test Hero"), "{stdout}");
    assert!(stdout.contains("Level:    7"), "{stdout}");
    assert!(stdout.contains("Chapter:  2"), "{stdout}");
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn get_and_set_round_trip() {
    let (root, save) = fixture();
    let get = jadeforge()
        .arg(&save)
        .args(["get", "save.sav", "PlayerCreature.Stats.Gold"])
        .output()
        .expect("run");
    assert!(get.status.success());
    assert!(String::from_utf8_lossy(&get.stdout).contains("100"));

    let set = jadeforge()
        .arg(&save)
        .args([
            "set",
            "save.sav",
            "PlayerCreature.Stats.Gold",
            "9999",
            "--no-backup",
        ])
        .output()
        .expect("run");
    assert!(
        set.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&set.stderr)
    );

    let get = jadeforge()
        .arg(&save)
        .args(["get", "save.sav", "PlayerCreature.Stats.Gold"])
        .output()
        .expect("run");
    assert!(String::from_utf8_lossy(&get.stdout).contains("9999"));

    // The savegame must still verify and keep its shape.
    let verify = jadeforge().arg(&save).arg("verify").output().expect("run");
    assert!(
        verify.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&verify.stderr)
    );
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn dry_run_does_not_touch_the_file() {
    let (root, save) = fixture();
    let before = std::fs::read(&save).expect("read");
    let output = jadeforge()
        .arg(&save)
        .args([
            "set",
            "save.sav",
            "PlayerCreature.Stats.Gold",
            "5",
            "--dry-run",
        ])
        .output()
        .expect("run");
    assert!(output.status.success());
    assert!(String::from_utf8_lossy(&output.stdout).contains("dry run"));
    let after = std::fs::read(&save).expect("read");
    assert_eq!(before, after);
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn unknown_field_fails_cleanly() {
    let (root, save) = fixture();
    let output = jadeforge()
        .arg(&save)
        .args(["get", "save.sav", "PlayerCreature.DoesNotExist"])
        .output()
        .expect("run");
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("does not exist"), "{stderr}");
    assert!(!stderr.contains("panicked"), "{stderr}");
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn dump_prints_json() {
    let (root, save) = fixture();
    let output = jadeforge()
        .arg(&save)
        .args(["dump", "save.sav", "--path", "PlayerCreature"])
        .output()
        .expect("run");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("\"FirstName\""), "{stdout}");
    assert!(stdout.contains("Test Hero"), "{stdout}");
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn accepts_a_folder_instead_of_a_file() {
    let (root, save) = fixture();
    let folder = save.parent().expect("parent").to_path_buf();
    let output = jadeforge()
        .arg(&folder)
        .arg("blocks")
        .output()
        .expect("run");
    assert!(output.status.success());
    assert!(String::from_utf8_lossy(&output.stdout).contains("save.sav"));
    let _ = std::fs::remove_dir_all(root);
}
