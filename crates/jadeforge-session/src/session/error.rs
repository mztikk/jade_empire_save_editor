//! Typed errors for UI-facing session actions.

use std::fmt;

/// A session action that could not be completed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SessionError {
    /// No savegame is open.
    NoSavegame,
    /// The savegame has no unsaved changes.
    NothingToSave,
    /// The savegame could not be reloaded from disk.
    CannotReload,
    /// A character form value was rejected.
    InvalidForm(String),
    /// The savegame could not be written.
    Save(String),
    /// An item could not be added.
    Item(String),
}

impl SessionError {
    /// Human-readable message, also used for the status line.
    #[must_use]
    pub fn message(&self) -> String {
        self.to_string()
    }

    /// True when the rendered message contains `needle`.
    ///
    /// This keeps the existing diagnostics tests readable while the public type
    /// is structured.
    #[must_use]
    pub fn contains(&self, needle: &str) -> bool {
        self.message().contains(needle)
    }
}

impl fmt::Display for SessionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoSavegame => write!(formatter, "no savegame is open"),
            Self::NothingToSave => write!(formatter, "nothing to save"),
            Self::CannotReload => write!(formatter, "cannot reload the savegame"),
            Self::InvalidForm(message) | Self::Save(message) | Self::Item(message) => {
                write!(formatter, "{message}")
            }
        }
    }
}

impl std::error::Error for SessionError {}
