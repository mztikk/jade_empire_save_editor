#![allow(clippy::expect_used)] // tests fail loudly; expects are assertions

//! Guards the Windows executable's linker subsystem.
//!
//! Windows links console-subsystem binaries by default, which makes Explorer open
//! a console window next to the editor. The PE header records the subsystem the
//! linker chose, so this test checks the executable that is actually shipped
//! instead of the source attribute that requests it.

#![cfg(windows)]

use std::fs;
use std::path::Path;

/// `IMAGE_SUBSYSTEM_WINDOWS_GUI` from the PE/COFF specification.
const IMAGE_SUBSYSTEM_WINDOWS_GUI: u16 = 2;

#[test]
fn the_gui_executable_is_a_windows_gui_subsystem_binary() {
    let path = Path::new(env!("CARGO_BIN_EXE_jadeforge"));
    let bytes = fs::read(path).expect("the GUI executable is readable");
    assert_eq!(
        pe_subsystem(&bytes),
        Some(IMAGE_SUBSYSTEM_WINDOWS_GUI),
        "{} must link with the Windows GUI subsystem",
        path.display()
    );
}

/// Reads the `Subsystem` field from a PE32 or PE32+ optional header.
fn pe_subsystem(bytes: &[u8]) -> Option<u16> {
    let pe_offset = usize::try_from(read_u32(bytes, 0x3c)?).ok()?;
    if !bytes.get(pe_offset..)?.starts_with(b"PE\0\0") {
        return None;
    }
    let optional_header = pe_offset.checked_add(24)?;
    match read_u16(bytes, optional_header)? {
        0x10b | 0x20b => read_u16(bytes, optional_header.checked_add(68)?),
        _ => None,
    }
}

fn read_u16(bytes: &[u8], offset: usize) -> Option<u16> {
    let end = offset.checked_add(2)?;
    Some(u16::from_le_bytes(bytes.get(offset..end)?.try_into().ok()?))
}

fn read_u32(bytes: &[u8], offset: usize) -> Option<u32> {
    let end = offset.checked_add(4)?;
    Some(u32::from_le_bytes(bytes.get(offset..end)?.try_into().ok()?))
}
