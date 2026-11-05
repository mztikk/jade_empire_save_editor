//! # `JadeForge` session
//!
//! UI independent state and behaviour of the desktop editor: scanning a save
//! folder, holding the opened savegame, applying form edits and persisting the
//! small settings file.
//!
//! The Freya views in `jadeforge-gui` render this state; nothing here depends
//! on a windowing toolkit, so the behaviour can be tested without a display.

pub mod config;
pub mod session;

pub use session::{
    CharacterForm, GameStatus, Message, MessageKind, SaveEntry, Session, SessionError,
    default_save_dir,
};
