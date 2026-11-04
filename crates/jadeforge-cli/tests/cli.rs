//! End-to-end tests for the `jadeforge` binary.
//!
//! The tests build a small synthetic savegame and invoke the real executable,
//! so argument parsing, dispatch, output and exit codes are covered too.

use std::path::PathBuf;
use std::process::{Command, Output};

use jadeforge_core::Gff;
use jadeforge_core::gff::Value;

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
        let dir = std::env::temp_dir().join(format!(
            "jadeforge-cli-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_or(0, |duration| duration.as_nanos())
        ));
        std::fs::create_dir_all(&dir).expect("temp dir");
        let save = dir.join("save.sac");
        let original = container(&build_block());
        std::fs::write(&save, &original).expect("write save");
        Self {
            dir,
            save,
            original,
        }
    }

    fn run(&self, args: &[&str]) -> Output {
        Command::new(env!("CARGO_BIN_EXE_jadeforge"))
            .arg(&self.save)
            .args(args)
            .output()
            .expect("run jadeforge")
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

    let output = Command::new(env!("CARGO_BIN_EXE_jadeforge"))
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
    let output = Command::new(env!("CARGO_BIN_EXE_jadeforge"))
        .arg(fixture.dir.to_str().expect("utf8 path"))
        .arg("blocks")
        .output()
        .expect("run jadeforge");
    assert!(output.status.success(), "{}", stderr(&output));
    assert!(stdout(&output).contains("save.sav"));
}

#[test]
fn missing_save_argument_fails() {
    let output = Command::new(env!("CARGO_BIN_EXE_jadeforge"))
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
