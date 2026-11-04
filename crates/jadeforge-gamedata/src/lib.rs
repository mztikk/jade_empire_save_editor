//! # `JadeForge` game data
//!
//! Item, style and other display names live in the game files, not in the
//! savegame. This crate reads the small part of a Jade Empire installation that
//! `JadeForge` needs: the talk table (`dialog.tlk`) and the text 2DA tables in
//! `override/`.
//!
//! Everything degrades gracefully: when the game files are missing the
//! savegame can still be edited, the UI just shows numeric ids instead of
//! names. Nothing here ever writes to the installation.

pub mod error;
pub mod gamedata;
pub mod talktable;
pub mod twoda;

pub use error::{Error, Result};
pub use gamedata::{GameData, ItemInfo, StyleInfo, is_game_root, save_type_for_category};
pub use talktable::TalkTable;
pub use twoda::TwoDa;
