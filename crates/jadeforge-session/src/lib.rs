//! # `JadeForge` session
//!
//! UI independent state and behaviour of the desktop editor: scanning a save
//! folder, holding the opened savegame, applying form edits and persisting the
//! small settings file.
//!
//! The Freya views in `jadeforge-gui` render this state; nothing here depends
//! on a windowing toolkit, so the behaviour can be tested without a display.

pub mod config;
pub mod message;
pub mod preferences;
pub mod session;

pub use jadeforge_i18n::{
    Key, Locale, current, fill, has_id, install, is_complete, keys, locale_label, plural, t,
};
pub use preferences::{
    MAX_TEXT_SCALE, MIN_TEXT_SCALE, TEXT_SCALES, ThemeChoice, normalize_text_scale,
};
pub use session::{
    BackupRow, CharacterForm, GameStatus, Message, MessageKind, PendingChange, PendingChangeGroup,
    SaveEntry, Session, SessionError, default_save_dir,
};
