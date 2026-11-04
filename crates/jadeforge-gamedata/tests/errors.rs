//! Malformed game-data files.
//!
//! RIM archives, talk tables and 2DA tables are built in memory here, so these
//! tests need no installation.

use std::path::PathBuf;

use jadeforge_gamedata::{RimArchive, TalkTable, TwoDa};

fn temp_dir() -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "jadeforge-gamedata-errors-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |duration| duration.as_nanos())
    ));
    std::fs::create_dir_all(&dir).expect("temp dir");
    dir
}

fn rim(entries: &[(&str, u32, u32, &[u8])], table_offset: u32, count: u32) -> Vec<u8> {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(b"RIM V1.0");
    bytes.extend_from_slice(&0_u32.to_le_bytes());
    bytes.extend_from_slice(&count.to_le_bytes());
    bytes.extend_from_slice(&table_offset.to_le_bytes());
    for (name, kind, offset, payload) in entries {
        let mut entry = [0_u8; 32];
        let name = name.as_bytes();
        entry[..name.len()].copy_from_slice(name);
        entry[16..20].copy_from_slice(&kind.to_le_bytes());
        entry[20..24].copy_from_slice(&1_u32.to_le_bytes());
        entry[24..28].copy_from_slice(&offset.to_le_bytes());
        entry[28..32].copy_from_slice(&u32::try_from(payload.len()).expect("size").to_le_bytes());
        bytes.extend_from_slice(&entry);
    }
    // Payloads are appended where the entries point; this helper only supports
    // payloads placed directly after the entry table.
    for (_, _, _, payload) in entries {
        bytes.extend_from_slice(payload);
    }
    bytes
}

#[test]
fn rim_archives_are_read_and_checked() {
    let dir = temp_dir();
    let path = dir.join("test.rim");
    let payload = b"2DA V2.0\n\nCOL\n0 value\n";
    let data = rim(&[("gems", 2017, 52, payload)], 20, 1);
    std::fs::write(&path, &data).expect("write rim");
    let archive = RimArchive::open(&path).expect("open rim");
    assert_eq!(archive.entries().len(), 1);
    assert_eq!(archive.path(), path.as_path());
    assert_eq!(archive.names().collect::<Vec<_>>(), vec!["gems"]);
    assert_eq!(archive.get("gems"), Some(payload.as_slice()));
    assert_eq!(archive.get_2da("gems"), Some(payload.as_slice()));
    assert!(archive.get("missing").is_none());
    assert!(archive.get_2da("gems").is_some());

    for bad in [
        Vec::new(),
        b"NOTRIM".to_vec(),
        rim(&[("x", 2017, 52, b"data")], 20, 99),
        rim(&[("x", 2017, 52, b"data")], 999, 1),
    ] {
        std::fs::write(&path, bad).expect("write bad rim");
        assert!(RimArchive::open(&path).is_err());
    }

    std::fs::write(&path, rim(&[("notatable", 9999, 52, b"x")], 20, 1)).expect("write");
    let archive = RimArchive::open(&path).expect("open");
    assert!(archive.get_2da("notatable").is_none());

    // A payload offset beyond the archive is accepted at open but unavailable.
    std::fs::write(&path, rim(&[("x", 2017, 999, b"data")], 20, 1)).expect("write");
    let archive = RimArchive::open(&path).expect("open broken entry");
    assert!(archive.get("x").is_none());

    let _ = std::fs::remove_dir_all(dir);
}

fn tlk(
    version: [u8; 4],
    languages: u32,
    count: u32,
    entry_offset: u32,
    data_offset: u32,
) -> Vec<u8> {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(b"TLK ");
    bytes.extend_from_slice(&version);
    bytes.extend_from_slice(&languages.to_le_bytes());
    bytes.extend_from_slice(&count.to_le_bytes());
    bytes.extend_from_slice(&entry_offset.to_le_bytes());
    bytes.extend_from_slice(&data_offset.to_le_bytes());
    bytes
}

fn entry(flags: u32, offset: u32, length: u16) -> Vec<u8> {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&flags.to_le_bytes());
    bytes.extend_from_slice(&offset.to_le_bytes());
    bytes.extend_from_slice(&length.to_le_bytes());
    bytes
}

#[test]
fn talk_tables_are_read_and_checked() {
    let mut data = tlk(*b"V4.0", 1, 2, 24, 44);
    data.extend_from_slice(&entry(0, 44, 5));
    data.extend_from_slice(&entry(1, 49, 0));
    data.extend_from_slice(b"Hello");
    let table = TalkTable::parse(PathBuf::from("dialog.tlk"), data.clone()).expect("parse tlk");
    assert_eq!(table.len(), 2);
    assert!(!table.is_empty());
    assert_eq!(table.path(), std::path::Path::new("dialog.tlk"));
    assert_eq!(table.get(0).as_deref(), Some("Hello"));
    assert_eq!(table.get(1).as_deref(), Some(""));
    assert_eq!(table.get_bytes(0), Some(b"Hello".as_slice()));
    assert!(table.get(9).is_none());

    for bad in [
        Vec::new(),
        tlk(*b"V3.0", 1, 0, 24, 24),
        tlk(*b"V4.0", 1, 2, 999, 44),
        tlk(*b"V4.0", 1, 1, 24, 999),
    ] {
        assert!(TalkTable::parse(PathBuf::from("bad.tlk"), bad).is_err());
    }

    // Entry strings pointing beyond the data section are ignored by `get`.
    let mut out_of_bounds = tlk(*b"V4.0", 1, 1, 24, 34);
    out_of_bounds.extend_from_slice(&entry(0, 999, 5));
    let table = TalkTable::parse(PathBuf::from("dialog.tlk"), out_of_bounds).expect("parse");
    assert!(table.get(0).is_none());
}

#[test]
fn two_da_tables_reject_malformed_input() {
    assert!(TwoDa::parse("").is_err());
    assert!(TwoDa::parse("2DA V2.0").is_err());
    assert!(TwoDa::parse_binary(b"2DA V2.b\nnot really").is_err());
    assert!(TwoDa::parse_binary(b"2DA V2.b\n\0").is_err());

    let table = TwoDa::parse("2DA V2.0\n\nLABEL TYPE\n0 \"hello world\" gems\n").expect("text 2da");
    assert_eq!(table.columns(), &["LABEL".to_owned(), "TYPE".to_owned()]);
    assert_eq!(table.len(), 1);
    assert!(!table.is_empty());
    let row = table.row(0).expect("row");
    assert_eq!(row.cell(0), Some("hello world"));
    assert_eq!(row.cell(9), None);
    assert_eq!(row.uint(1), None);
    assert_eq!(table.column("type"), Some(1));
}
