#![allow(clippy::expect_used)] // tests fail loudly; expects are assertions

//! Deterministic hostile-input pass over the game-data readers.

use std::path::PathBuf;

use jadeforge_gamedata::rim::RimArchive;
use jadeforge_gamedata::talktable::TalkTable;
use jadeforge_gamedata::twoda::TwoDa;

fn next(state: &mut u64) -> u64 {
    *state ^= *state << 13;
    *state ^= *state >> 7;
    *state ^= *state << 17;
    *state
}

#[test]
fn hostile_tables_do_not_panic() {
    let mut state = 0x9e37_79b9_7f4a_7c15_u64;
    for length in 0..512 {
        let mut bytes = Vec::with_capacity(length);
        for _ in 0..length {
            bytes.push((next(&mut state) & 0xff) as u8);
        }
        let _ = TwoDa::parse_binary(&bytes);
        let text = String::from_utf8_lossy(&bytes);
        let _ = TwoDa::parse(&text);
        let _ = TalkTable::parse(PathBuf::from("mutant.tlk"), bytes);
    }
}

#[test]
fn hostile_rim_headers_do_not_panic() {
    let mut state = 0x0123_4567_89ab_cdef_u64;
    let path = std::env::temp_dir().join(format!("jadeforge-rim-mut-{}", std::process::id()));
    for length in 0..256 {
        let mut bytes = Vec::with_capacity(length);
        for _ in 0..length {
            bytes.push((next(&mut state) & 0xff) as u8);
        }
        std::fs::write(&path, &bytes).expect("write mutant");
        let _ = RimArchive::open(&path);
    }
    let _ = std::fs::remove_file(path);
}
