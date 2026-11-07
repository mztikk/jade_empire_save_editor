//! # `JadeForge` game data
//!
//! Item, style and other display names live in the game files, not in the
//! savegame. This crate reads the small part of a Jade Empire installation that
//! `JadeForge` needs: the talk table (`dialog.tlk`), the text 2DA tables in
//! `override/` and the binary 2DA tables in the global RIM archives.
//!
//! Everything degrades gracefully: when the game files are missing the
//! savegame can still be edited, and the generated English name index supplies
//! item, style and player appearance names until the real installation is
//! available. Nothing here ever writes to the installation.

pub mod error;
pub mod gamedata;
pub mod names;
pub mod rim;
pub mod talktable;
pub mod twoda;

pub use error::{Error, Result};
pub use gamedata::{
    Advance, GameData, GemInfo, ItemInfo, PlayerAppearanceInfo, StyleChoice, StyleInfo, StyleKind,
    category_for_save_type, is_game_root, is_playable_item, pretty_effect_name,
    save_type_for_category, set_test_game_data_missing, set_test_game_root, style_kind,
    test_game_data_missing, test_game_root,
};
pub use names::{BundledItem, NameIndex, export_english};
pub use rim::RimArchive;
pub use talktable::TalkTable;
pub use twoda::TwoDa;
