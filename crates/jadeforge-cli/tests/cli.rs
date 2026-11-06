#![allow(clippy::expect_used)] // tests fail loudly; expects are assertions

//! End-to-end tests for the `jadeforge` binary.
//!
//! The tests build a small synthetic savegame and invoke the real executable,
//! so argument parsing, dispatch, output and exit codes are covered too.

use std::path::PathBuf;
use std::process::{Command, Output, Stdio};

use jadeforge_core::gff::Value;
use jadeforge_core::{Gff, LocString, SaveGame};

#[allow(clippy::too_many_lines)] // one field per line keeps the fixture readable
fn build_block() -> Gff {
    let mut gff = Gff {
        version: *b"V3.2",
        labels: Vec::new(),
        structs: Vec::new(),
        root: 0,
    };
    let root = gff.push_struct(u32::MAX);
    let player = gff.push_struct(0);
    let stats = gff.push_struct(0);
    let item = gff.push_struct(0);
    let style = gff.push_struct(0);
    let henchmen = gff.push_struct(0);
    let party_member = gff.push_struct(0);

    gff.add_field(root, "PlayerCreature", Value::Struct(player))
        .expect("player field");
    gff.add_field(player, "FirstName", Value::String("Test Hero".to_owned()))
        .expect("name");
    gff.add_field(player, "Level", Value::Byte(5))
        .expect("level");
    gff.add_field(player, "CurrentXP", Value::DWord(1234))
        .expect("xp");
    gff.add_field(player, "Stats", Value::Struct(stats))
        .expect("stats");
    gff.add_field(stats, "Gold", Value::DWord(99))
        .expect("gold");
    gff.add_field(stats, "UnassignedPts", Value::DWord(2))
        .expect("unspent points");
    gff.add_field(stats, "WordField", Value::Word(1))
        .expect("word field");
    gff.add_field(stats, "CharField", Value::Char(-2))
        .expect("char field");
    gff.add_field(stats, "ShortField", Value::Short(-3))
        .expect("short field");
    gff.add_field(stats, "IntField", Value::Int(-4))
        .expect("int field");
    gff.add_field(stats, "DWord64Field", Value::DWord64(5))
        .expect("dword64 field");
    gff.add_field(stats, "Int64Field", Value::Int64(-6))
        .expect("int64 field");
    gff.add_field(stats, "FloatField", Value::Float(1.5))
        .expect("float field");
    gff.add_field(stats, "DoubleField", Value::Double(2.5))
        .expect("double field");
    gff.add_field(stats, "StringField", Value::String("old".to_owned()))
        .expect("string field");
    gff.add_field(stats, "ResRefField", Value::ResRef("old".to_owned()))
        .expect("resref field");
    gff.add_field(
        stats,
        "LocStringField",
        Value::LocString(LocString {
            strref: Some(7),
            text: Some("old".to_owned()),
        }),
    )
    .expect("locstring field");
    gff.add_field(
        stats,
        "LocStringRef",
        Value::LocString(LocString::from_strref(8)),
    )
    .expect("locstring reference");
    gff.add_field(stats, "VoidField", Value::Void(vec![1, 2, 3]))
        .expect("void field");
    gff.add_field(
        stats,
        "QuaternionField",
        Value::Quaternion([1.0, 2.0, 3.0, 4.0]),
    )
    .expect("quaternion field");
    gff.add_field(stats, "VectorField", Value::Vector3([1.0, 2.0, 3.0]))
        .expect("vector field");
    gff.add_field(stats, "Styles", Value::List(vec![style]))
        .expect("styles");
    gff.add_field(style, "StyleId", Value::Word(1))
        .expect("style id");
    gff.add_field(style, "StyleLevel", Value::Byte(2))
        .expect("style level");
    gff.add_field(style, "StyleLevelInfo", Value::DWord64(1))
        .expect("style mask");
    gff.add_field(player, "ItemList", Value::List(vec![item]))
        .expect("items");
    gff.add_field(item, "Count", Value::Byte(2)).expect("count");
    gff.add_field(item, "ItemId", Value::Word(42))
        .expect("item id");
    gff.add_field(item, "Type", Value::Byte(6))
        .expect("item type");
    gff.add_field(player, "Henchmen", Value::Struct(henchmen))
        .expect("henchmen");
    gff.add_field(henchmen, "HenchList", Value::List(vec![party_member]))
        .expect("hench list");
    gff.add_field(
        party_member,
        "BaseResref",
        Value::String("test_hench".to_owned()),
    )
    .expect("resref");
    gff.add_field(party_member, "Level", Value::Byte(3))
        .expect("hench level");
    gff.add_field(party_member, "Available", Value::Byte(1))
        .expect("available");
    gff
}

fn container(gff: &Gff) -> Vec<u8> {
    let block = gff.to_bytes().expect("serialize block");
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&1_u32.to_le_bytes());
    bytes.extend_from_slice(&9_u32.to_le_bytes());
    bytes.extend_from_slice(b"save.sav\0");
    bytes.extend_from_slice(
        &u32::try_from(block.len())
            .expect("block size")
            .to_le_bytes(),
    );
    bytes.extend_from_slice(&block);
    bytes
}

struct Fixture {
    dir: PathBuf,
    save: PathBuf,
    original: Vec<u8>,
}

impl Fixture {
    fn new() -> Self {
        Self::with_block(&build_block())
    }

    fn without_item_list() -> Self {
        let mut block = build_block();
        let _ = block
            .remove("PlayerCreature.ItemList")
            .expect("remove item list");
        Self::with_block(&block)
    }

    fn with_block(block: &Gff) -> Self {
        let dir = std::env::temp_dir().join(format!(
            "jadeforge-cli-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_or(0, |duration| duration.as_nanos())
        ));
        std::fs::create_dir_all(&dir).expect("temp dir");
        let save = dir.join("save.sac");
        let original = container(block);
        std::fs::write(&save, &original).expect("write save");
        Self {
            dir,
            save,
            original,
        }
    }

    fn run(&self, args: &[&str]) -> Output {
        Command::new(env!("CARGO_BIN_EXE_jadeforge-cli"))
            .arg(&self.save)
            .args(args)
            .output()
            .expect("run jadeforge")
    }

    /// Runs without the two environment overrides, so game-data fallbacks are
    /// exercised even on a developer machine that has the game installed.
    fn run_clean_env(&self, args: &[&str]) -> Output {
        Command::new(env!("CARGO_BIN_EXE_jadeforge-cli"))
            .arg(&self.save)
            .args(args)
            .env_remove("JADEFORGE_GAME_DIR")
            .env_remove("JADEFORGE_SAVE_DIR")
            .output()
            .expect("run jadeforge")
    }

    fn backups(&self) -> usize {
        std::fs::read_dir(&self.dir)
            .expect("read dir")
            .flatten()
            .filter(|entry| {
                entry
                    .file_name()
                    .to_string_lossy()
                    .contains("jadeforge-backup")
            })
            .count()
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

fn stdout(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

#[test]
fn read_commands_print_the_savegame() {
    let fixture = Fixture::new();

    let output = fixture.run(&["blocks"]);
    assert!(output.status.success(), "{}", stderr(&output));
    assert!(stdout(&output).contains("save.sav"));

    let output = fixture.run(&["info"]);
    assert!(output.status.success(), "{}", stderr(&output));
    assert!(stdout(&output).contains("Name:     Test Hero"));
    assert!(stdout(&output).contains("Unspent attribute points: 2"));

    let output = fixture.run(&["character"]);
    assert!(output.status.success(), "{}", stderr(&output));
    assert!(stdout(&output).contains("Test Hero"));

    let output = fixture.run(&["get", "save.sav", "PlayerCreature.Level"]);
    assert!(output.status.success(), "{}", stderr(&output));
    assert!(
        stdout(&output).contains("= 5 (type byte)"),
        "{}",
        stdout(&output)
    );

    let output = fixture.run(&["dump"]);
    assert!(output.status.success(), "{}", stderr(&output));
    assert!(
        stdout(&output).contains("\"PlayerCreature\""),
        "{}",
        stdout(&output)
    );

    let output = fixture.run(&["dump", "--path", "PlayerCreature.FirstName"]);
    assert!(output.status.success(), "{}", stderr(&output));
    assert!(stdout(&output).contains("Test Hero"));

    let output = fixture.run(&["dump", "--json"]);
    assert!(output.status.success(), "{}", stderr(&output));
    serde_json::from_slice::<serde_json::Value>(&output.stdout).expect("valid JSON");

    let output = fixture.run(&["verify"]);
    assert!(output.status.success(), "{}", stderr(&output));
    assert!(stdout(&output).contains("ok: 1 blocks"));

    let output = fixture.run(&["items"]);
    assert!(output.status.success(), "{}", stderr(&output));
    assert!(stdout(&output).contains("42"));

    let output = fixture.run(&["styles"]);
    assert!(output.status.success(), "{}", stderr(&output));
    // The name depends on whether game data is available; the id does not.
    assert!(stdout(&output).contains("level"), "{}", stdout(&output));

    let output = fixture.run(&["companions"]);
    assert!(output.status.success(), "{}", stderr(&output));
    assert!(stdout(&output).contains("test_hench"));
}

#[test]
fn set_dry_run_does_not_touch_the_file() {
    let fixture = Fixture::new();
    let output = fixture.run(&["set", "save.sav", "PlayerCreature.Level", "9", "--dry-run"]);
    assert!(output.status.success(), "{}", stderr(&output));
    assert_eq!(
        std::fs::read(&fixture.save).expect("read save"),
        fixture.original
    );

    let output = fixture.run(&["set", "save.sav", "PlayerCreature.Level", "9"]);
    assert!(output.status.success(), "{}", stderr(&output));

    let output = fixture.run(&["get", "save.sav", "PlayerCreature.Level"]);
    assert!(
        stdout(&output).contains("= 9 (type byte)"),
        "{}",
        stdout(&output)
    );

    let backup = std::fs::read_dir(&fixture.dir)
        .expect("read dir")
        .flatten()
        .any(|entry| {
            entry
                .file_name()
                .to_string_lossy()
                .contains("jadeforge-backup")
        });
    assert!(backup, "set did not create a backup next to the savegame");
}

#[test]
fn bad_inputs_exit_with_an_error() {
    let fixture = Fixture::new();

    let output = fixture.run(&["get", "save.sav", "PlayerCreature.Nope"]);
    assert!(!output.status.success());
    assert!(stderr(&output).contains("error:"), "{}", stderr(&output));

    let output = fixture.run(&["get", "missing.sav", "PlayerCreature.Level"]);
    assert!(!output.status.success());
    assert!(
        stderr(&output).contains("does not exist"),
        "{}",
        stderr(&output)
    );

    let output = fixture.run(&["set", "save.sav", "PlayerCreature.Level", "not-a-number"]);
    assert!(!output.status.success());

    let output = Command::new(env!("CARGO_BIN_EXE_jadeforge-cli"))
        .arg(fixture.dir.join("nope.sac"))
        .arg("info")
        .output()
        .expect("run jadeforge");
    assert!(!output.status.success());
    assert!(stderr(&output).contains("does not exist"));
}

#[test]
fn directory_argument_resolves_to_save_sac() {
    let fixture = Fixture::new();
    let output = Command::new(env!("CARGO_BIN_EXE_jadeforge-cli"))
        .arg(fixture.dir.to_str().expect("utf8 path"))
        .arg("blocks")
        .output()
        .expect("run jadeforge");
    assert!(output.status.success(), "{}", stderr(&output));
    assert!(stdout(&output).contains("save.sav"));
}

#[test]
fn missing_save_argument_fails() {
    let output = Command::new(env!("CARGO_BIN_EXE_jadeforge-cli"))
        .arg("definitely-not-a-save")
        .arg("info")
        .output()
        .expect("run jadeforge");
    assert!(!output.status.success());
    assert!(
        stderr(&output).contains("does not exist"),
        "{}",
        stderr(&output)
    );
}

#[test]
fn set_parses_every_field_type() {
    let fixture = Fixture::new();
    let cases = [
        ("PlayerCreature.Level", "9", "= 9 (type byte)"),
        (
            "PlayerCreature.Stats.WordField",
            "65000",
            "= 65000 (type word)",
        ),
        ("PlayerCreature.Stats.CharField", "-7", "= -7 (type char)"),
        (
            "PlayerCreature.Stats.ShortField",
            "-300",
            "= -300 (type short)",
        ),
        (
            "PlayerCreature.Stats.IntField",
            "-2000",
            "= -2000 (type int)",
        ),
        (
            "PlayerCreature.Stats.DWord64Field",
            "9000000000",
            "= 9000000000 (type dword64)",
        ),
        (
            "PlayerCreature.Stats.Int64Field",
            "-9000000000",
            "= -9000000000 (type int64)",
        ),
        (
            "PlayerCreature.Stats.FloatField",
            "2.25",
            "= 2.25 (type float)",
        ),
        (
            "PlayerCreature.Stats.DoubleField",
            "3.5",
            "= 3.5 (type double)",
        ),
        (
            "PlayerCreature.Stats.StringField",
            "new",
            "= new (type string)",
        ),
        (
            "PlayerCreature.Stats.ResRefField",
            "newres",
            "= newres (type resref)",
        ),
        (
            "PlayerCreature.Stats.LocStringField",
            "newloc",
            "= newloc (type locstring)",
        ),
        (
            "PlayerCreature.Stats.UnassignedPts",
            "7",
            "= 7 (type dword)",
        ),
    ];

    for (path, raw, expected) in cases {
        let mut args = vec!["set", "save.sav", path];
        if raw.starts_with('-') {
            args.push("--");
        }
        args.push(raw);
        let output = fixture.run(&args);
        assert!(output.status.success(), "{}: {}", path, stderr(&output));
        let output = fixture.run(&["get", "save.sav", path]);
        assert!(
            stdout(&output).contains(expected),
            "{path}: {}",
            stdout(&output)
        );
    }
}

#[test]
fn get_renders_complex_values() {
    let fixture = Fixture::new();
    let cases = [
        ("PlayerCreature.ItemList", "<list of 1>"),
        ("PlayerCreature.Stats", "<struct "),
        ("PlayerCreature.Stats.VoidField", "<3 bytes>"),
        ("PlayerCreature.Stats.QuaternionField", "1.0"),
        ("PlayerCreature.Stats.VectorField", "2.0"),
        ("PlayerCreature.Stats.LocStringRef", "<strref 8>"),
    ];

    for (path, expected) in cases {
        let output = fixture.run(&["get", "save.sav", path]);
        assert!(output.status.success(), "{path}: {}", stderr(&output));
        assert!(
            stdout(&output).contains(expected),
            "{path}: {}",
            stdout(&output)
        );
    }
}

#[test]
fn set_rejects_non_editable_complex_fields() {
    let fixture = Fixture::new();
    for path in [
        "PlayerCreature.Stats",
        "PlayerCreature.ItemList",
        "PlayerCreature.Stats.VoidField",
        "PlayerCreature.Stats.QuaternionField",
        "PlayerCreature.Stats.VectorField",
    ] {
        let output = fixture.run(&["set", "save.sav", path, "1"]);
        assert!(!output.status.success(), "{path} was accepted");
        assert!(
            stderr(&output).contains("cannot be edited"),
            "{path}: {}",
            stderr(&output)
        );
    }
    assert_eq!(
        std::fs::read(&fixture.save).expect("read save"),
        fixture.original
    );
}

#[test]
fn add_item_dry_run_does_not_write() {
    let fixture = Fixture::new();
    let output = fixture.run(&["add-item", "1234", "--dry-run"]);
    assert!(output.status.success(), "{}", stderr(&output));
    assert!(stdout(&output).contains("would add item 1234"));
    assert_eq!(
        std::fs::read(&fixture.save).expect("read save"),
        fixture.original
    );
    assert_eq!(fixture.backups(), 0);
}

#[test]
fn add_item_dry_run_still_validates() {
    let fixture = Fixture::new();
    let output = fixture.run(&["add-item", "1234", "--count", "0", "--dry-run"]);
    assert!(!output.status.success());

    let fixture = Fixture::without_item_list();
    let output = fixture.run(&["add-item", "1234", "--dry-run"]);
    assert!(!output.status.success());
    assert_eq!(
        std::fs::read(&fixture.save).expect("read save"),
        fixture.original
    );
}

#[test]
fn a_closed_stdout_pipe_is_not_an_error() {
    let mut block = build_block();
    block
        .add_field(0, "Blob", Value::Void(vec![0_u8; 200_000]))
        .expect("blob");
    let fixture = Fixture::with_block(&block);
    let mut child = Command::new(env!("CARGO_BIN_EXE_jadeforge-cli"))
        .arg(&fixture.save)
        .arg("dump")
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn");
    drop(child.stdout.take());
    let status = child.wait().expect("wait");
    assert!(status.success(), "a broken pipe must exit successfully");
}

#[test]
fn add_item_writes_the_item_and_a_backup() {
    let fixture = Fixture::new();
    let output = fixture.run(&["add-item", "1234", "--kind", "7", "--count", "2"]);
    assert!(output.status.success(), "{}", stderr(&output));
    assert!(stdout(&output).contains("added item 1234"));

    let save = SaveGame::open(&fixture.save).expect("reopen");
    let item = save
        .items()
        .expect("items")
        .into_iter()
        .find(|item| item.item_id == 1234)
        .expect("added item");
    assert_eq!(item.count, 2);
    assert_eq!(item.kind, 7);
    assert_eq!(fixture.backups(), 1);
}

#[test]
fn add_item_rejects_zero_count_and_a_missing_item_list() {
    let fixture = Fixture::new();
    let output = fixture.run(&["add-item", "1234", "--count", "0"]);
    assert!(!output.status.success());
    assert!(
        stderr(&output).contains("item count"),
        "{}",
        stderr(&output)
    );
    assert_eq!(
        std::fs::read(&fixture.save).expect("read save"),
        fixture.original
    );

    let fixture = Fixture::without_item_list();
    let output = fixture.run(&["add-item", "1234"]);
    assert!(!output.status.success());
    assert!(
        stderr(&output).contains("does not exist"),
        "{}",
        stderr(&output)
    );
    assert_eq!(
        std::fs::read(&fixture.save).expect("read save"),
        fixture.original
    );
}

#[test]
fn no_backup_actions_leave_no_backup_file() {
    let fixture = Fixture::new();
    let output = fixture.run(&[
        "set",
        "save.sav",
        "PlayerCreature.Level",
        "9",
        "--no-backup",
    ]);
    assert!(output.status.success(), "{}", stderr(&output));
    assert_eq!(fixture.backups(), 0);

    let fixture = Fixture::new();
    let output = fixture.run(&["add-item", "1234", "--no-backup"]);
    assert!(output.status.success(), "{}", stderr(&output));
    assert_eq!(fixture.backups(), 0);
}

#[test]
fn set_rejects_a_missing_block() {
    let fixture = Fixture::new();
    let output = fixture.run(&["set", "missing.sav", "PlayerCreature.Level", "9"]);
    assert!(!output.status.success());
    assert!(
        stderr(&output).contains("does not exist"),
        "{}",
        stderr(&output)
    );
    assert_eq!(
        std::fs::read(&fixture.save).expect("read save"),
        fixture.original
    );
}

#[test]
fn no_save_argument_uses_the_current_directory() {
    let fixture = Fixture::new();
    let output = Command::new(env!("CARGO_BIN_EXE_jadeforge-cli"))
        .current_dir(&fixture.dir)
        .arg("blocks")
        .output()
        .expect("run jadeforge");
    assert!(output.status.success(), "{}", stderr(&output));
    assert!(stdout(&output).contains("save.sav"));
}

#[test]
fn a_directory_without_save_sac_fails() {
    let fixture = Fixture::new();
    let empty = fixture.dir.join("empty");
    std::fs::create_dir_all(&empty).expect("create empty folder");
    let output = Command::new(env!("CARGO_BIN_EXE_jadeforge-cli"))
        .arg(&empty)
        .arg("blocks")
        .output()
        .expect("run jadeforge");
    assert!(!output.status.success());
    assert!(
        stderr(&output).contains("does not contain a save.sac"),
        "{}",
        stderr(&output)
    );
}

#[test]
fn item_and_style_names_use_the_bundled_index_without_game_data() {
    let fixture = Fixture::new();

    let output = fixture.run_clean_env(&["items"]);
    assert!(output.status.success(), "{}", stderr(&output));
    assert!(
        stdout(&output).contains("The Bronze Tongue"),
        "{}",
        stdout(&output)
    );
    assert!(
        stdout(&output).contains("gems"),
        "the bundled category is used: {}",
        stdout(&output)
    );

    let output = fixture.run_clean_env(&["styles"]);
    assert!(output.status.success(), "{}", stderr(&output));
    assert!(
        stdout(&output).contains("Paralyzing Palm"),
        "{}",
        stdout(&output)
    );
}

#[test]
fn unknown_item_ids_still_fall_back_without_game_data() {
    let fixture = Fixture::new();
    let output = fixture.run_clean_env(&["add-item", "65000", "--kind", "8"]);
    assert!(output.status.success(), "{}", stderr(&output));

    let output = fixture.run_clean_env(&["items"]);
    assert!(output.status.success(), "{}", stderr(&output));
    assert!(
        stdout(&output).contains("<item 65000>"),
        "{}",
        stdout(&output)
    );
    assert!(stdout(&output).contains("improve"), "{}", stdout(&output));
}

#[test]
fn export_names_rejects_a_broken_game_installation() {
    let fixture = Fixture::new();
    let output_path = fixture.dir.join("names.toml");
    let output_arg = output_path.to_string_lossy();
    let output = fixture.run_clean_env(&[
        "export-names",
        "--game",
        "/definitely/not/a/game",
        "--output",
        &output_arg,
    ]);
    assert!(!output.status.success());
    assert!(
        stderr(&output).contains("cannot read game data"),
        "{}",
        stderr(&output)
    );
    assert!(
        !output_path.exists(),
        "a failed export must not write a file"
    );
}
