//! `JadeForge` desktop application.
//!
//! The binary in `src/main.rs` only starts the window; everything else lives
//! here so that the headless test suite can drive the application.

pub mod app;
pub mod config;
pub mod dialog;
pub mod form;
pub mod session;
pub mod theme;
pub mod views;
pub mod widgets;

pub use app::{app, app_with_directory};
pub use session::{
    CharacterForm, GameStatus, Message, MessageKind, SaveEntry, Session, default_save_dir,
};
