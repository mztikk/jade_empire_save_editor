#![allow(clippy::expect_used)] // tests fail loudly; expects are assertions

//! Guards the bundled application icon assets.
//!
//! The window icon is embedded in the binary, so a missing or malformed asset
//! would otherwise only show up when someone opens the window. These tests parse
//! the PNG, ICO and ICNS containers directly; they need no display, no game
//! data and no SVG renderer.

use std::fs;
use std::path::{Path, PathBuf};

const PNG_SIGNATURE: [u8; 8] = [0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a];

/// Every rendered square icon and the size encoded in its file name.
const ICON_SIZES: [(u32, &str); 9] = [
    (16, "icon-16.png"),
    (24, "icon-24.png"),
    (32, "icon-32.png"),
    (48, "icon-48.png"),
    (64, "icon-64.png"),
    (128, "icon-128.png"),
    (256, "icon-256.png"),
    (512, "icon-512.png"),
    (1024, "icon-1024.png"),
];

fn asset(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("assets")
        .join(name)
}

fn read_asset(name: &str) -> Vec<u8> {
    fs::read(asset(name)).expect("bundled icon asset is readable")
}

/// Reads the dimensions from a PNG's IHDR chunk without decoding the image.
fn png_size(bytes: &[u8]) -> Option<(u32, u32)> {
    if bytes.len() < 24 || !bytes.starts_with(&PNG_SIGNATURE) {
        return None;
    }
    if bytes.get(12..16) != Some(&b"IHDR"[..]) {
        return None;
    }
    let width = u32::from_be_bytes(bytes.get(16..20)?.try_into().ok()?);
    let height = u32::from_be_bytes(bytes.get(20..24)?.try_into().ok()?);
    Some((width, height))
}

#[test]
fn embedded_window_icon_is_a_256_png() {
    assert_eq!(png_size(jadeforge_gui::ICON_PNG), Some((256, 256)));
}

#[test]
fn every_png_matches_the_size_in_its_name() {
    for (size, name) in ICON_SIZES {
        let bytes = read_asset(name);
        assert_eq!(png_size(&bytes), Some((size, size)), "{name}");
    }
}

#[test]
fn windows_ico_contains_every_size() {
    let bytes = read_asset("jadeforge.ico");
    assert!(bytes.starts_with(&[0, 0, 1, 0]), "ICO header");
    let count = usize::from(u16::from_le_bytes(
        bytes[4..6].try_into().expect("ICO entry count"),
    ));
    assert_eq!(count, 7);

    let mut sizes = Vec::new();
    for index in 0..count {
        let entry = 6 + index * 16;
        let width = u32::from(bytes[entry]);
        let height = u32::from(bytes[entry + 1]);
        let size = u32::from_le_bytes(
            bytes[entry + 8..entry + 12]
                .try_into()
                .expect("ICO image size"),
        );
        let offset = usize::try_from(u32::from_le_bytes(
            bytes[entry + 12..entry + 16]
                .try_into()
                .expect("ICO image offset"),
        ))
        .expect("ICO offset fits usize");
        assert!(size > 0);
        assert!(
            offset
                .checked_add(usize::try_from(size).expect("ICO size fits usize"))
                .is_some_and(|end| end <= bytes.len()),
            "ICO entry {index} points inside the file"
        );
        let reported = if width == 0 { 256 } else { width };
        assert_eq!(reported, if height == 0 { 256 } else { height });
        sizes.push(reported);
    }
    assert_eq!(sizes, vec![16, 24, 32, 48, 64, 128, 256]);
}

#[test]
fn macos_icns_is_a_valid_container() {
    let bytes = read_asset("jadeforge.icns");
    assert!(bytes.starts_with(b"icns"), "ICNS magic");
    let length = usize::try_from(u32::from_be_bytes(
        bytes[4..8].try_into().expect("ICNS length"),
    ))
    .expect("ICNS length fits usize");
    assert_eq!(length, bytes.len());

    let expected = [
        "icp4", "icp5", "icp6", "ic07", "ic08", "ic09", "ic10", "ic11", "ic12", "ic13", "ic14",
    ];
    let mut kinds = Vec::new();
    let mut position = 8;
    while position < bytes.len() {
        let kind = &bytes[position..position + 4];
        let element_length = usize::try_from(u32::from_be_bytes(
            bytes[position + 4..position + 8]
                .try_into()
                .expect("ICNS element length"),
        ))
        .expect("ICNS element length fits usize");
        assert!(element_length >= 8);
        assert!(position + element_length <= bytes.len());
        assert!(
            bytes[position + 8..position + element_length].starts_with(&PNG_SIGNATURE),
            "{} does not carry a PNG payload",
            String::from_utf8_lossy(kind)
        );
        kinds.push(String::from_utf8_lossy(kind).into_owned());
        position += element_length;
    }
    assert_eq!(kinds, expected);
}

#[test]
fn linux_entry_uses_the_same_app_id_and_icon_name() {
    let app_id = jadeforge_gui::APP_ID;
    assert_eq!(app_id, "jadeforge");
    let entry = include_str!("../assets/linux/jadeforge.desktop");
    let exec = format!("Exec={app_id}");
    let icon = format!("Icon={app_id}");
    let startup = format!("StartupWMClass={app_id}");
    for line in [
        "Type=Application",
        exec.as_str(),
        icon.as_str(),
        startup.as_str(),
    ] {
        assert!(
            entry.lines().any(|candidate| candidate.trim() == line),
            "desktop entry is missing {line}"
        );
    }
}
