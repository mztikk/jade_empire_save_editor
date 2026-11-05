//! The native folder dialog.
//!
//! `JadeForge` asks the operating system for the save folder instead of shipping
//! its own browser: `rfd` opens the real GTK/portal dialog on Linux, the Win32
//! dialog on Windows and `NSOpenPanel` on macOS.
//!
//! The dialog is modal, so it blocks the window while it is open, which is what
//! users expect from a file dialog. A thread local test seam lets the headless
//! UI tests answer the dialog without opening a window.

use std::cell::RefCell;
use std::path::{Path, PathBuf};

/// What the test seam should answer instead of opening a dialog.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Answer {
    /// The user cancelled the dialog.
    Cancelled,
    /// The user picked this folder.
    Picked(PathBuf),
}

thread_local! {
    /// Answer the next dialog call instead of opening a native dialog.
    static NEXT_PICK: RefCell<Option<Answer>> = const { RefCell::new(None) };
    /// Answer the next confirmation instead of opening a native dialog.
    static NEXT_CONFIRM: RefCell<Option<bool>> = const { RefCell::new(None) };
}

/// Answers the next [`pick_folder`] call with `path`.
///
/// Test seam: the headless UI tests cannot open a portal or a Win32 dialog, so
/// they queue the answer they want. `None` simulates the user cancelling.
#[doc(hidden)]
pub fn set_next_pick_for_tests(path: Option<PathBuf>) {
    let answer = path.map_or(Answer::Cancelled, Answer::Picked);
    NEXT_PICK.with(|slot| *slot.borrow_mut() = Some(answer));
}

/// Opens the native folder dialog.
///
/// `start` is used as the initial directory when it exists. Returns the chosen
/// folder, or `None` when the user cancelled.
#[must_use]
pub fn pick_folder(start: Option<&Path>) -> Option<PathBuf> {
    if let Some(answer) = NEXT_PICK.with(|slot| slot.borrow_mut().take()) {
        return match answer {
            Answer::Cancelled => None,
            Answer::Picked(path) => Some(path),
        };
    }
    let mut dialog = rfd::FileDialog::new().set_title("Choose the Jade Empire save folder");
    if let Some(start) = start.filter(|path| path.is_dir()) {
        dialog = dialog.set_directory(start);
    }
    dialog.pick_folder()
}

/// Answers the next [`confirm`] call with `confirmed`.
///
/// Test seam: destructive actions must not open a native modal in the headless
/// suite.
#[doc(hidden)]
pub fn set_next_confirm_for_tests(confirmed: bool) {
    NEXT_CONFIRM.with(|slot| *slot.borrow_mut() = Some(confirmed));
}

/// Asks for confirmation of a destructive action.
///
/// Returns `true` only when the user explicitly confirms. The native Yes/No
/// dialog is modal; the headless suite replaces it through
/// [`set_next_confirm_for_tests`].
#[must_use]
pub fn confirm(title: &str, description: &str) -> bool {
    if let Some(confirmed) = NEXT_CONFIRM.with(|slot| slot.borrow_mut().take()) {
        return confirmed;
    }
    matches!(
        rfd::MessageDialog::new()
            .set_title(title)
            .set_description(description)
            .set_buttons(rfd::MessageButtons::YesNo)
            .set_level(rfd::MessageLevel::Warning)
            .show(),
        rfd::MessageDialogResult::Yes
    )
}

/// Folder the dialog should start in: the current one, else the home folder.
#[must_use]
pub fn start_directory(current: &str) -> Option<PathBuf> {
    let current = Path::new(current);
    if current.is_dir() {
        return Some(current.to_path_buf());
    }
    std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from)
        .filter(|path| path.is_dir())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn queued_answers_are_consumed_once() {
        set_next_pick_for_tests(Some(PathBuf::from("/tmp/saves")));
        assert_eq!(pick_folder(None), Some(PathBuf::from("/tmp/saves")));
        // The queue is empty now; the next call would open the real dialog, so
        // only assert that the seam was consumed.
        let consumed = NEXT_PICK.with(|slot| slot.borrow().is_none());
        assert!(consumed);
    }

    #[test]
    fn confirmation_answers_are_consumed_once() {
        set_next_confirm_for_tests(false);
        assert!(!confirm("Remove", "Remove the item?"));
        let consumed = NEXT_CONFIRM.with(|slot| slot.borrow().is_none());
        assert!(consumed);
    }
}
