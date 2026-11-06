//! `JadeForge` desktop application.
//!
//! The binary in `src/main.rs` only starts the window; everything else lives
//! here so that the headless test suite can drive the application. The UI
//! independent state lives in `jadeforge-session` and is re-exported so the
//! views keep their `crate::session` paths.

pub mod app;
pub mod clipboard;
pub mod dialog;
pub mod form;
pub mod geometry;
pub mod i18n;
pub mod paths;
pub mod perf;
pub mod renderer;
#[cfg(target_os = "linux")]
pub mod system_theme;
pub mod theme;
pub mod views;
pub mod widgets;

pub use app::{app, app_with_directory};
pub use jadeforge_session::{
    CharacterForm, GameStatus, Locale, Message, MessageKind, SaveEntry, Session, ThemeChoice,
    default_save_dir, normalize_text_scale,
};
pub use jadeforge_session::{config, preferences, session};

/// The application id: the Wayland app id and the base name of the Linux
/// desktop entry (`jadeforge.desktop`) and its hicolor icon.
pub const APP_ID: &str = "jadeforge";

/// Application version, shown in the About surface and window help.
pub const APP_VERSION: &str = env!("CARGO_PKG_VERSION");

/// The application icon as a 256 px PNG, embedded so the window can set its
/// icon without reading a file at startup.
pub const ICON_PNG: &[u8] = include_bytes!("../assets/icon-256.png");
