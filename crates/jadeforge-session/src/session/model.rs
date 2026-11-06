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
    /// Last modification time of `save.sac`.
    pub modified: Option<std::time::SystemTime>,
    /// Set when the savegame could not be read while scanning.
    pub error: Option<String>,
}

impl SaveEntry {
    /// One line summary of the savegame: level, play time, character and the
    /// modification time, in that order.
    ///
    /// The game names the folder, not the character, so the folder label stays
    /// the row title; the character name and when the save was last written are
    /// what the README promises and were missing from the row.
    #[must_use]
    pub fn summary(&self) -> String {
        if self.error.is_some() {
            return "Cannot be read".to_owned();
        }
        let level = self
            .level
            .map_or_else(|| "?".to_owned(), |level| level.to_string());
        let mut parts = vec![format!(
            "Level {level} · {}",
            format::duration(self.time_played)
        )];
        if let Some(character) = self.character.as_deref().filter(|name| !name.is_empty()) {
            parts.push(character.to_owned());
        }
        if let Some(modified) = self.modified {
            parts.push(format::timestamp(modified));
        }
        parts.join(" · ")
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

/// A savegame section the reader recovered from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Section {
    /// Character sheet.
    Character,
    /// Inventory.
    Items,
    /// Fighting styles.
    Styles,
    /// Companions.
    Companions,
    /// Gems.
    Gems,
    /// The amulet.
    Amulet,
}

impl Section {
    /// Human-readable name used in notices.
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::Character => "Character",
            Self::Items => "Inventory",
            Self::Styles => "Styles",
            Self::Companions => "Companions",
            Self::Gems => "Gems",
            Self::Amulet => "Amulet",
        }
    }
}

/// One savegame section that could not be read.
///
/// Kept apart from the status message so a view can say that its section is
/// unreadable instead of rendering the same "empty" hint as a genuine empty
/// list.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SectionError {
    /// Which section failed.
    pub section: Section,
    /// Reader error.
    pub message: String,
}

/// State of the optional game data.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GameStatus {
    /// The installation is being read on a worker; the UI shows a pending
    /// state instead of claiming the game data is missing.
    Loading,
    /// Game files are available and resolved.
    Loaded {
        /// Number of named items.
        items: usize,
        /// Number of named styles.
        styles: usize,
        /// Directory the data was read from.
        root: PathBuf,
    },
    /// No game installation was found; the bundled English index supplies
    /// names, but descriptions and upgrade tables are unavailable.
    NamesOnly {
        /// Number of named items.
        items: usize,
        /// Number of named styles.
        styles: usize,
    },
    /// No game installation and no bundled names were found.
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
    /// Icon resource name stored in the savegame.
    pub icon: Option<String>,
    /// Weapon resource name stored in the savegame.
    pub weapon: Option<String>,
    /// Number of weapons the style carries (`WeaponCount`).
    pub weapon_count: u8,
    /// Whole seconds the player spent in this style (`TimeSpentInStyle`).
    pub time_spent: u32,
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

/// A predefined gem that can be added.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GemSuggestion {
    /// Item id used in the savegame.
    pub item_id: u16,
    /// Display name.
    pub name: String,
    /// Readable effect lines, for example `+2 Body`.
    pub effects: Vec<String>,
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
