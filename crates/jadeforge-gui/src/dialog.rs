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

/// Result of the native folder dialog.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PickOutcome {
    /// The user chose this existing folder.
    Picked(PathBuf),
    /// The user closed the dialog without choosing a folder.
    ///
    /// This is distinct from an unreadable or empty folder: the caller reports
    /// the cancellation and leaves the current save folder untouched.
    Cancelled,
}

/// What the user chose in the unsaved-changes close dialog.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CloseChoice {
    /// Write the applied edits, then close.
    Save,
    /// Close without writing, discarding the in-memory edits.
    Discard,
    /// Keep the window open and keep editing.
    Cancel,
}

thread_local! {
    /// Answer the next dialog call instead of opening a native dialog.
    static NEXT_PICK: RefCell<Option<Answer>> = const { RefCell::new(None) };
    /// Answer the next confirmation instead of opening a native dialog.
    static NEXT_CONFIRM: RefCell<Option<bool>> = const { RefCell::new(None) };
    /// Answer the next close dialog instead of opening a native dialog.
    static NEXT_CLOSE: RefCell<Option<CloseChoice>> = const { RefCell::new(None) };
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
/// `start` is used as the initial directory when it exists. A closed dialog is
/// reported as [`PickOutcome::Cancelled`] so the caller can say that nothing
/// changed instead of treating it like an empty folder. The `rfd` API folds a
/// backend failure into the same `None`, so callers still validate a picked
/// path before scanning it.
#[must_use]
pub fn pick_folder(start: Option<&Path>) -> PickOutcome {
    if let Some(answer) = NEXT_PICK.with(|slot| slot.borrow_mut().take()) {
        return match answer {
            Answer::Cancelled => PickOutcome::Cancelled,
            Answer::Picked(path) => PickOutcome::Picked(path),
        };
    }
    let mut dialog = rfd::FileDialog::new().set_title("Choose the Jade Empire save folder");
    if let Some(start) = start.filter(|path| path.is_dir()) {
        dialog = dialog.set_directory(start);
    }
    dialog
        .pick_folder()
        .map_or(PickOutcome::Cancelled, PickOutcome::Picked)
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

/// Answers the next [`ask_close_choice`] call with `choice`.
///
/// Test seam: the window close hook is only exercised by a real window, so the
/// headless suite can at least verify the choice plumbing without a modal.
#[doc(hidden)]
pub fn set_next_close_choice_for_tests(choice: CloseChoice) {
    NEXT_CLOSE.with(|slot| *slot.borrow_mut() = Some(choice));
}

/// Asks what to do with unsaved changes before closing the window.
///
/// The native dialog uses explicit `Save`, `Discard` and `Cancel` labels
/// instead of Yes/No so the destructive choice cannot be guessed from a
/// generic button. On Windows the custom labels need the
/// `common-controls-v6` feature and the application manifest.
#[must_use]
pub fn ask_close_choice() -> CloseChoice {
    if let Some(choice) = NEXT_CLOSE.with(|slot| slot.borrow_mut().take()) {
        return choice;
    }
    let result = rfd::MessageDialog::new()
        .set_title("Unsaved changes")
        .set_description(
            "This savegame has changes that are only in memory. Save them before closing, discard them, or cancel and keep editing.",
        )
        .set_buttons(rfd::MessageButtons::YesNoCancelCustom(
            "Save".to_owned(),
            "Discard".to_owned(),
            "Cancel".to_owned(),
        ))
        .set_level(rfd::MessageLevel::Warning)
        .show();
    close_choice(&result)
}

/// Maps the native dialog result onto the explicit close choice.
///
/// Every rfd backend returns `Custom(label)` for a custom button rather than
/// Yes/No/Cancel (the GTK, macOS, TaskDialog and zenity paths all do), so the
/// labels are the contract. Yes/No remain accepted for the non-custom fallback
/// on a Windows build without the ComCtl32 v6 manifest.
fn close_choice(result: &rfd::MessageDialogResult) -> CloseChoice {
    match result {
        rfd::MessageDialogResult::Yes => CloseChoice::Save,
        rfd::MessageDialogResult::No => CloseChoice::Discard,
        rfd::MessageDialogResult::Custom(label) if label == "Save" => CloseChoice::Save,
        rfd::MessageDialogResult::Custom(label) if label == "Discard" => CloseChoice::Discard,
        rfd::MessageDialogResult::Ok
        | rfd::MessageDialogResult::Cancel
        | rfd::MessageDialogResult::Custom(_) => CloseChoice::Cancel,
    }
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
        assert_eq!(
            pick_folder(None),
            PickOutcome::Picked(PathBuf::from("/tmp/saves"))
        );
        // The queue is empty now; the next call would open the real dialog, so
        // only assert that the seam was consumed.
        let consumed = NEXT_PICK.with(|slot| slot.borrow().is_none());
        assert!(consumed);
    }

    #[test]
    fn a_cancelled_picker_is_reported_as_a_cancellation() {
        set_next_pick_for_tests(None);
        assert_eq!(pick_folder(None), PickOutcome::Cancelled);
    }

    #[test]
    fn confirmation_answers_are_consumed_once() {
        set_next_confirm_for_tests(false);
        assert!(!confirm("Remove", "Remove the item?"));
        let consumed = NEXT_CONFIRM.with(|slot| slot.borrow().is_none());
        assert!(consumed);
    }

    #[test]
    fn close_dialog_answers_are_consumed_once() {
        set_next_close_choice_for_tests(CloseChoice::Discard);
        assert_eq!(ask_close_choice(), CloseChoice::Discard);
        let consumed = NEXT_CLOSE.with(|slot| slot.borrow().is_none());
        assert!(consumed);
    }

    #[test]
    fn the_custom_buttons_map_to_explicit_close_choices() {
        use rfd::MessageDialogResult;
        assert_eq!(close_choice(&MessageDialogResult::Yes), CloseChoice::Save);
        assert_eq!(close_choice(&MessageDialogResult::No), CloseChoice::Discard);
        assert_eq!(
            close_choice(&MessageDialogResult::Cancel),
            CloseChoice::Cancel
        );
        assert_eq!(close_choice(&MessageDialogResult::Ok), CloseChoice::Cancel);
        assert_eq!(
            close_choice(&MessageDialogResult::Custom("Save".to_owned())),
            CloseChoice::Save
        );
        assert_eq!(
            close_choice(&MessageDialogResult::Custom("Discard".to_owned())),
            CloseChoice::Discard
        );
        assert_eq!(
            close_choice(&MessageDialogResult::Custom("Cancel".to_owned())),
            CloseChoice::Cancel
        );
        assert_eq!(
            close_choice(&MessageDialogResult::Custom("other".to_owned())),
            CloseChoice::Cancel
        );
    }
}
