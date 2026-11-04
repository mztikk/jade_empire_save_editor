//! UI independent session state.
//!
//! Everything the desktop application does with a savegame goes through this
//! module. It keeps the Freya views thin and makes the behaviour testable
//! without a window.

use std::path::{Path, PathBuf};
use std::rc::Rc;

use jadeforge_core::{
    AmuletInfo, Character, GemLocation, Henchman, Item, SaveGame, SaveImage, SaveOptions, Style,
};
use jadeforge_gamedata::GameData;

/// Folder to start with: the environment override, the last folder used, or
/// nothing at all.
///
/// `JadeForge` deliberately ships no guess at where a game installation lives:
/// GOG, Steam, Heroic, Wine and manual installs all put it somewhere else, and
/// a wrong default is worse than an empty field.
#[must_use]
pub fn default_save_dir() -> PathBuf {
    if let Some(directory) = std::env::var_os("JADEFORGE_SAVE_DIR") {
        return PathBuf::from(directory);
    }
    crate::config::Config::load().save_dir.unwrap_or_default()
}

/// Formatting helpers shared by the views and the list entries.
pub mod format {
    /// Formats a duration given in microseconds.
    #[must_use]
    pub fn duration(microseconds: u64) -> String {
        let seconds = microseconds / 1_000_000;
        let hours = seconds / 3600;
        let minutes = (seconds % 3600) / 60;
        if hours > 0 {
            format!("{hours}h {minutes:02}m")
        } else {
            format!("{minutes}m")
        }
    }

    /// Formats a float without trailing zeros.
    #[must_use]
    pub fn number(value: f32) -> String {
        if (value - value.round()).abs() < f32::EPSILON {
            format!("{}", value.round() as i64)
        } else {
            format!("{value:.1}")
        }
    }

    /// Names the alignment the way the game does.
    ///
    /// The engine starts a character at [`jadeforge_core::DEFAULT_ALIGNMENT`]
    /// and only writes the field once the value moves away from it.
    #[must_use]
    pub fn alignment(value: i32) -> &'static str {
        let reference = jadeforge_core::DEFAULT_ALIGNMENT as i32;
        match value.cmp(&reference) {
            std::cmp::Ordering::Greater => "Way of the Open Palm",
            std::cmp::Ordering::Less => "Way of the Closed Fist",
            std::cmp::Ordering::Equal => "Balanced",
        }
    }

    /// Formats a byte count as `4.6 MB`.
    #[must_use]
    pub fn bytes(value: u64) -> String {
        let value = value as f64;
        if value >= 1024.0 * 1024.0 {
            format!("{:.1} MB", value / (1024.0 * 1024.0))
        } else if value >= 1024.0 {
            format!("{:.1} kB", value / 1024.0)
        } else {
            format!("{value:.0} B")
        }
    }

    /// Formats a large number with thin separators.
    #[must_use]
    pub fn amount(value: u32) -> String {
        let digits = value.to_string();
        let mut out = String::new();
        for (index, character) in digits.chars().enumerate() {
            if index > 0 && (digits.len() - index) % 3 == 0 {
                out.push('\u{202f}');
            }
            out.push(character);
        }
        out
    }
}

/// Order in which the savegame list is shown.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SaveSort {
    /// Folder name, which is how the game names savegames.
    Name,
    /// Character level.
    Level,
    /// Time played.
    Played,
}

impl SaveSort {
    /// Short label for the sort buttons.
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::Name => "Name",
            Self::Level => "Level",
            Self::Played => "Played",
        }
    }
}

/// Compares two strings, treating runs of digits as numbers.
///
/// Savegame folders are named `Auto 10`, `Auto 104`, ...; plain string order
/// puts `Auto 104` before `Auto 2`, which is not what the game shows.
#[must_use]
pub fn natural_order(left: &str, right: &str) -> std::cmp::Ordering {
    use std::cmp::Ordering;

    let left = left.chars().collect::<Vec<_>>();
    let right = right.chars().collect::<Vec<_>>();
    let mut left_index = 0;
    let mut right_index = 0;
    while left_index < left.len() && right_index < right.len() {
        let left_digit = left[left_index].is_ascii_digit();
        let right_digit = right[right_index].is_ascii_digit();
        if left_digit && right_digit {
            let left_end = run_end(&left, left_index);
            let right_end = run_end(&right, right_index);
            let left_digits: String = left[left_index..left_end].iter().collect();
            let right_digits: String = right[right_index..right_end].iter().collect();
            let left_trimmed = left_digits.trim_start_matches('0');
            let right_trimmed = right_digits.trim_start_matches('0');
            let order = left_trimmed
                .len()
                .cmp(&right_trimmed.len())
                .then_with(|| left_trimmed.cmp(right_trimmed));
            if order != Ordering::Equal {
                return order;
            }
            left_index = left_end;
            right_index = right_end;
            continue;
        }
        let order = left[left_index]
            .to_lowercase()
            .cmp(right[right_index].to_lowercase());
        if order != Ordering::Equal {
            return order;
        }
        left_index += 1;
        right_index += 1;
    }
    (left.len() - left_index).cmp(&(right.len() - right_index))
}

fn run_end(text: &[char], start: usize) -> usize {
    let mut end = start;
    while end < text.len() && text[end].is_ascii_digit() {
        end += 1;
    }
    end
}

/// Filters and sorts the savegame list.
///
/// Returns the indices of the entries to show, so the caller keeps using the
/// index into [`Session::entries`] when opening one.
#[must_use]
pub fn visible_entries(
    entries: &[SaveEntry],
    query: &str,
    sort: SaveSort,
    descending: bool,
) -> Vec<usize> {
    let needle = query.trim().to_lowercase();
    let mut indices: Vec<usize> = entries
        .iter()
        .enumerate()
        .filter(|(_, entry)| {
            needle.is_empty()
                || entry.label.to_lowercase().contains(&needle)
                || entry
                    .character
                    .as_deref()
                    .is_some_and(|name| name.to_lowercase().contains(&needle))
        })
        .map(|(index, _)| index)
        .collect();
    indices.sort_by(|left, right| {
        let left = &entries[*left];
        let right = &entries[*right];
        let order = match sort {
            SaveSort::Name => natural_order(&left.label, &right.label),
            SaveSort::Level => left
                .level
                .unwrap_or(0)
                .cmp(&right.level.unwrap_or(0))
                .then_with(|| natural_order(&left.label, &right.label)),
            SaveSort::Played => left
                .time_played
                .cmp(&right.time_played)
                .then_with(|| natural_order(&left.label, &right.label)),
        };
        if descending { order.reverse() } else { order }
    });
    indices
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

/// The editable values of the character form.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CharacterForm {
    /// First name.
    pub name: String,
    /// Level.
    pub level: String,
    /// Experience points.
    pub experience: String,
    /// Gold.
    pub gold: String,
    /// Unspent attribute points.
    pub attribute_points: String,
    /// Current health.
    pub health: String,
    /// Maximum health.
    pub health_max: String,
    /// Current chi.
    pub chi: String,
    /// Maximum chi.
    pub chi_max: String,
    /// Current focus.
    pub focus: String,
    /// Maximum focus.
    pub focus_max: String,
    /// Body.
    pub body: String,
    /// Mind.
    pub mind: String,
    /// Spirit.
    pub spirit: String,
    /// Charm.
    pub charm: String,
    /// Intuition.
    pub intuition: String,
    /// Intimidate.
    pub intimidate: String,
    /// Alignment.
    pub alignment: String,
    /// Appearance index.
    pub appearance: String,
    /// Chapter.
    pub chapter: String,
    /// Kills.
    pub kills: String,
}

impl CharacterForm {
    /// Fills the form from a character.
    #[must_use]
    pub fn from_character(character: &Character) -> Self {
        Self {
            name: character.name.clone(),
            level: character.level.to_string(),
            experience: character.experience.to_string(),
            gold: character.gold.unwrap_or(0).to_string(),
            attribute_points: character.attribute_points.to_string(),
            health: format::number(character.health),
            health_max: character.health_max.to_string(),
            chi: format::number(character.chi),
            chi_max: character.chi_max.to_string(),
            focus: format::number(character.focus),
            focus_max: format::number(character.focus_max),
            body: character.body.to_string(),
            mind: character.mind.to_string(),
            spirit: character.spirit.to_string(),
            charm: character.charm.to_string(),
            intuition: character.intuition.to_string(),
            intimidate: character.intimidate.to_string(),
            alignment: character.alignment.to_string(),
            appearance: character.appearance.to_string(),
            chapter: character.chapter.to_string(),
            kills: character.kills.to_string(),
        }
    }

    /// Builds a character by applying the form on top of a base character.
    pub fn to_character(&self, base: &Character) -> Result<Character, String> {
        let mut character = base.clone();
        character.name.clone_from(&self.name);
        character.level = parse::<u8>("level", &self.level)?;
        character.experience = parse::<u32>("experience", &self.experience)?;
        character.gold = Some(parse::<u32>("gold", &self.gold)?);
        character.attribute_points = parse::<u32>("attribute points", &self.attribute_points)?;
        character.health = parse_float("health", &self.health)?;
        character.health_max = parse::<u32>("maximum health", &self.health_max)?;
        character.chi = parse_float("chi", &self.chi)?;
        character.chi_max = parse::<u32>("maximum chi", &self.chi_max)?;
        character.focus = parse_float("focus", &self.focus)?;
        character.focus_max = parse_float("maximum focus", &self.focus_max)?;
        character.body = parse::<u32>("body", &self.body)?;
        character.mind = parse::<u32>("mind", &self.mind)?;
        character.spirit = parse::<u32>("spirit", &self.spirit)?;
        character.charm = parse::<i32>("charm", &self.charm)?;
        character.intuition = parse::<i32>("intuition", &self.intuition)?;
        character.intimidate = parse::<i32>("intimidate", &self.intimidate)?;
        character.alignment = parse::<i32>("alignment", &self.alignment)?;
        character.appearance = parse::<i32>("appearance", &self.appearance)?;
        character.chapter = parse::<u32>("chapter", &self.chapter)?;
        character.kills = parse::<u32>("kills", &self.kills)?;
        Ok(character)
    }
}

fn parse<T: std::str::FromStr>(what: &str, raw: &str) -> Result<T, String> {
    raw.trim()
        .parse()
        .map_err(|_| format!("{what}: {:?} is not a number", raw.trim()))
}

fn parse_float(what: &str, raw: &str) -> Result<f32, String> {
    let value: f32 = raw
        .trim()
        .parse()
        .map_err(|_| format!("{what}: {:?} is not a number", raw.trim()))?;
    if value.is_finite() {
        Ok(value)
    } else {
        Err(format!("{what}: {raw:?} is not a finite number"))
    }
}

/// A loaded editing session.
pub struct Session {
    /// Folder that is currently scanned.
    pub directory: PathBuf,
    /// Savegames found in [`Session::directory`].
    pub entries: Vec<SaveEntry>,
    /// Index into [`Session::entries`] of the opened savegame.
    pub selected: Option<usize>,
    /// The opened savegame.
    pub save: Option<SaveGame>,
    /// Character of the opened savegame.
    pub character: Option<Character>,
    /// Inventory of the opened savegame.
    pub items: Vec<ItemRow>,
    /// Fighting styles of the opened savegame.
    pub styles: Vec<StyleRow>,
    /// Companions of the opened savegame.
    pub henchmen: Vec<HenchRow>,
    /// Gems of the opened savegame.
    pub gems: Vec<GemRow>,
    /// Amulet tier and unlocked sockets; `None` until the amulet is unlocked.
    pub amulet: Option<AmuletInfo>,
    /// Game data used for names.
    pub game: Option<Rc<GameData>>,
    /// Human readable state of the game data.
    pub game_status: GameStatus,
    /// Screenshot of the opened savegame.
    pub preview: Option<Preview>,
    /// Status of the last action.
    pub message: Message,
    preview_generation: u64,
}

impl Default for Session {
    fn default() -> Self {
        Self::new()
    }
}

impl Session {
    /// Creates an empty session pointing at the default save directory.
    #[must_use]
    pub fn new() -> Self {
        Self::with_directory(default_save_dir())
    }

    /// Creates a session for a specific directory.
    #[must_use]
    pub fn with_directory(directory: impl Into<PathBuf>) -> Self {
        let mut session = Self {
            directory: directory.into(),
            entries: Vec::new(),
            selected: None,
            save: None,
            character: None,
            items: Vec::new(),
            styles: Vec::new(),
            henchmen: Vec::new(),
            gems: Vec::new(),
            amulet: None,
            game: None,
            game_status: GameStatus::Missing,
            preview: None,
            message: Message::info("Select a savegame to start editing."),
            preview_generation: 0,
        };
        session.scan();
        session
    }

    /// Changes the directory and scans it.
    ///
    /// Refuses to throw away unsaved changes.
    pub fn set_directory(&mut self, directory: impl Into<PathBuf>) {
        let directory = directory.into();
        if self.is_dirty() {
            self.message = Message::info(
                "This savegame has unsaved changes: save them or press Revert before scanning.",
            );
            return;
        }
        self.directory = directory;
        self.persist_directory();
        self.selected = None;
        self.save = None;
        self.character = None;
        self.items.clear();
        self.styles.clear();
        self.henchmen.clear();
        self.preview = None;
        self.scan();
    }

    /// Scans the directory for savegames and loads the game data.
    pub fn scan(&mut self) {
        self.entries.clear();
        let directory = self.directory.clone();
        if directory.join("save.sac").is_file() {
            self.entries.push(describe(&directory));
        } else if let Ok(read) = std::fs::read_dir(&directory) {
            let mut candidates: Vec<PathBuf> = read
                .flatten()
                .map(|entry| entry.path())
                .filter(|path| path.join("save.sac").is_file())
                .collect();
            candidates.sort();
            self.entries = candidates.iter().map(|path| describe(path)).collect();
        }

        self.load_game_data();

        let total = self.entries.len();
        let broken = self
            .entries
            .iter()
            .filter(|entry| entry.error.is_some())
            .count();
        let plural = if total == 1 { "" } else { "s" };
        let directory = self.directory.display();
        if total == 0 {
            self.message = Message::info(format!("No savegames found in {directory}."));
        } else if broken == 0 {
            self.message =
                Message::success(format!("Found {total} savegame{plural} in {directory}."));
        } else if broken == total {
            self.message = Message::error(format!(
                "Found {total} savegame{plural} in {directory}, but none of them can be read."
            ));
        } else {
            self.message = Message::error(format!(
                "Found {total} savegame{plural} in {directory}, {broken} of them cannot be read."
            ));
        }
    }

    /// Remembers the folder for the next start (best effort).
    fn persist_directory(&self) {
        if self.directory.as_os_str().is_empty() {
            return;
        }
        let config = crate::config::Config {
            save_dir: Some(self.directory.clone()),
        };
        let _ = config.store();
    }

    /// Loads the game installation that belongs to the save folder.
    fn load_game_data(&mut self) {
        if let Some(game) = GameData::discover(&self.directory) {
            self.game_status = GameStatus::Loaded {
                items: game.playable_item_count(),
                styles: game.style_count(),
                root: game.root().to_path_buf(),
            };
            self.game = Some(Rc::new(game));
        } else {
            self.game = None;
            self.game_status = GameStatus::Missing;
        }
    }

    /// Opens the savegame at `index` of the entry list.
    ///
    /// Refuses to switch away from a savegame with unsaved changes.
    pub fn open(&mut self, index: usize) -> bool {
        if self.is_dirty() && self.selected != Some(index) {
            self.message = Message::info(
                "This savegame has unsaved changes: save them or press Revert first.",
            );
            return false;
        }
        self.load(index)
    }

    /// Loads a savegame, discarding unsaved changes.
    fn load(&mut self, index: usize) -> bool {
        let Some(entry) = self.entries.get(index) else {
            return false;
        };
        let path = entry.path.join("save.sac");
        match SaveGame::open(&path) {
            Ok(save) => {
                let label = entry.label.clone();
                self.character = save.character().ok();
                self.save = Some(save);
                self.selected = Some(index);
                self.rebuild_rows();
                self.load_preview(&path);
                if self.game.is_none() {
                    self.load_game_data();
                }
                self.message = Message::success(format!("Opened {label}."));
                true
            }
            Err(error) => {
                self.message = Message::error(format!("Cannot open {}: {error}", path.display()));
                false
            }
        }
    }

    fn load_preview(&mut self, save_path: &Path) {
        self.preview_generation += 1;
        self.preview = SaveImage::load(save_path).ok().map(|image| Preview {
            width: image.width,
            height: image.height,
            rgba: image.rgba,
            generation: self.preview_generation,
        });
    }

    /// Re-reads items, styles and companions from the open savegame.
    fn rebuild_rows(&mut self) {
        let rows = self.build_rows();
        if let Some(rows) = rows {
            self.items = rows.0;
            self.styles = rows.1;
            self.henchmen = rows.2;
        }
        self.gems = self.build_gem_rows();
    }

    fn build_rows(&self) -> Option<(Vec<ItemRow>, Vec<StyleRow>, Vec<HenchRow>)> {
        let save = self.save.as_ref()?;
        let items = save
            .items()
            .unwrap_or_default()
            .into_iter()
            .map(|item| self.describe_item(&item))
            .collect();
        let styles = save
            .styles()
            .unwrap_or_default()
            .into_iter()
            .map(|style| self.describe_style(&style))
            .collect();
        let henchmen = save
            .henchmen()
            .unwrap_or_default()
            .into_iter()
            .map(describe_henchman)
            .collect();
        Some((items, styles, henchmen))
    }

    fn describe_item(&self, item: &Item) -> ItemRow {
        let info = self.game.as_ref().and_then(|game| game.item(item.item_id));
        let fallback = format!("Item {}", item.item_id);
        ItemRow {
            struct_index: item.struct_index,
            item_id: item.item_id,
            count: item.count,
            kind: item.kind,
            name: info.map_or(fallback, |info| info.name.clone()),
            category: info.and_then(|info| info.category.clone()),
            description: info.and_then(|info| info.description.clone()),
        }
    }

    fn describe_style(&self, style: &Style) -> StyleRow {
        let info = self
            .game
            .as_ref()
            .and_then(|game| game.style(style.style_id));
        let fallback = format!("Style {}", style.style_id);
        StyleRow {
            struct_index: style.struct_index,
            style_id: style.style_id,
            level: style.level,
            name: info.map_or(fallback, |info| info.name.clone()),
            label: info.map_or_else(String::new, |info| info.label.clone()),
            description: info.and_then(|info| info.description.clone()),
            mask: style.mask,
            upgrades: self.upgrade_choices(style.style_id, style.mask),
            kind: info.map_or("style", |info| {
                if info.kind == jadeforge_gamedata::StyleKind::Other && info.weapon_count > 0 {
                    "weapon"
                } else {
                    info.kind.label()
                }
            }),
        }
    }

    /// True when the opened savegame has unsaved changes.
    #[must_use]
    pub fn is_dirty(&self) -> bool {
        self.save.as_ref().is_some_and(SaveGame::is_modified)
    }

    /// Path of the opened savegame.
    #[must_use]
    pub fn current_path(&self) -> Option<PathBuf> {
        self.selected
            .and_then(|index| self.entries.get(index))
            .map(|entry| entry.path.join("save.sac"))
    }

    /// Number of blocks in the open savegame.
    #[must_use]
    pub fn block_count(&self) -> usize {
        self.save.as_ref().map_or(0, |save| save.blocks().len())
    }

    /// Size of the open savegame in bytes.
    #[must_use]
    pub fn file_size(&self) -> Option<u64> {
        let path = self.current_path()?;
        std::fs::metadata(path).ok().map(|meta| meta.len())
    }

    /// Applies the form to the in memory savegame.
    pub fn apply(&mut self, form: &CharacterForm) -> Result<usize, String> {
        let Some(base) = self.character.as_ref() else {
            return Err("no savegame is open".to_owned());
        };
        let character = form.to_character(base)?;
        let Some(save) = self.save.as_mut() else {
            return Err("no savegame is open".to_owned());
        };
        match save.apply_character(&character) {
            Ok(changes) => {
                self.character = Some(character);
                self.rebuild_rows();
                self.refresh_entry();
                if changes.is_empty() {
                    self.message = Message::info("Nothing to apply.");
                } else {
                    self.message =
                        Message::success(format!("Applied {} change(s) in memory.", changes.len()));
                }
                Ok(changes.len())
            }
            Err(error) => {
                self.message = Message::error(error.to_string());
                Err(error.to_string())
            }
        }
    }

    /// Writes the opened savegame back to disk.
    pub fn save(&mut self) -> Result<PathBuf, String> {
        let Some(save) = self.save.as_mut() else {
            return Err("no savegame is open".to_owned());
        };
        if !save.is_modified() {
            self.message = Message::info("Nothing was changed, not writing a file.");
            return Err("nothing to save".to_owned());
        }
        match save.save(SaveOptions { backup: true }) {
            Ok(path) => {
                self.message = Message::success(format!("Saved {}.", path.display()));
                self.refresh_entry();
                Ok(path)
            }
            Err(error) => {
                self.message = Message::error(format!("Cannot save: {error}"));
                Err(error.to_string())
            }
        }
    }

    /// Discards all changes by reloading the file.
    pub fn revert(&mut self) -> Result<(), String> {
        let Some(index) = self.selected else {
            return Err("no savegame is open".to_owned());
        };
        if self.load(index) {
            self.message = Message::info("Reloaded the savegame from disk.");
            Ok(())
        } else {
            Err("cannot reload the savegame".to_owned())
        }
    }

    /// Changes the stack size of an item by `delta`.
    pub fn adjust_item_count(&mut self, struct_index: usize, delta: i32) {
        let Some((name, current)) = self
            .items
            .iter()
            .find(|item| item.struct_index == struct_index)
            .map(|item| (item.name.clone(), item.count))
        else {
            return;
        };
        let new_count = i32::from(current) + delta;
        if new_count > 255 {
            self.message = Message::info(format!("{name}: a stack cannot hold more than 255."));
            return;
        }
        if new_count < 1 {
            self.message = Message::info("A stack needs at least one item; use Remove instead.");
            return;
        }
        let Some(save) = self.save.as_mut() else {
            return;
        };
        match save.set_item_count(struct_index, new_count as u8) {
            Ok(()) => {
                if let Some(item) = self
                    .items
                    .iter_mut()
                    .find(|item| item.struct_index == struct_index)
                {
                    item.count = new_count as u8;
                }
                self.message = Message::success(format!("{name}: {new_count} in the stack."));
            }
            Err(error) => self.message = Message::error(format!("Cannot change item: {error}")),
        }
    }

    /// Adds an item stack by its `items.2da` row.
    pub fn add_item(&mut self, item_id: u16) -> Result<(), String> {
        let (name, save_type) = self
            .game
            .as_ref()
            .and_then(|game| game.item(item_id))
            .map_or_else(
                || (format!("Item {item_id}"), 8),
                |info| (info.name.clone(), info.save_type.unwrap_or(8)),
            );
        let Some(save) = self.save.as_mut() else {
            return Err("no savegame is open".to_owned());
        };
        match save.add_item(item_id, save_type, 1) {
            Ok(_) => {
                self.rebuild_rows();
                self.message = Message::success(format!("Added {name}."));
                Ok(())
            }
            Err(error) => {
                self.message = Message::error(format!("Cannot add item: {error}"));
                Err(error.to_string())
            }
        }
    }

    /// Removes an item stack.
    pub fn remove_item(&mut self, struct_index: usize) {
        let Some(name) = self
            .items
            .iter()
            .find(|item| item.struct_index == struct_index)
            .map(|item| item.name.clone())
        else {
            return;
        };
        let Some(save) = self.save.as_mut() else {
            return;
        };
        match save.remove_item(struct_index) {
            Ok(true) => {
                self.items.retain(|item| item.struct_index != struct_index);
                self.message = Message::success(format!("Removed {name}."));
            }
            Ok(false) => self.message = Message::info("The item was already gone."),
            Err(error) => self.message = Message::error(format!("Cannot remove item: {error}")),
        }
    }

    /// Searches the item database.
    ///
    /// An empty query lists the whole database so the picker doubles as a
    /// browsable list.
    #[must_use]
    pub fn search_items(&self, query: &str, limit: usize) -> (Vec<ItemSuggestion>, usize) {
        let Some(game) = self.game.as_ref() else {
            return (Vec::new(), 0);
        };
        let matches: Vec<&jadeforge_gamedata::ItemInfo> = if query.trim().is_empty() {
            let mut all: Vec<&jadeforge_gamedata::ItemInfo> = game.playable_items().collect();
            all.sort_by(|left, right| {
                left.name
                    .to_lowercase()
                    .cmp(&right.name.to_lowercase())
                    .then_with(|| left.id.cmp(&right.id))
            });
            all
        } else {
            game.search_items(query, usize::MAX)
        };
        let total = matches.len();
        let rows: Vec<ItemSuggestion> = matches
            .into_iter()
            .map(|item| ItemSuggestion {
                item_id: item.id,
                name: item.name.clone(),
                category: item.category.clone().unwrap_or_else(|| "item".to_owned()),
                save_type: item.save_type.unwrap_or(8),
                description: item.description.clone(),
            })
            .take(limit)
            .collect();
        (rows, total)
    }

    /// Builds the upgrade slots a style offers, marked with what is owned.
    fn upgrade_choices(&self, style_id: u16, mask: u64) -> Vec<StyleUpgrade> {
        let Some(game) = self.game.as_ref() else {
            return Vec::new();
        };
        let Some(choices) = game.style_choices(style_id) else {
            return Vec::new();
        };
        choices
            .iter()
            .map(|choice| {
                let advance = game.advance(choice.advance);
                StyleUpgrade {
                    slot: choice.slot,
                    advance: choice.advance,
                    family: choice.family.clone(),
                    tier: choice.tier,
                    name: advance.map_or_else(
                        || format!("Upgrade {}", choice.advance),
                        |advance| advance.name.clone(),
                    ),
                    effect: advance
                        .map_or_else(String::new, jadeforge_gamedata::Advance::effect_text),
                    cost: advance.map_or(0, |advance| advance.cost),
                    taken: mask & (1_u64 << (choice.slot - 1)) != 0,
                }
            })
            .collect()
    }

    /// Changes a whole family at once: owning the clicked slot means every
    /// missing slot up to it is bought, an owned slot is removed together with
    /// everything above it.
    ///
    /// This mirrors the game's level-up screen (one pick per level, never a
    /// gap), so `1 -> 4` buys slots 2, 3 and 4 and clicking slot 1 of a maxed
    /// family drops it back to zero.
    pub fn change_upgrade(&mut self, struct_index: usize, slot: u16) -> bool {
        let Some(style) = self
            .styles
            .iter()
            .find(|style| style.struct_index == struct_index)
            .cloned()
        else {
            return false;
        };
        let Some(clicked) = style.upgrades.iter().find(|upgrade| upgrade.slot == slot) else {
            return false;
        };
        let taken = !clicked.taken;
        let mut targets: Vec<u16> = style
            .upgrades
            .iter()
            .filter(|candidate| candidate.family == clicked.family)
            .filter(|candidate| {
                if taken {
                    candidate.slot <= clicked.slot
                } else {
                    candidate.slot >= clicked.slot
                }
            })
            .map(|candidate| candidate.slot)
            .collect();
        targets.sort_unstable();
        let mut changed = 0_usize;
        let mut error = None;
        if let Some(save) = self.save.as_mut() {
            for target in &targets {
                match save.set_style_slot(struct_index, *target, taken) {
                    Ok(_) => changed += 1,
                    Err(failure) => {
                        error = Some(failure);
                        break;
                    }
                }
            }
        } else {
            return false;
        }
        if let Some(error) = error {
            self.rebuild_rows();
            self.message = Message::error(format!("Cannot change the upgrade: {error}"));
            return false;
        }
        self.rebuild_rows();
        self.message = if changed == 0 {
            Message::info("That upgrade is already in the requested state.")
        } else {
            let noun = if changed == 1 { "upgrade" } else { "upgrades" };
            if taken {
                Message::success(format!("Bought {changed} {noun}."))
            } else {
                Message::success(format!("Removed {changed} {noun}."))
            }
        };
        changed > 0
    }

    fn build_gem_rows(&mut self) -> Vec<GemRow> {
        let Some(save) = self.save.as_ref() else {
            return Vec::new();
        };
        let Ok(gems) = save.gems() else {
            return Vec::new();
        };
        let amulet = save.amulet().unwrap_or(None);
        self.amulet = amulet;
        let mut seen_amulet = 0_usize;
        gems.into_iter()
            .filter(|gem| {
                if gem.location == GemLocation::Amulet {
                    let Some(amulet) = amulet else {
                        return false;
                    };
                    seen_amulet += 1;
                    seen_amulet <= usize::from(amulet.sockets)
                } else {
                    true
                }
            })
            .map(|gem| {
                let info = self.game.as_ref().and_then(|game| game.item(gem.item_id));
                let empty = info.is_none() && gem.effects.is_empty();
                let effects: Vec<String> = gem
                    .effects
                    .iter()
                    .map(|effect| {
                        let name = self
                            .game
                            .as_ref()
                            .and_then(|game| game.effect_name(u32::try_from(effect.kind).ok()?))
                            .map_or_else(|| format!("Effect {}", effect.kind), ToOwned::to_owned);
                        if effect.duration > 0.0 {
                            format!("{:+} {} ({:.0}s)", effect.value, name, effect.duration)
                        } else {
                            format!("{:+} {}", effect.value, name)
                        }
                    })
                    .collect();
                GemRow {
                    struct_index: gem.struct_index,
                    name: info.map_or_else(
                        || {
                            if empty {
                                "Empty socket".to_owned()
                            } else {
                                format!("Gem {}", gem.item_id)
                            }
                        },
                        |item| item.name.clone(),
                    ),
                    effects,
                    empty,
                    location: gem.location,
                }
            })
            .collect()
    }

    /// Gems that can be added, as `(item id, name, effect count)`.
    #[must_use]
    pub fn available_gems(&self) -> Vec<(u16, String, usize)> {
        self.game.as_ref().map_or_else(Vec::new, |game| {
            game.gems()
                .iter()
                .map(|gem| (gem.item_id, gem.name.clone(), gem.effects.len()))
                .collect()
        })
    }

    /// Adds a predefined gem to the inventory.
    pub fn add_gem(&mut self, item_id: u16) {
        let Some(gem) = self
            .game
            .as_ref()
            .and_then(|game| game.gem(item_id))
            .cloned()
        else {
            self.message = Message::error("That gem is not in the game data.");
            return;
        };
        let Some(save) = self.save.as_mut() else {
            return;
        };
        match save.add_gem(item_id, &gem.effects, gem.duration) {
            Ok(_) => {
                self.gems = self.build_gem_rows();
                self.message = Message::success(format!("Added {}.", gem.name));
            }
            Err(error) => {
                self.message = Message::error(format!("Cannot add the gem: {error}"));
            }
        }
    }

    /// Moves a gem between the amulet and the inventory.
    pub fn move_gem(&mut self, struct_index: usize) {
        let Some(save) = self.save.as_mut() else {
            return;
        };
        match save.move_gem(struct_index) {
            Ok(location) => {
                self.gems = self.build_gem_rows();
                self.message = Message::success(match location {
                    GemLocation::Amulet => "Gem socketed into the amulet.",
                    GemLocation::Inventory => "Gem moved to the inventory.",
                });
            }
            Err(error) => {
                self.message = Message::error(format!("Cannot move the gem: {error}"));
            }
        }
    }

    /// Removes a gem from the inventory or the amulet.
    pub fn remove_gem(&mut self, struct_index: usize) {
        let Some(save) = self.save.as_mut() else {
            return;
        };
        match save.remove_gem(struct_index) {
            Ok(()) => {
                self.gems = self.build_gem_rows();
                self.message = Message::success("Gem removed.");
            }
            Err(error) => {
                self.message = Message::error(format!("Cannot remove the gem: {error}"));
            }
        }
    }

    /// All styles from the game data that the player does not own yet.
    ///
    /// Styles used by henchmen or scripted scenes are filtered out: they cannot
    /// be learned by the player and would only clutter the list.
    #[must_use]
    pub fn missing_styles(&self) -> Vec<StyleSuggestion> {
        let Some(game) = self.game.as_ref() else {
            return Vec::new();
        };
        let owned: std::collections::HashSet<u16> =
            self.styles.iter().map(|style| style.style_id).collect();
        let mut styles: Vec<StyleSuggestion> = game
            .styles()
            .filter(|style| !owned.contains(&style.id))
            .filter(|style| is_player_style(&style.label))
            .map(|style| StyleSuggestion {
                style_id: style.id,
                name: style.name.clone(),
                label: style.label.clone(),
                description: style.description.clone(),
                kind: if style.kind == jadeforge_gamedata::StyleKind::Other
                    && style.weapon_count > 0
                {
                    "weapon"
                } else {
                    style.kind.label()
                },
            })
            .collect();
        styles.sort_by(|left, right| {
            left.name
                .cmp(&right.name)
                .then_with(|| left.style_id.cmp(&right.style_id))
        });
        styles
    }

    /// Adds a style by its `stylesuperlist.2da` row.
    pub fn add_style(&mut self, style_id: u16) {
        let Some((name, name_ref)) = self
            .game
            .as_ref()
            .and_then(|game| game.style(style_id))
            .map(|info| (info.name.clone(), info.name_ref))
        else {
            self.message = Message::error("Unknown style.");
            return;
        };
        let Some(template) = self.styles.first().map(|style| style.struct_index) else {
            self.message =
                Message::error("Learn a style first so JadeForge can copy the field layout.");
            return;
        };
        let Some(save) = self.save.as_mut() else {
            return;
        };
        match save.add_style(template, style_id, name_ref) {
            Ok(_) => {
                self.rebuild_rows();
                self.message = Message::success(format!("Added {name}."));
            }
            Err(error) => self.message = Message::error(format!("Cannot add style: {error}")),
        }
    }

    /// Removes a style from the player.
    pub fn remove_style(&mut self, struct_index: usize) {
        let Some(name) = self
            .styles
            .iter()
            .find(|style| style.struct_index == struct_index)
            .map(|style| style.name.clone())
        else {
            return;
        };
        let Some(save) = self.save.as_mut() else {
            return;
        };
        match save.remove_style(struct_index) {
            Ok(true) => {
                self.styles
                    .retain(|style| style.struct_index != struct_index);
                self.message = Message::success(format!("Removed {name}."));
            }
            Ok(false) => self.message = Message::info("The style was already gone."),
            Err(error) => self.message = Message::error(format!("Cannot remove style: {error}")),
        }
    }

    /// Changes the level of a companion.
    pub fn set_henchman_level(&mut self, struct_index: usize, level: u8) {
        if !(1..=60).contains(&level) {
            self.message = Message::info("Companion levels go from 1 to 60.");
            return;
        }
        let Some(save) = self.save.as_mut() else {
            return;
        };
        match save.set_henchman_level(struct_index, level) {
            Ok(()) => {
                if let Some(henchman) = self
                    .henchmen
                    .iter_mut()
                    .find(|henchman| henchman.struct_index == struct_index)
                {
                    henchman.level = level;
                }
                self.message = Message::success(format!("Companion level set to {level}."));
            }
            Err(error) => {
                self.message = Message::error(format!("Cannot change companion: {error}"));
            }
        }
    }

    /// Makes a companion available or unavailable.
    pub fn toggle_henchman_available(&mut self, struct_index: usize) {
        let Some((name, available)) = self
            .henchmen
            .iter()
            .find(|henchman| henchman.struct_index == struct_index)
            .map(|henchman| (henchman.name.clone(), henchman.available))
        else {
            return;
        };
        let Some(save) = self.save.as_mut() else {
            return;
        };
        match save.set_henchman_available(struct_index, !available) {
            Ok(()) => {
                if let Some(henchman) = self
                    .henchmen
                    .iter_mut()
                    .find(|henchman| henchman.struct_index == struct_index)
                {
                    henchman.available = !available;
                }
                let state = if available {
                    "unavailable"
                } else {
                    "available"
                };
                self.message = Message::success(format!("{name} is now {state}."));
            }
            Err(error) => {
                self.message = Message::error(format!("Cannot change companion: {error}"));
            }
        }
    }

    /// Reports a validation problem in the status line.
    pub fn set_error(&mut self, text: &str) {
        self.message = Message::error(text);
    }

    /// Refreshes the summary of the selected entry after an edit.
    fn refresh_entry(&mut self) {
        let Some(index) = self.selected else {
            return;
        };
        let Some(character) = self.character.as_ref() else {
            return;
        };
        if let Some(entry) = self.entries.get_mut(index) {
            entry.character = Some(character.name.clone());
            entry.level = Some(character.level);
            entry.time_played = character.time_played;
        }
    }
}

fn describe(folder: &Path) -> SaveEntry {
    let label = folder.file_name().map_or_else(
        || folder.display().to_string(),
        |name| name.to_string_lossy().into(),
    );
    let path = folder.join("save.sac");
    match SaveGame::open(&path) {
        Ok(save) => match save.character() {
            Ok(character) => SaveEntry {
                path: folder.to_path_buf(),
                label,
                character: Some(character.name),
                level: Some(character.level),
                time_played: character.time_played,
                error: None,
            },
            Err(error) => SaveEntry {
                path: folder.to_path_buf(),
                label,
                character: None,
                level: None,
                time_played: 0,
                error: Some(error.to_string()),
            },
        },
        Err(error) => SaveEntry {
            path: folder.to_path_buf(),
            label,
            character: None,
            level: None,
            time_played: 0,
            error: Some(error.to_string()),
        },
    }
}

fn describe_henchman(henchman: Henchman) -> HenchRow {
    HenchRow {
        struct_index: henchman.struct_index,
        name: henchman_name(&henchman.resref),
        resref: henchman.resref,
        level: henchman.level,
        available: henchman.available,
    }
}

/// Heuristic: styles that the player can learn.
///
/// The style table also contains variants for henchmen, cutscenes and unused
/// development entries; their labels start with `C_`, `NPC_`, `HENCH_` or
/// `cut_`.
#[must_use]
pub fn is_player_style(label: &str) -> bool {
    let lower = label.to_lowercase();
    !(lower.starts_with("cut_")
        || lower.starts_with("npc_")
        || lower.starts_with("hench")
        || lower.starts_with("c_")
        || lower.starts_with("zzz"))
}

/// Best effort display name for a companion template.
///
/// The game keeps companion names in the module archives that are not part of
/// the base installation data, so `JadeForge` ships the names of the game's
/// companions and prettifies unknown templates.
#[must_use]
pub fn henchman_name(resref: &str) -> String {
    let known = match resref.to_lowercase().as_str() {
        "j00h_ho" => Some("Henpecked Hou"),
        "j00h_kang" => Some("Kang the Mad"),
        "j00h_sky" => Some("Sky"),
        "j00h_bwind" => Some("Black Whirlwind"),
        "j00h_dstar" => Some("Dawn Star"),
        "j00h_zu" => Some("Zu"),
        "j00h_sfox" => Some("Silk Fox"),
        "j00h_chaika" => Some("Chaika"),
        "j00h_abbot" => Some("Abbot Song"),
        _ => None,
    };
    if let Some(name) = known {
        return name.to_owned();
    }
    let trimmed = resref
        .trim_start_matches("j00h_")
        .trim_start_matches("j00c_")
        .trim_start_matches("j00_")
        .trim_end_matches('0');
    let mut characters = trimmed.chars();
    match characters.next() {
        Some(first) => first.to_uppercase().chain(characters).collect(),
        None => resref.to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn character() -> Character {
        Character {
            name: "Test".to_owned(),
            level: 7,
            experience: 1234,
            gold: Some(99),
            attribute_points: 2,
            health: 120.0,
            health_max: 150,
            chi: 80.0,
            chi_max: 100,
            focus: 20.0,
            focus_max: 30.0,
            body: 5,
            mind: 6,
            spirit: 7,
            charm: 1,
            intuition: 2,
            intimidate: 3,
            alignment: 500,
            appearance: 4,
            chapter: 2,
            time_played: 90_000_000_000,
            kills: 42,
        }
    }

    #[test]
    fn form_round_trips_character() {
        let character = character();
        let form = CharacterForm::from_character(&character);
        let parsed = form.to_character(&character).expect("parse");
        assert_eq!(parsed, character);
    }

    #[test]
    fn form_reports_invalid_numbers() {
        let character = character();
        let mut form = CharacterForm::from_character(&character);
        form.level = "abc".to_owned();
        let error = form.to_character(&character).expect_err("invalid");
        assert!(error.contains("level"), "{error}");
    }

    #[test]
    fn empty_session_has_a_message() {
        let session = Session::with_directory("/definitely/not/a/save/directory");
        assert_eq!(session.entries.len(), 0, "no savegames expected here");
        assert!(session.message.text.contains("No savegames"));
    }

    #[test]
    fn natural_order_compares_numbers() {
        use std::cmp::Ordering;
        assert_eq!(natural_order("Auto 2", "Auto 10"), Ordering::Less);
        assert_eq!(natural_order("Auto 10", "Auto 2"), Ordering::Greater);
        assert_eq!(natural_order("Auto 2", "Auto 2"), Ordering::Equal);
        assert_eq!(natural_order("Auto 2", "Auto 02"), Ordering::Equal);
        assert_eq!(natural_order("Spielstand 9", "Auto 99"), Ordering::Greater);
    }

    #[test]
    fn alignment_names_follow_the_engine_default() {
        assert_eq!(format::alignment(220), "Way of the Open Palm");
        assert_eq!(format::alignment(129), "Way of the Open Palm");
        assert_eq!(format::alignment(128), "Balanced");
        assert_eq!(format::alignment(127), "Way of the Closed Fist");
        assert_eq!(format::alignment(0), "Way of the Closed Fist");
    }

    #[test]
    fn entries_can_be_filtered_and_sorted() {
        let entry = |label: &str, level: u8, played: u64| SaveEntry {
            path: PathBuf::from(label),
            label: label.to_owned(),
            character: Some(format!("{label} hero")),
            level: Some(level),
            time_played: played,
            error: None,
        };
        let entries = vec![
            entry("Auto 10", 5, 3_000),
            entry("Auto 2", 9, 1_000),
            entry("Auto 1", 1, 2_000),
        ];
        let names = |indices: &[usize]| -> Vec<&str> {
            indices
                .iter()
                .map(|index| entries[*index].label.as_str())
                .collect()
        };
        assert_eq!(
            names(&visible_entries(&entries, "", SaveSort::Name, false)),
            ["Auto 1", "Auto 2", "Auto 10"]
        );
        assert_eq!(
            names(&visible_entries(&entries, "", SaveSort::Level, true)),
            ["Auto 2", "Auto 10", "Auto 1"]
        );
        assert_eq!(
            names(&visible_entries(&entries, "", SaveSort::Played, false)),
            ["Auto 2", "Auto 1", "Auto 10"]
        );
        assert_eq!(
            names(&visible_entries(&entries, "hero", SaveSort::Name, false)).len(),
            3
        );
        assert_eq!(
            names(&visible_entries(&entries, "auto 2", SaveSort::Name, false)),
            ["Auto 2"]
        );
        assert_eq!(
            visible_entries(&entries, "nothing", SaveSort::Name, false).len(),
            0
        );
    }

    #[test]
    fn companion_names_are_human_readable() {
        assert_eq!(henchman_name("j00h_ho"), "Henpecked Hou");
        assert_eq!(henchman_name("j00h_sfox"), "Silk Fox");
        assert_eq!(henchman_name("j00h_unknown0"), "Unknown");
    }

    #[test]
    fn durations_are_compact() {
        assert_eq!(format::duration(0), "0m");
        assert_eq!(format::duration(90_000_000), "1m");
        assert_eq!(format::duration(3_600_000_000), "1h 00m");
        assert_eq!(format::duration(58_920_000_000), "16h 22m");
    }
}
