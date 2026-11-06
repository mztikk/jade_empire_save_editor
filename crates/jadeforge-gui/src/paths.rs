//! Opening folders in the desktop file manager.
//!
//! A path shown in the interface is only useful if the user can reach it. The
//! editor has no file browser of its own, so it asks the desktop to open the
//! containing folder. The launcher runs detached: some file managers keep
//! `xdg-open` alive for as long as their window exists, and waiting for that
//! would freeze the editor.
//!
//! The headless UI tests cannot launch a file manager on CI, so a thread-local
//! seam records which folder a click would have opened.

use std::cell::RefCell;
use std::path::{Path, PathBuf};

thread_local! {
    /// When armed, the next open call is recorded instead of launching a file
    /// manager. The recorded path is what the headless test asserts on.
    static CAPTURE: RefCell<Option<PathBuf>> = const { RefCell::new(None) };
    static ARMED: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

/// Opens `folder` in the system file manager.
///
/// # Errors
///
/// Returns the launcher error when no desktop opener is available.
pub fn open_folder(folder: &Path) -> std::io::Result<()> {
    if ARMED.with(|armed| armed.replace(false)) {
        CAPTURE.with(|slot| *slot.borrow_mut() = Some(folder.to_path_buf()));
        return Ok(());
    }
    open::that_detached(folder)
}

/// Records the next [`open_folder`] call instead of opening a window.
#[doc(hidden)]
pub fn capture_next_open_for_tests() {
    ARMED.with(|armed| armed.set(true));
    CAPTURE.with(|slot| *slot.borrow_mut() = None);
}

/// Folder recorded by the last captured open, if any.
#[doc(hidden)]
#[must_use]
pub fn last_opened_for_tests() -> Option<PathBuf> {
    CAPTURE.with(|slot| slot.borrow().clone())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_test_seam_records_the_folder_without_launching_it() {
        capture_next_open_for_tests();
        open_folder(Path::new("/games/Jade Empire/Save")).expect("the capture never fails");
        assert_eq!(
            last_opened_for_tests(),
            Some(PathBuf::from("/games/Jade Empire/Save"))
        );
    }
}
