//! Clipboard writes with a deterministic headless test seam.
//!
//! The headless suite runs on CI machines without a desktop clipboard. A test
//! that clicks a copy action would therefore see the "clipboard unavailable"
//! error instead of the success path. The seam records the text instead.

use std::cell::{Cell, RefCell};

use freya::prelude::Clipboard;

thread_local! {
    static ARMED: Cell<bool> = const { Cell::new(false) };
    static CAPTURED: RefCell<Option<String>> = const { RefCell::new(None) };
}

/// Copies text to the system clipboard.
///
/// Returns `false` when the desktop clipboard is unavailable. A test that
/// armed [`capture_next_copy_for_tests`] gets a recorded success instead.
#[must_use]
pub fn copy(text: String) -> bool {
    if ARMED.with(|armed| armed.replace(false)) {
        CAPTURED.with(|slot| *slot.borrow_mut() = Some(text));
        return true;
    }
    Clipboard::set(text).is_ok()
}

/// Records the next [`copy`] call instead of touching the desktop clipboard.
#[doc(hidden)]
pub fn capture_next_copy_for_tests() {
    ARMED.with(|armed| armed.set(true));
    CAPTURED.with(|slot| *slot.borrow_mut() = None);
}

/// Text recorded by the last captured copy.
#[doc(hidden)]
#[must_use]
pub fn last_copied_for_tests() -> Option<String> {
    CAPTURED.with(|slot| slot.borrow().clone())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_test_seam_records_the_text_without_the_desktop_clipboard() {
        capture_next_copy_for_tests();
        assert!(copy("/games/Jade Empire/Save/save.sac".to_owned()));
        assert_eq!(
            last_copied_for_tests().as_deref(),
            Some("/games/Jade Empire/Save/save.sac")
        );
        assert!(!ARMED.with(Cell::get));
    }
}
