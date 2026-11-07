//! Data rows shown by the session and the GUI.

use std::path::PathBuf;

use jadeforge_core::{GemLocation, PlayerAppearance};
use jadeforge_i18n::{Key, keys};

use crate::message;
use crate::session::format;

/// One selectable player appearance.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppearanceOption {
    /// Player appearance and its `appearance.2da` row.
    pub appearance: PlayerAppearance,
    /// Localized name from the game data, or the English fallback.
    pub label: String,
}

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
    /// Time played in microseconds.
    pub time_played: u64,
    /// Last modification time of `save.sac`.
    pub modified: Option<std::time::SystemTime>,
    /// Set when the savegame could not be read while scanning.
    pub error: Option<String>,
}

impl SaveEntry {
    /// One line summary of the savegame: level, time played, character and the
    /// modification time, in that order.
    ///
    /// The game names the folder, not the character, so the folder label stays
    /// the row title; the character name and when the save was last written are
    /// what the README promises and were missing from the row.
    #[must_use]
    pub fn summary(&self) -> String {
        if self.error.is_some() {
            return message::tr(keys::SESSION_CANNOT_BE_READ).to_owned();
        }
        let level = self
            .level
            .map_or_else(|| "?".to_owned(), |level| level.to_string());
        let mut parts = vec![message::fill(
            keys::SESSION_ROW_LEVEL_TIME,
            &[
                ("level", &level),
                ("time", &format::duration(self.time_played)),
            ],
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
    /// Stable catalog key used in notices.
    #[must_use]
    pub fn label(self) -> Key {
        match self {
            Self::Character => keys::NAV_CHARACTER,
            Self::Items => keys::LABEL_INVENTORY,
            Self::Styles => keys::COUNT_STYLE_OTHER,
            Self::Companions => keys::NAV_COMPANIONS,
            Self::Gems => keys::COUNT_GEM_OTHER,
            Self::Amulet => keys::LABEL_AMULET,
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

/// One field or row that an applied edit changed.
///
/// The pending-changes screen groups these by action so the user can see the
/// exact fields and rows a save will write, not just "unsaved changes".
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PendingChange {
    /// Dotted field path or row description.
    pub field: String,
    /// Value before the edit.
    pub before: String,
    /// Value after the edit.
    pub after: String,
}

impl PendingChange {
    /// Builds a change row.
    #[must_use]
    pub fn new(
        field: impl Into<String>,
        before: impl Into<String>,
        after: impl Into<String>,
    ) -> Self {
        Self {
            field: field.into(),
            before: before.into(),
            after: after.into(),
        }
    }
}

/// All pending changes made by one user action.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PendingChangeGroup {
    /// Short action label such as `Inventory` or `Upgrade`.
    pub label: String,
    /// Fields and rows the action changed.
    pub changes: Vec<PendingChange>,
}

/// One backup listed by the backup screen.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BackupRow {
    /// Path of the backup file.
    pub path: PathBuf,
    /// The savegame the backup belongs to.
    pub original: PathBuf,
    /// Creation instant parsed from the backup name.
    pub created: std::time::SystemTime,
    /// Size in bytes.
    pub size: u64,
}

impl BackupRow {
    /// UTC timestamp shown in the backup list.
    #[must_use]
    pub fn timestamp(&self) -> String {
        format::timestamp(self.created)
    }
}
