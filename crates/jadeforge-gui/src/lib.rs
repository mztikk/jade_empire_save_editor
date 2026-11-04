//! `JadeForge` desktop application.
//!
//! The binary in `src/main.rs` only starts the window; everything else lives
//! here so that the headless test suite can drive the application. The UI
//! independent state lives in `jadeforge-session` and is re-exported so the
//! views keep their `crate::session` paths.

pub mod app;
pub mod dialog;
pub mod form;
pub mod theme;
pub mod views;
pub mod widgets;

pub use app::{app, app_with_directory};
pub use jadeforge_session::{
    CharacterForm, GameStatus, Message, MessageKind, SaveEntry, Session, default_save_dir,
};
pub use jadeforge_session::{config, session};
