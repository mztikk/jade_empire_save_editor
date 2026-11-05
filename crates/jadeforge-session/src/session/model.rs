//! Data rows shown by the session and the GUI.

use std::path::PathBuf;

use jadeforge_core::GemLocation;

use crate::session::format;

/// One entry of the savegame list.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SaveEntry {
    /// Folder that contains `save.sac`.
    pub path: PathBuf,
    /// Name shown in the list.
    pub label: String,
    /// Character name, when the savegame could be read.
    pub character: Option<String>,
    /// Character level.
    pub level: Option<u8>,
    /// Play time in microseconds.
    pub time_played: u64,
    /// Set when the savegame could not be read while scanning.
    pub error: Option<String>,
}

impl SaveEntry {
    /// One line summary of the savegame.
    #[must_use]
    pub fn summary(&self) -> String {
        if self.error.is_some() {
            return "Cannot be read".to_owned();
        }
        let level = self
            .level
            .map_or_else(|| "?".to_owned(), |level| level.to_string());
        format!("Level {level} · {}", format::duration(self.time_played))
    }
}

/// Severity of the status message shown in the header.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MessageKind {
    /// Neutral information.
    Info,
    /// The last action succeeded.
    Success,
    /// The last action failed.
    Error,
}

/// A status message.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Message {
    /// Severity.
    pub kind: MessageKind,
    /// Text.
    pub text: String,
}

impl Message {
    /// Neutral message.
    #[must_use]
    pub fn info(text: impl Into<String>) -> Self {
        Self {
            kind: MessageKind::Info,
            text: text.into(),
        }
    }

    /// Success message.
    #[must_use]
    pub fn success(text: impl Into<String>) -> Self {
        Self {
            kind: MessageKind::Success,
            text: text.into(),
        }
    }

    /// Error message.
    #[must_use]
    pub fn error(text: impl Into<String>) -> Self {
        Self {
            kind: MessageKind::Error,
            text: text.into(),
        }
    }
}

/// State of the optional game data.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GameStatus {
    /// Game files are available and resolved.
    Loaded {
        /// Number of named items.
        items: usize,
        /// Number of named styles.
        styles: usize,
        /// Directory the data was read from.
        root: PathBuf,
    },
    /// No game installation was found next to the savegames.
    Missing,
    /// A game installation was found but could not be read.
    Error(String),
}

/// An item stack together with its display data.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ItemRow {
    /// Index of the struct that stores the item.
    pub struct_index: usize,
    /// Item template id.
    pub item_id: u16,
    /// Stack size.
    pub count: u8,
    /// Item category as stored in the savegame.
    pub kind: u8,
    /// Display name.
    pub name: String,
    /// Category label, for example `weapons` or `gems`.
    pub category: Option<String>,
    /// Description from the game data.
    pub description: Option<String>,
}

/// One purchasable slot of a style's level-up table.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StyleUpgrade {
    /// 1-based slot in the style's level-up table; the toggle sets bit `slot - 1`.
    pub slot: u16,
    /// Row in `StyleAdvance.2da` the slot grants.
    pub advance: u16,
    /// Localized family name.
    pub family: String,
    /// Tier inside the family (1 based).
    pub tier: u8,
    /// Localized name.
    pub name: String,
    /// Short effect description, for example `Damage +25%`.
    pub effect: String,
    /// Style points the upgrade costs.
    pub cost: u8,
    /// Whether the player already owns it.
    pub taken: bool,
}

/// A fighting style together with its display data.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StyleRow {
    /// Index of the struct that stores the style.
    pub struct_index: usize,
    /// Row in `stylesuperlist.2da`.
    pub style_id: u16,
    /// Current level.
    pub level: u8,
    /// Display name.
    pub name: String,
    /// Internal template label, for example `Martial_Legendary_Strike`.
    pub label: String,
    /// Kind of style, for example `magic` or `support`.
    pub kind: &'static str,
    /// Description from the game data.
    pub description: Option<String>,
    /// Raw `StyleLevelInfo` upgrade mask.
    pub mask: u64,
    /// Upgrade choices that can be toggled for this style.
    pub upgrades: Vec<StyleUpgrade>,
}

/// A style the player could learn.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StyleSuggestion {
    /// Row in `stylesuperlist.2da`.
    pub style_id: u16,
    /// Localized name.
    pub name: String,
    /// Internal template label.
    pub label: String,
    /// Kind of style.
    pub kind: &'static str,
    /// Description from the game data.
    pub description: Option<String>,
}

/// A gem together with display data.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GemRow {
    /// Index of the struct that stores the gem.
    pub struct_index: usize,
    /// Display name from the item table.
    pub name: String,
    /// Readable effect lines, for example `+2 Body`.
    pub effects: Vec<String>,
    /// Empty amulet socket: no gem in it.
    pub empty: bool,
    /// Whether the gem is socketed or carried.
    pub location: GemLocation,
}

/// A companion together with display data.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HenchRow {
    /// Index of the struct that stores the companion.
    pub struct_index: usize,
    /// Creature template.
    pub resref: String,
    /// Display name.
    pub name: String,
    /// Companion level.
    pub level: u8,
    /// Whether the companion is selectable.
    pub available: bool,
}

/// A search hit for the item picker.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ItemSuggestion {
    /// Row in `items.2da`, usable as item id.
    pub item_id: u16,
    /// Display name.
    pub name: String,
    /// Category label.
    pub category: String,
    /// Value for the `Type` field of a new item struct.
    pub save_type: u8,
    /// Description from the game data.
    pub description: Option<String>,
}

/// A decoded savegame screenshot.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Preview {
    /// Width in pixels.
    pub width: u32,
    /// Height in pixels.
    pub height: u32,
    /// RGBA pixels.
    pub rgba: Vec<u8>,
    /// Bumped every time a new preview is loaded.
    pub generation: u64,
}
