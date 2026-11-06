//! UI independent session state.
//!
//! Everything the desktop application does with a savegame goes through this
//! module. It keeps the Freya views thin and makes the behaviour testable
//! without a window.

use std::path::{Path, PathBuf};
use std::rc::Rc;

use jadeforge_core::{
    AmuletInfo, Character, GemLocation, Henchman, Item, SaveGame, SaveImage, SaveOptions, Style,
    StyleIdentity,
};
use jadeforge_gamedata::{BundledItem, GameData, NameIndex};

mod error;
mod form;
mod model;

pub use error::SessionError;
pub use form::{CharacterForm, FormError};
pub use model::{
    GameStatus, GemRow, HenchRow, ItemRow, ItemSuggestion, Message, MessageKind, Preview,
    SaveEntry, StyleRow, StyleSuggestion, StyleUpgrade,
};

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

    /// Formats a duration given in whole seconds.
    ///
    /// Short values keep their seconds instead of rounding down to `0m`.
    #[must_use]
    pub fn elapsed(seconds: u32) -> String {
        if seconds < 60 {
            format!("{seconds}s")
        } else {
            duration(u64::from(seconds) * 1_000_000)
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

    /// Formats a float so that parsing it yields the exact same value again.
    ///
    /// The form is auto-applied while typing, so a rounded display value would
    /// otherwise "change" the savegame just by opening it.
    #[must_use]
    pub fn exact(value: f32) -> String {
        value.to_string()
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
            if index > 0 && (digits.len() - index).is_multiple_of(3) {
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

    /// Stable key used in the settings file.
    #[must_use]
    pub fn key(self) -> &'static str {
        match self {
            Self::Name => "name",
            Self::Level => "level",
            Self::Played => "played",
        }
    }

    /// Parses a key written by [`SaveSort::key`].
    #[must_use]
    pub fn from_key(key: &str) -> Option<Self> {
        match key {
            "name" => Some(Self::Name),
            "level" => Some(Self::Level),
            "played" => Some(Self::Played),
            _ => None,
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

/// A loaded editing session.
pub struct Session {
    /// Folder that is currently scanned.
    pub(crate) directory: PathBuf,
    /// Savegames found in [`Session::directory`].
    ///
    /// Shared so rendering can borrow the list without copying it every frame.
    pub(crate) entries: Rc<Vec<SaveEntry>>,
    /// Index into [`Session::entries`] of the opened savegame.
    pub(crate) selected: Option<usize>,
    /// The opened savegame.
    pub(crate) save: Option<SaveGame>,
    /// Character of the opened savegame.
    pub(crate) character: Option<Character>,
    /// Inventory of the opened savegame.
    pub(crate) items: Rc<Vec<ItemRow>>,
    /// Fighting styles of the opened savegame.
    pub(crate) styles: Rc<Vec<StyleRow>>,
    /// Companions of the opened savegame.
    pub(crate) henchmen: Rc<Vec<HenchRow>>,
    /// Gems of the opened savegame.
    pub(crate) gems: Rc<Vec<GemRow>>,
    /// Amulet tier and unlocked sockets; `None` until the amulet is unlocked.
    pub(crate) amulet: Option<AmuletInfo>,
    /// Game data used for names.
    pub(crate) game: Option<Rc<GameData>>,
    /// Bundled English names, used when no installation is available.
    pub(crate) name_index: Option<&'static NameIndex>,
    /// Human readable state of the game data.
    pub(crate) game_status: GameStatus,
    /// Screenshot of the opened savegame.
    ///
    /// Shared so rendering can clone the handle without copying the RGBA
    /// buffer.
    pub(crate) preview: Option<Rc<Preview>>,
    /// Status of the last action.
    pub(crate) message: Message,
    file_size: Option<u64>,
    preview_generation: u64,
    scan_token: u64,
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
        let mut session = Self::empty(directory);
        session.scan();
        session
    }

    /// Creates a session without scanning the directory.
    ///
    /// The UI shows its first frame immediately and calls [`Session::scan`]
    /// afterwards, so opening the window does not wait for every savegame.
    #[must_use]
    pub fn empty(directory: impl Into<PathBuf>) -> Self {
        let directory = directory.into();
        let mut session = Self {
            directory,
            entries: Rc::new(Vec::new()),
            selected: None,
            save: None,
            character: None,
            items: Rc::new(Vec::new()),
            styles: Rc::new(Vec::new()),
            henchmen: Rc::new(Vec::new()),
            gems: Rc::new(Vec::new()),
            amulet: None,
            game: None,
            name_index: NameIndex::bundled_english(),
            game_status: GameStatus::Missing,
            preview: None,
            message: Message::info("Scanning for savegames…"),
            file_size: None,
            preview_generation: 0,
            scan_token: 0,
        };
        session.load_game_data();
        session
    }

    /// Changes the directory and scans it.
    ///
    /// Refuses to throw away unsaved changes and reports whether the directory
    /// was accepted, so the caller does not show a folder that was rejected.
    pub fn set_directory(&mut self, directory: impl Into<PathBuf>) -> bool {
        let directory = directory.into();
        if self.is_dirty() {
            self.message = Message::info(
                "This savegame has unsaved changes: save them or press Revert before scanning.",
            );
            return false;
        }
        self.directory = directory;
        self.persist_directory();
        self.selected = None;
        self.save = None;
        self.character = None;
        Rc::make_mut(&mut self.items).clear();
        Rc::make_mut(&mut self.styles).clear();
        Rc::make_mut(&mut self.henchmen).clear();
        Rc::make_mut(&mut self.gems).clear();
        self.amulet = None;
        self.preview = None;
        self.file_size = None;
        self.scan();
        true
    }

    /// Replaces the entry list with the result of [`Session::scan_entries`].
    pub fn apply_entries(&mut self, entries: Vec<SaveEntry>) {
        let total = entries.len();
        let broken = entries.iter().filter(|entry| entry.error.is_some()).count();
        self.entries = Rc::new(entries);
        let plural = if total == 1 { "" } else { "s" };
        let directory = self.directory.display();
        self.message = if total == 0 {
            Message::info(format!("No savegames found in {directory}."))
        } else if broken == 0 {
            Message::success(format!("Found {total} savegame{plural} in {directory}."))
        } else if broken == total {
            Message::error(format!(
                "Found {total} savegame{plural} in {directory}, but none of them can be read."
            ))
        } else {
            Message::error(format!(
                "Found {total} savegame{plural} in {directory}, {broken} of them cannot be read."
            ))
        };
    }

    /// Describes every savegame in `directory` without touching session state.
    ///
    /// Intended for a background thread; pass the result to
    /// [`Session::apply_entries`].
    #[must_use]
    pub fn scan_entries(directory: &Path) -> Vec<SaveEntry> {
        save_folders(directory)
            .iter()
            .map(|path| describe(path))
            .collect()
    }

    /// Scans the directory for savegames and loads the game data.
    pub fn scan(&mut self) {
        self.scan_token = self.scan_token.wrapping_add(1);
        self.load_game_data();
        let entries = Self::scan_entries(&self.directory);
        self.apply_entries(entries);
    }

    /// Token identifying the current scan. The GUI captures it before starting
    /// a worker and uses it to discard a stale result.
    #[must_use]
    pub fn scan_token(&self) -> u64 {
        self.scan_token
    }

    /// Applies a worker result only when it still belongs to the current
    /// directory and scan generation.
    pub fn apply_entries_for(
        &mut self,
        directory: &Path,
        token: u64,
        entries: Vec<SaveEntry>,
    ) -> bool {
        if self.directory != directory || self.scan_token != token {
            return false;
        }
        self.apply_entries(entries);
        true
    }

    /// Reports a failed background scan only when it still belongs to the
    /// current directory and scan generation.
    pub fn report_scan_failure_for(&mut self, directory: &Path, token: u64) -> bool {
        if self.directory != directory || self.scan_token != token {
            return false;
        }
        self.report_scan_failure();
        true
    }

    /// Reports that the background scan could not be completed.
    pub fn report_scan_failure(&mut self) {
        Rc::make_mut(&mut self.entries).clear();
        self.message = Message::error(format!(
            "Could not scan {}: the savegame reader stopped unexpectedly.",
            self.directory.display()
        ));
    }

    /// Remembers the folder for the next start (best effort).
    fn persist_directory(&self) {
        if self.directory.as_os_str().is_empty() {
            return;
        }
        let mut config = crate::config::Config::load();
        config.save_dir = Some(self.directory.clone());
        let _ = config.store();
    }

    /// Remembers the savegame list sorting for the next start (best effort).
    pub fn persist_sort(&self, sort: SaveSort, descending: bool) {
        let mut config = crate::config::Config::load();
        config.sort = Some(sort.key().to_owned());
        config.sort_descending = descending;
        let _ = config.store();
    }

    /// Loads the game installation that belongs to the save folder.
    fn load_game_data(&mut self) -> Vec<String> {
        match GameData::discover_result(&self.directory) {
            Ok(Some(game)) => {
                self.game_status = GameStatus::Loaded {
                    items: game.playable_item_count(),
                    styles: game.style_count(),
                    root: game.root().to_path_buf(),
                };
                let warnings = game.warnings().to_vec();
                self.game = Some(Rc::new(game));
                warnings
            }
            Ok(None) => {
                self.game = None;
                self.game_status = match self.name_index {
                    Some(names) if names.item_count() > 0 || names.style_count() > 0 => {
                        GameStatus::NamesOnly {
                            items: names.item_count(),
                            styles: names.style_count(),
                        }
                    }
                    _ => GameStatus::Missing,
                };
                Vec::new()
            }
            Err(error) => {
                self.game = None;
                self.game_status = GameStatus::Error(error.to_string());
                vec![format!("game data: {error}")]
            }
        }
    }

    /// Opens the savegame at `index` of the entry list.
    ///
    /// Refuses to switch away from a savegame with unsaved changes.
    pub fn open(&mut self, index: usize) -> bool {
        if self.selected == Some(index) {
            // Clicking the row that is already open must not reload it: that
            // would silently throw away unsaved edits.
            return true;
        }
        if self.is_dirty() {
            self.message = Message::info(
                "This savegame has unsaved changes: save them or press Revert first.",
            );
            return false;
        }
        let Some(entry) = self.entries.get(index) else {
            self.message = Message::error("That savegame no longer exists; scan the folder again.");
            return false;
        };
        if let Some(error) = &entry.error {
            self.message = Message::error(format!(
                "Cannot open {}: the scan already found it unreadable: {error}",
                entry.label
            ));
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
                self.file_size = std::fs::metadata(&path).ok().map(|meta| meta.len());
                let label = entry.label.clone();
                let mut warnings = Vec::new();
                self.character = match save.character() {
                    Ok(character) => Some(character),
                    Err(error) => {
                        warnings.push(format!("character: {error}"));
                        None
                    }
                };
                self.save = Some(save);
                self.selected = Some(index);
                warnings.extend(self.rebuild_rows());
                self.load_preview(&path);
                if self.game.is_none() {
                    warnings.extend(self.load_game_data());
                }
                self.message = if warnings.is_empty() {
                    Message::success(format!("Opened {label}."))
                } else {
                    Message::error(format!(
                        "Opened {label}, but some sections could not be read: {}",
                        warnings.join("; ")
                    ))
                };
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
        self.preview = SaveImage::load(save_path).ok().map(|image| {
            Rc::new(Preview {
                width: image.width,
                height: image.height,
                rgba: image.rgba,
                generation: self.preview_generation,
            })
        });
    }

    /// Re-reads items, styles and companions from the open savegame.
    ///
    /// Returns the section errors that were recovered from, so the UI can say
    /// that a savegame opened with missing sections instead of pretending every
    /// read succeeded.
    fn rebuild_rows(&mut self) -> Vec<String> {
        let (items, styles, henchmen, mut warnings) = self.build_rows();
        self.items = Rc::new(items);
        self.styles = Rc::new(styles);
        self.henchmen = Rc::new(henchmen);
        let (gems, gem_warnings) = self.build_gem_rows();
        self.gems = Rc::new(gems);
        warnings.extend(gem_warnings);
        warnings
    }

    fn build_rows(&self) -> (Vec<ItemRow>, Vec<StyleRow>, Vec<HenchRow>, Vec<String>) {
        let Some(save) = self.save.as_ref() else {
            return (Vec::new(), Vec::new(), Vec::new(), Vec::new());
        };
        let mut warnings = Vec::new();
        let items = match save.items() {
            Ok(items) => items
                .into_iter()
                .map(|item| self.describe_item(&item))
                .collect(),
            Err(error) => {
                warnings.push(format!("items: {error}"));
                Vec::new()
            }
        };
        let styles = match save.styles() {
            Ok(styles) => styles
                .into_iter()
                .map(|style| self.describe_style(&style))
                .collect(),
            Err(error) => {
                warnings.push(format!("styles: {error}"));
                Vec::new()
            }
        };
        let henchmen = match save.henchmen() {
            Ok(henchmen) => henchmen.into_iter().map(describe_henchman).collect(),
            Err(error) => {
                warnings.push(format!("companions: {error}"));
                Vec::new()
            }
        };
        (items, styles, henchmen, warnings)
    }

    fn describe_item(&self, item: &Item) -> ItemRow {
        let info = self.game.as_ref().and_then(|game| game.item(item.item_id));
        let bundled = self.bundled_item(item.item_id);
        let fallback = format!("Item {}", item.item_id);
        let name = info
            .map(|info| info.name.clone())
            .or_else(|| bundled.map(|item| item.name.clone()))
            .unwrap_or(fallback);
        let category = info
            .and_then(|info| info.category.clone())
            .or_else(|| jadeforge_gamedata::category_for_save_type(item.kind).map(str::to_owned))
            .or_else(|| bundled.and_then(|item| item.category.clone()));
        ItemRow {
            struct_index: item.struct_index,
            item_id: item.item_id,
            count: item.count,
            kind: item.kind,
            name,
            category,
            description: info.and_then(|info| info.description.clone()),
        }
    }

    fn describe_style(&self, style: &Style) -> StyleRow {
        let info = self
            .game
            .as_ref()
            .and_then(|game| game.style(style.style_id));
        let fallback = format!("Style {}", style.style_id);
        let name = info
            .map(|info| info.name.clone())
            .or_else(|| self.bundled_style(style.style_id).map(ToOwned::to_owned))
            .unwrap_or(fallback);
        // Without a label, `WeaponCount` is the one kind signal the savegame
        // itself carries. It is already the documented fallback when a label
        // prefix is not recognised.
        let fallback_kind = if style.weapon_count > 0 {
            "weapon"
        } else {
            "style"
        };
        StyleRow {
            struct_index: style.struct_index,
            style_id: style.style_id,
            level: style.level,
            name,
            label: info.map_or_else(String::new, |info| info.label.clone()),
            description: info.and_then(|info| info.description.clone()),
            icon: style.icon.clone(),
            weapon: style.weapon.clone(),
            weapon_count: style.weapon_count,
            time_spent: style.time_spent.max(0.0).round() as u32,
            mask: style.mask,
            upgrades: self.upgrade_choices(style.style_id, style.mask),
            kind: info.map_or(fallback_kind, |info| {
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

    /// Folder that is currently scanned.
    #[must_use]
    pub fn directory(&self) -> &Path {
        &self.directory
    }

    /// Shared list of savegames found in the folder.
    #[must_use]
    pub fn entries(&self) -> Rc<Vec<SaveEntry>> {
        Rc::clone(&self.entries)
    }

    /// Index of the opened savegame in [`Session::entries`].
    #[must_use]
    pub fn selected(&self) -> Option<usize> {
        self.selected
    }

    /// The opened savegame, when a row was successfully opened.
    #[must_use]
    pub fn opened_save(&self) -> Option<&SaveGame> {
        self.save.as_ref()
    }

    /// Character of the opened savegame.
    #[must_use]
    pub fn character(&self) -> Option<&Character> {
        self.character.as_ref()
    }

    /// Shared inventory rows.
    #[must_use]
    pub fn items(&self) -> Rc<Vec<ItemRow>> {
        Rc::clone(&self.items)
    }

    /// Shared style rows.
    #[must_use]
    pub fn styles(&self) -> Rc<Vec<StyleRow>> {
        Rc::clone(&self.styles)
    }

    /// Shared companion rows.
    #[must_use]
    pub fn henchmen(&self) -> Rc<Vec<HenchRow>> {
        Rc::clone(&self.henchmen)
    }

    /// Shared gem rows.
    #[must_use]
    pub fn gems(&self) -> Rc<Vec<GemRow>> {
        Rc::clone(&self.gems)
    }

    /// Amulet tier and sockets, when the amulet is unlocked.
    #[must_use]
    pub fn amulet(&self) -> Option<AmuletInfo> {
        self.amulet
    }

    /// Loaded game data, when an installation was found.
    #[must_use]
    pub fn game(&self) -> Option<&GameData> {
        self.game.as_deref()
    }

    /// Human readable state of the game data.
    #[must_use]
    pub fn game_status(&self) -> &GameStatus {
        &self.game_status
    }

    /// True when names come from the bundled English index.
    #[must_use]
    pub fn has_bundled_names(&self) -> bool {
        self.game.is_none()
            && self
                .name_index
                .is_some_and(|names| names.item_count() > 0 || names.style_count() > 0)
    }

    /// Number of items that can be added through the picker.
    #[must_use]
    pub fn item_database_count(&self) -> usize {
        match &self.game {
            Some(game) => game.playable_item_count(),
            None => self.name_index.map_or(0, NameIndex::addable_item_count),
        }
    }

    /// True when the item picker has a database to browse.
    #[must_use]
    pub fn items_ready(&self) -> bool {
        self.item_database_count() > 0
    }

    fn bundled_item(&self, id: u16) -> Option<&BundledItem> {
        if self.game.is_some() {
            return None;
        }
        self.name_index.and_then(|names| names.item(id))
    }

    fn bundled_style(&self, id: u16) -> Option<&str> {
        if self.game.is_some() {
            return None;
        }
        self.name_index.and_then(|names| names.style(id))
    }

    /// Shared screenshot of the opened savegame.
    #[must_use]
    pub fn preview(&self) -> Option<Rc<Preview>> {
        self.preview.clone()
    }

    /// Status of the last action.
    #[must_use]
    pub fn message(&self) -> &Message {
        &self.message
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
        self.file_size
    }

    /// Applies the form to the in memory savegame.
    pub fn apply(&mut self, form: &CharacterForm) -> Result<usize, SessionError> {
        let Some(base) = self.character.as_ref() else {
            return Err(SessionError::NoSavegame);
        };
        let character = match form.to_character(base) {
            Ok(character) => character,
            Err(error) => {
                self.message = Message::error(error.to_string());
                return Err(SessionError::InvalidForm(error.to_string()));
            }
        };
        let Some(save) = self.save.as_mut() else {
            self.message = Message::error("no savegame is open");
            return Err(SessionError::NoSavegame);
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
                Err(SessionError::Save(error.to_string()))
            }
        }
    }

    /// Applies the form to the in-memory savegame without a message.
    ///
    /// Used by the shell to apply character edits as soon as they are valid, so
    /// every edit behaves like the item/style/gem edits. Invalid input is left
    /// in the form for the user to fix and reported by Save.
    pub fn apply_form_silently(&mut self, form: &CharacterForm) -> bool {
        let Some(base) = self.character.as_ref() else {
            return false;
        };
        let Ok(character) = form.to_character(base) else {
            return false;
        };
        if self.character.as_ref() == Some(&character) {
            return false;
        }
        let Some(save) = self.save.as_mut() else {
            return false;
        };
        match save.apply_character(&character) {
            Ok(changes) if !changes.is_empty() => {
                self.character = Some(character);
                self.refresh_entry();
                true
            }
            _ => false,
        }
    }

    /// Writes the opened savegame back to disk.
    pub fn save(&mut self) -> Result<PathBuf, SessionError> {
        let Some(save) = self.save.as_mut() else {
            return Err(SessionError::NoSavegame);
        };
        if !save.is_modified() {
            self.message = Message::info("Nothing was changed, not writing a file.");
            return Err(SessionError::NothingToSave);
        }
        match save.save(SaveOptions { backup: true }) {
            Ok(path) => {
                self.file_size = std::fs::metadata(&path).ok().map(|meta| meta.len());
                self.message = Message::success(format!("Saved {}.", path.display()));
                self.refresh_entry();
                Ok(path)
            }
            Err(error) => {
                self.message = Message::error(format!("Cannot save: {error}"));
                Err(SessionError::Save(error.to_string()))
            }
        }
    }

    /// Discards all changes by reloading the file.
    pub fn revert(&mut self) -> Result<(), SessionError> {
        let Some(index) = self.selected else {
            return Err(SessionError::NoSavegame);
        };
        if self.load(index) {
            self.message = Message::info("Reloaded the savegame from disk.");
            Ok(())
        } else {
            Err(SessionError::CannotReload)
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
                if let Some(item) = Rc::make_mut(&mut self.items)
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
    pub fn add_item(&mut self, item_id: u16) -> Result<(), SessionError> {
        let bundled = self.bundled_item(item_id);
        let (name, save_type) =
            if let Some(info) = self.game.as_ref().and_then(|game| game.item(item_id)) {
                let Some(save_type) = info.save_type else {
                    let error = format!("item {item_id} has no savegame category type");
                    self.message = Message::error(error.clone());
                    return Err(SessionError::Item(error));
                };
                (info.name.clone(), save_type)
            } else if let Some(item) = bundled {
                let Some(save_type) = item.save_type() else {
                    let error = format!("item {item_id} has no savegame category type");
                    self.message = Message::error(error.clone());
                    return Err(SessionError::Item(error));
                };
                (item.name.clone(), save_type)
            } else {
                // The CLI and older fixtures can add an id that is not in the
                // current item table; the legacy generic type keeps that working.
                (format!("Item {item_id}"), 8)
            };
        let Some(save) = self.save.as_mut() else {
            return Err(SessionError::NoSavegame);
        };
        match save.add_item(item_id, save_type, 1) {
            Ok(_) => {
                self.rebuild_rows();
                self.message = Message::success(format!("Added {name}."));
                Ok(())
            }
            Err(error) => {
                self.message = Message::error(format!("Cannot add item: {error}"));
                Err(SessionError::Item(error.to_string()))
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
                Rc::make_mut(&mut self.items).retain(|item| item.struct_index != struct_index);
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
        if let Some(game) = self.game.as_ref() {
            let matches: Vec<&jadeforge_gamedata::ItemInfo> = if query.trim().is_empty() {
                let mut all: Vec<(&jadeforge_gamedata::ItemInfo, String)> = game
                    .playable_items()
                    .filter(|item| item.save_type.is_some())
                    .map(|item| (item, item.name.to_lowercase()))
                    .collect();
                all.sort_by(|(left, left_name), (right, right_name)| {
                    left_name
                        .cmp(right_name)
                        .then_with(|| left.id.cmp(&right.id))
                });
                all.into_iter().map(|(item, _)| item).collect()
            } else {
                game.search_items(query, usize::MAX)
                    .into_iter()
                    .filter(|item| item.save_type.is_some())
                    .collect()
            };
            let total = matches.len();
            let rows: Vec<ItemSuggestion> = matches
                .into_iter()
                .take(limit)
                .filter_map(|item| {
                    Some(ItemSuggestion {
                        item_id: item.id,
                        name: item.name.clone(),
                        category: item.category.clone().unwrap_or_else(|| "item".to_owned()),
                        save_type: item.save_type?,
                        description: item.description.clone(),
                    })
                })
                .collect();
            return (rows, total);
        }

        let Some(names) = self.name_index else {
            return (Vec::new(), 0);
        };
        let (matches, total) = names.search_items(query, limit);
        let rows = matches
            .into_iter()
            .filter_map(|item| {
                Some(ItemSuggestion {
                    item_id: item.id,
                    name: item.name.clone(),
                    category: item.category.clone().unwrap_or_else(|| "item".to_owned()),
                    save_type: item.save_type()?,
                    description: None,
                })
            })
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
        let family_slots: Vec<u16> = style
            .upgrades
            .iter()
            .filter(|candidate| candidate.family == clicked.family)
            .map(|candidate| candidate.slot)
            .collect();
        let Some(save) = self.save.as_mut() else {
            return false;
        };
        match save.set_style_family_slot(struct_index, &family_slots, slot) {
            Ok(changed) => {
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
            Err(error) => {
                self.rebuild_rows();
                self.message = Message::error(format!("Cannot change the upgrade: {error}"));
                false
            }
        }
    }

    fn build_gem_rows(&mut self) -> (Vec<GemRow>, Vec<String>) {
        let Some(save) = self.save.as_ref() else {
            return (Vec::new(), Vec::new());
        };
        let mut warnings = Vec::new();
        let gems = match save.gems() {
            Ok(gems) => gems,
            Err(error) => {
                warnings.push(format!("gems: {error}"));
                Vec::new()
            }
        };
        let amulet = match save.amulet() {
            Ok(amulet) => amulet,
            Err(error) => {
                warnings.push(format!("amulet: {error}"));
                None
            }
        };
        self.amulet = amulet;
        let mut seen_amulet = 0_usize;
        let rows: Vec<GemRow> = gems
            .into_iter()
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
                let bundled = self.bundled_item(gem.item_id);
                let empty = info.is_none() && bundled.is_none() && gem.effects.is_empty();
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
                let name = info
                    .map(|item| item.name.clone())
                    .or_else(|| bundled.map(|item| item.name.clone()))
                    .unwrap_or_else(|| {
                        if empty {
                            "Empty socket".to_owned()
                        } else {
                            format!("Gem {}", gem.item_id)
                        }
                    });
                GemRow {
                    struct_index: gem.struct_index,
                    name,
                    effects,
                    empty,
                    location: gem.location,
                }
            })
            .collect();
        (rows, warnings)
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
                self.gems = Rc::new(self.build_gem_rows().0);
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
                self.gems = Rc::new(self.build_gem_rows().0);
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
                self.gems = Rc::new(self.build_gem_rows().0);
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
        let Some(info) = self
            .game
            .as_ref()
            .and_then(|game| game.style(style_id))
            .cloned()
        else {
            self.message = Message::error("Unknown style.");
            return;
        };
        let Some(template) = self.styles.first().map(|style| style.struct_index) else {
            self.message =
                Message::error("Learn a style first so JadeForge can copy the field layout.");
            return;
        };
        let identity = StyleIdentity {
            style_id,
            name_ref: info.name_ref,
            desc_ref: info.desc_ref,
            icon: info.icon.clone(),
            weapon: info.weapon.clone(),
            weapon_count: info.weapon_count,
        };
        let Some(save) = self.save.as_mut() else {
            return;
        };
        match save.add_style(template, &identity) {
            Ok(_) => {
                self.rebuild_rows();
                self.message = Message::success(format!("Added {}.", info.name));
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
        if self.styles.len() <= 1 {
            self.message = Message::error(
                "Keep at least one style: JadeForge needs an existing style as the layout template for learning one.",
            );
            return;
        }
        let Some(save) = self.save.as_mut() else {
            return;
        };
        match save.remove_style(struct_index) {
            Ok(true) => {
                Rc::make_mut(&mut self.styles).retain(|style| style.struct_index != struct_index);
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
                if let Some(henchman) = Rc::make_mut(&mut self.henchmen)
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
                if let Some(henchman) = Rc::make_mut(&mut self.henchmen)
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

    /// Reports a successful action the session did not perform itself.
    ///
    /// The GUI uses it for things like copying the save path, so the feedback
    /// lands in the same persistent status area as every other action.
    pub fn notify(&mut self, text: &str) {
        self.message = Message::success(text.to_owned());
    }

    /// Refreshes the summary of the selected entry after an edit.
    fn refresh_entry(&mut self) {
        let Some(index) = self.selected else {
            return;
        };
        let Some(character) = self.character.as_ref() else {
            return;
        };
        if let Some(entry) = Rc::make_mut(&mut self.entries).get_mut(index) {
            entry.character = Some(character.name.clone());
            entry.level = Some(character.level);
            entry.time_played = character.time_played;
        }
    }
}

fn save_folders(directory: &Path) -> Vec<PathBuf> {
    if directory.join("save.sac").is_file() {
        return vec![directory.to_path_buf()];
    }
    let mut candidates: Vec<PathBuf> = std::fs::read_dir(directory)
        .into_iter()
        .flatten()
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.join("save.sac").is_file())
        .collect();
    candidates.sort();
    candidates
}

fn describe(folder: &Path) -> SaveEntry {
    let label = folder.file_name().map_or_else(
        || folder.display().to_string(),
        |name| name.to_string_lossy().into(),
    );
    let path = folder.join("save.sac");
    match SaveGame::metadata(&path) {
        Ok(metadata) => SaveEntry {
            path: folder.to_path_buf(),
            label,
            character: Some(metadata.name),
            level: Some(metadata.level),
            time_played: metadata.time_played,
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
#[allow(clippy::assert_is_empty)] // empty checks read better than an empty-slice comparison
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;

    use jadeforge_core::{Gff, Value};

    struct TempDir {
        path: PathBuf,
    }

    impl TempDir {
        fn new(tag: &str) -> Self {
            let stamp = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_or(0, |duration| duration.as_nanos());
            let path = std::env::temp_dir().join(format!(
                "jadeforge-session-{tag}-{}-{stamp}",
                std::process::id()
            ));
            std::fs::create_dir_all(&path).expect("create temp directory");
            Self { path }
        }

        fn path(&self) -> &Path {
            &self.path
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.path);
        }
    }

    fn add(gff: &mut Gff, structure: usize, label: &str, value: Value) {
        gff.add_field(structure, label, value).expect("field");
    }

    #[allow(clippy::too_many_lines)] // one line per field reads best
    fn fixture_gff() -> Gff {
        let mut gff = Gff {
            version: *b"V3.2",
            labels: Vec::new(),
            structs: Vec::new(),
            root: 0,
        };
        let root = gff.push_struct(u32::MAX);
        let player = gff.push_struct(0);
        let stats = gff.push_struct(0);
        let focus = gff.push_struct(0);
        let progress = gff.push_struct(0);
        let campaign = gff.push_struct(0);
        let item = gff.push_struct(0);
        let style = gff.push_struct(0);
        let henchmen = gff.push_struct(0);
        let hench = gff.push_struct(0);
        let gem = gff.push_struct(0);
        let effect = gff.push_struct(0);
        let gem_entry = gff.push_struct(0);
        let amulet = gff.push_struct(0);

        add(&mut gff, root, "PlayerCreature", Value::Struct(player));
        add(&mut gff, root, "Campaign", Value::Struct(campaign));
        add(&mut gff, campaign, "Chapter", Value::DWord(2));
        add(
            &mut gff,
            campaign,
            "TimePlayed",
            Value::DWord64(90_000_000_000),
        );
        add(
            &mut gff,
            player,
            "FirstName",
            Value::String("Test Hero".to_owned()),
        );
        add(&mut gff, player, "Level", Value::Byte(7));
        add(&mut gff, player, "CurrentXP", Value::DWord(1234));
        add(&mut gff, player, "Stats", Value::Struct(stats));
        add(&mut gff, player, "Progress", Value::Struct(progress));
        add(&mut gff, stats, "Gold", Value::DWord(99));
        add(&mut gff, stats, "UnassignedPts", Value::DWord(2));
        add(&mut gff, stats, "Health", Value::Float(120.0));
        add(&mut gff, stats, "HealthBase", Value::DWord(150));
        add(&mut gff, stats, "Chi", Value::Float(80.0));
        add(&mut gff, stats, "ChiBase", Value::DWord(100));
        add(&mut gff, stats, "Focus", Value::Struct(focus));
        add(&mut gff, focus, "STAT_CUR", Value::Float(20.0));
        add(&mut gff, focus, "STAT_MAX", Value::Float(30.0));
        add(&mut gff, stats, "Body", Value::DWord(5));
        add(&mut gff, stats, "Mind", Value::DWord(6));
        add(&mut gff, stats, "Spirit", Value::DWord(7));
        add(&mut gff, stats, "Charm", Value::Int(1));
        add(&mut gff, stats, "Intuition", Value::Int(2));
        add(&mut gff, stats, "Intimidate", Value::Int(3));
        add(&mut gff, stats, "Alignment", Value::Int(500));
        add(&mut gff, stats, "Appearance", Value::Int(4));
        add(&mut gff, progress, "Kills", Value::DWord(42));

        add(&mut gff, player, "ItemList", Value::List(vec![item]));
        add(&mut gff, item, "Count", Value::Byte(5));
        add(&mut gff, item, "ItemId", Value::Word(42));
        add(&mut gff, item, "Type", Value::Byte(6));

        add(&mut gff, stats, "Styles", Value::List(vec![style]));
        add(&mut gff, style, "StyleId", Value::Word(21));
        add(&mut gff, style, "StyleLevel", Value::Byte(1));
        add(&mut gff, style, "StyleLevelInfo", Value::DWord64(0));
        add(&mut gff, style, "NameRef", Value::DWord(7));
        add(&mut gff, style, "DescRef", Value::DWord(8));
        add(&mut gff, style, "IconRef", Value::ResRef("icon".to_owned()));
        add(&mut gff, style, "TimeSpentInStyle", Value::Float(0.0));
        add(&mut gff, style, "FirstLoad", Value::Byte(1));

        add(&mut gff, player, "Henchmen", Value::Struct(henchmen));
        add(&mut gff, henchmen, "HenchList", Value::List(vec![hench]));
        add(
            &mut gff,
            hench,
            "BaseResref",
            Value::String("j00h_ho".to_owned()),
        );
        add(&mut gff, hench, "Level", Value::Byte(3));
        add(&mut gff, hench, "Available", Value::Byte(1));
        add(&mut gff, hench, "Available2", Value::Byte(1));
        add(&mut gff, hench, "State", Value::Byte(0));
        add(&mut gff, hench, "CombatTime", Value::Float(0.0));

        add(&mut gff, player, "Gems", Value::List(vec![gem_entry]));
        add(&mut gff, gem_entry, "Gem", Value::Struct(gem));
        add(&mut gff, gem, "ItemID", Value::Word(77));
        add(&mut gff, gem, "GemsEffectList", Value::List(vec![effect]));
        add(&mut gff, effect, "GemsEffectType", Value::DWord(62));
        add(&mut gff, effect, "GemsEffectValue", Value::DWord(2));
        add(&mut gff, effect, "GemsEffectCurVal", Value::DWord(2));
        add(&mut gff, effect, "GemsEffectDurati", Value::Float(0.0));

        add(&mut gff, player, "Amulet", Value::Struct(amulet));
        add(&mut gff, amulet, "Type", Value::Byte(2));
        let slots: Vec<usize> = (0..7).map(|_| gff.push_struct(0)).collect();
        add(&mut gff, amulet, "OpenGemsList", Value::List(slots));
        gff
    }

    fn container(gff: &Gff) -> Vec<u8> {
        let block = gff.to_bytes().expect("serialize block");
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&1_u32.to_le_bytes());
        bytes.extend_from_slice(&9_u32.to_le_bytes());
        bytes.extend_from_slice(b"save.sav\0");
        bytes.extend_from_slice(
            &u32::try_from(block.len())
                .expect("block size")
                .to_le_bytes(),
        );
        bytes.extend_from_slice(&block);
        bytes
    }

    fn fixture() -> (TempDir, Session) {
        let directory = TempDir::new("fixture");
        fs::write(directory.path().join("save.sac"), container(&fixture_gff()))
            .expect("write savegame");
        let mut session = Session::with_directory(directory.path());
        assert_eq!(session.entries.len(), 1, "{}", session.message.text);
        assert!(session.open(0), "{}", session.message.text);
        (directory, session)
    }

    fn form_for(session: &Session) -> CharacterForm {
        CharacterForm::from_character(session.character.as_ref().expect("character"))
    }

    fn set_field(session: &mut Session, path: &str, value: Value) {
        session
            .save
            .as_mut()
            .expect("open savegame")
            .block_mut("save.sav")
            .expect("save block")
            .gff_mut()
            .get_mut(path)
            .expect("field")
            .value = value;
    }

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
    fn form_preserves_a_missing_gold_field() {
        let mut character = character();
        character.gold = None;
        let form = CharacterForm::from_character(&character);
        assert!(
            form.gold.is_empty(),
            "a missing gold field must stay empty, not become zero"
        );
        let parsed = form.to_character(&character).expect("parse");
        assert_eq!(parsed, character);
        // The GUI derives its invalid badge from this comparison. A missing
        // gold field used to make every untouched Gold-less save look invalid.
        assert_eq!(parsed.gold, None);
    }

    #[test]
    fn form_can_create_a_missing_gold_field() {
        let mut character = character();
        character.gold = None;
        let mut form = CharacterForm::from_character(&character);
        form.gold = "7".to_owned();
        let parsed = form.to_character(&character).expect("parse");
        assert_eq!(parsed.gold, Some(7));
    }

    #[test]
    fn form_rejects_clearing_an_existing_gold_field() {
        let character = character();
        let mut form = CharacterForm::from_character(&character);
        form.gold.clear();
        let error = form.to_character(&character).expect_err("cleared gold");
        assert!(error.contains("gold"), "{error}");
    }

    #[test]
    fn empty_session_has_a_message() {
        let session = Session::with_directory("/definitely/not/a/save/directory");
        assert_eq!(session.entries.len(), 0, "no savegames expected here");
        assert!(session.message.text.contains("No savegames"));
    }

    #[test]
    fn a_missing_installation_reports_bundled_names() {
        let session = Session::empty("/definitely/not/a/save/directory");
        if session.game.is_some() {
            // A developer has a game installation configured; the bundled
            // branch needs no game, so there is nothing to assert here.
            return;
        }
        assert!(session.has_bundled_names());
        assert!(matches!(
            session.game_status(),
            GameStatus::NamesOnly { .. }
        ));
        assert!(session.items_ready());
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
        assert_eq!(format::elapsed(12), "12s");
        assert_eq!(format::elapsed(60), "1m");
        assert_eq!(format::elapsed(3439), "57m");
    }
    #[test]
    fn background_scan_results_can_be_applied() {
        let directory =
            std::env::temp_dir().join(format!("jadeforge-empty-session-{}", std::process::id()));
        std::fs::create_dir_all(&directory).expect("temp dir");
        let mut session = Session::empty(&directory);
        assert_eq!(session.entries.len(), 0);
        let entries = Session::scan_entries(&directory);
        assert_eq!(entries.len(), 0);
        session.apply_entries(entries);
        assert!(session.message.text.contains("No savegames found"));
        session.report_scan_failure();
        assert!(session.message.text.contains("Could not scan"));
        let _ = std::fs::remove_dir_all(directory);
    }

    #[test]
    fn missing_game_data_still_describes_savegame_fields() {
        let mut session = Session::empty("/definitely/not/a/save");
        session.game = None;
        session.name_index = None;

        let item = Item {
            struct_index: 0,
            item_id: 42,
            count: 2,
            kind: 6,
        };
        let row = session.describe_item(&item);
        assert_eq!(row.name, "Item 42");
        assert_eq!(row.category.as_deref(), Some("plot_items"));

        let style = Style {
            struct_index: 1,
            style_id: 21,
            level: 13,
            mask: 0,
            name_ref: None,
            desc_ref: None,
            icon: Some("I_TCUTS".to_owned()),
            weapon: Some("J00_COLLISION_01".to_owned()),
            weapon_count: 2,
            time_spent: 3439.2458,
        };
        let row = session.describe_style(&style);
        assert_eq!(row.name, "Style 21");
        assert_eq!(row.kind, "weapon");
        assert_eq!(row.icon.as_deref(), Some("I_TCUTS"));
        assert_eq!(row.weapon.as_deref(), Some("J00_COLLISION_01"));
        assert_eq!(row.weapon_count, 2);
        assert_eq!(row.time_spent, 3439);
        assert!(row.label.is_empty());
    }

    #[test]
    fn bundled_english_names_fill_rows_and_the_item_picker() {
        let (_directory, mut session) = fixture();
        session.game = None;
        session.name_index = NameIndex::bundled_english();
        session.rebuild_rows();
        assert!(session.has_bundled_names());

        let item = session
            .items
            .iter()
            .find(|item| item.item_id == 42)
            .expect("fixture item");
        assert_eq!(item.name, "The Bronze Tongue");
        // The savegame type is authoritative for an owned item, even when the
        // bundled table would classify the same id differently.
        assert_eq!(item.category.as_deref(), Some("plot_items"));

        let weapon = session.describe_item(&Item {
            struct_index: 2,
            item_id: 369,
            count: 1,
            kind: 5,
        });
        assert_eq!(weapon.name, "Flawless");
        assert_eq!(weapon.category.as_deref(), Some("weapons"));

        let style = session
            .styles
            .iter()
            .find(|style| style.style_id == 21)
            .expect("fixture style");
        assert_eq!(style.name, "Thousand Cuts");
        assert!(style.label.is_empty());

        assert!(session.items_ready());
        assert!(session.item_database_count() > 100);
        let (matches, total) = session.search_items("bronze", 10);
        assert!(total >= 1);
        assert!(matches.iter().any(|item| item.item_id == 42));
        assert_eq!(matches[0].save_type, 1);

        session.add_item(369).expect("add a bundled item");
        let added = session
            .items
            .iter()
            .find(|item| item.item_id == 369)
            .expect("added item");
        assert_eq!(added.name, "Flawless");
        assert_eq!(added.kind, 5);
    }

    #[test]
    fn opening_a_fixture_exposes_rows_and_file_state() {
        let (directory, session) = fixture();
        assert_eq!(
            session.current_path(),
            Some(directory.path().join("save.sac"))
        );
        assert_eq!(session.block_count(), 1);
        assert_eq!(
            session.file_size(),
            Some(
                std::fs::metadata(directory.path().join("save.sac"))
                    .expect("metadata")
                    .len()
            )
        );
        let character = session.character.as_ref().expect("character");
        assert_eq!(character.name, "Test Hero");
        assert_eq!(character.level, 7);
        assert_eq!(character.time_played, 90_000_000_000);
        assert_eq!(session.items.len(), 1);
        assert_eq!(session.items[0].count, 5);
        assert_eq!(session.styles.len(), 1);
        assert_eq!(session.styles[0].level, 1);
        assert_eq!(session.henchmen.len(), 1);
        assert_eq!(session.henchmen[0].name, "Henpecked Hou");
        assert_eq!(session.gems.len(), 6);
        assert!(
            session
                .gems
                .iter()
                .any(|gem| !gem.empty && gem.location == GemLocation::Inventory)
        );
        assert_eq!(session.gems.iter().filter(|gem| gem.empty).count(), 5);
        assert_eq!(session.amulet.expect("amulet").sockets, 5);
        assert!(!session.is_dirty());
        assert_eq!(session.message.kind, MessageKind::Success);
    }

    #[test]
    fn save_writes_a_backup_and_revert_reloads_from_disk() {
        let (directory, mut session) = fixture();
        let mut form = form_for(&session);
        form.level = "9".to_owned();
        assert!(session.apply(&form).expect("apply") >= 1);
        assert!(session.is_dirty());

        let path = session.save().expect("save");
        assert_eq!(path, directory.path().join("save.sac"));
        assert!(!session.is_dirty());
        assert!(
            session.message.text.contains("Saved"),
            "{}",
            session.message.text
        );
        let backups = fs::read_dir(directory.path())
            .expect("directory")
            .flatten()
            .filter(|entry| {
                entry
                    .file_name()
                    .to_string_lossy()
                    .contains("jadeforge-backup")
            })
            .count();
        assert_eq!(backups, 1);
        assert_eq!(
            SaveGame::open(&path)
                .expect("reopen")
                .character()
                .expect("character")
                .level,
            9
        );

        let mut form = form_for(&session);
        form.level = "10".to_owned();
        assert!(session.apply(&form).expect("apply") >= 1);
        session.revert().expect("revert");
        assert_eq!(session.character.as_ref().expect("character").level, 9);
        assert!(!session.is_dirty());
        assert!(session.message.text.contains("Reloaded"));
    }

    #[test]
    fn save_without_changes_and_without_a_savegame() {
        let (directory, mut session) = fixture();
        let error = session.save().expect_err("nothing to save");
        assert!(error.contains("nothing to save"), "{error}");
        assert!(session.message.text.contains("Nothing was changed"));
        assert!(!session.is_dirty());

        let mut empty = Session::empty(directory.path());
        assert!(empty.save().is_err());
        assert!(empty.revert().is_err());
    }

    #[test]
    fn apply_and_apply_form_silently_cover_their_early_returns() {
        let (directory, mut session) = fixture();
        let form = form_for(&session);
        assert_eq!(session.apply(&form).expect("apply unchanged"), 0);
        assert!(
            session.message.text.contains("Nothing to apply"),
            "{}",
            session.message.text
        );
        assert!(!session.is_dirty());

        let mut invalid = form.clone();
        invalid.level = "not a number".to_owned();
        let error = session.apply(&invalid).expect_err("invalid form");
        assert!(error.contains("level"), "{error}");
        assert!(!session.is_dirty());
        assert!(!session.apply_form_silently(&invalid));

        let mut rejected = form.clone();
        rejected.level = "0".to_owned();
        let error = session.apply(&rejected).expect_err("rejected character");
        assert!(error.contains("level"), "{error}");
        assert_eq!(session.message.kind, MessageKind::Error);
        assert!(!session.is_dirty());
        assert!(!session.apply_form_silently(&rejected));

        let mut changed = form.clone();
        changed.level = "10".to_owned();
        let message = session.message.text.clone();
        assert!(session.apply_form_silently(&changed));
        assert!(session.is_dirty());
        assert_eq!(session.message.text, message);

        let mut empty = Session::empty(directory.path());
        assert!(empty.apply(&form).is_err());
        assert!(!empty.apply_form_silently(&form));
    }

    #[test]
    fn item_count_and_removal_actions_cover_all_branches() {
        let (directory, mut session) = fixture();
        let index = session.items[0].struct_index;
        session.adjust_item_count(index, 1);
        assert_eq!(session.items[0].count, 6);
        assert!(!session.items[0].name.is_empty());
        assert!(
            session.message.text.contains("6 in the stack"),
            "{}",
            session.message.text
        );

        session.adjust_item_count(index, -100);
        assert!(session.message.text.contains("at least one"));
        assert_eq!(session.items[0].count, 6);
        session.adjust_item_count(index, 300);
        assert!(session.message.text.contains("more than 255"));
        assert_eq!(session.items[0].count, 6);
        let message = session.message.text.clone();
        session.adjust_item_count(999, 1);
        assert_eq!(session.message.text, message);

        session.remove_item(index);
        assert!(session.items.is_empty());
        assert!(session.message.text.contains("Removed"));

        let (_, mut session) = fixture();
        let index = session.items[0].struct_index;
        session
            .save
            .as_mut()
            .expect("savegame")
            .remove_item(index)
            .expect("remove from save only");
        let rows = session.items.len();
        session.remove_item(index);
        assert_eq!(session.items.len(), rows);
        assert!(session.message.text.contains("already gone"));

        let (_, mut session) = fixture();
        let index = session.items[0].struct_index;
        set_field(&mut session, "PlayerCreature.ItemList", Value::Byte(1));
        session.remove_item(index);
        assert!(session.message.text.contains("Cannot remove item"));

        let (_, mut session) = fixture();
        let before = session.items.len();
        session.add_item(1234).expect("add item");
        assert_eq!(session.items.len(), before + 1);
        assert!(session.message.text.contains("Added"));
        set_field(&mut session, "PlayerCreature.ItemList", Value::Byte(1));
        assert!(session.add_item(5).is_err());
        assert!(session.message.text.contains("Cannot add item"));

        let mut empty = Session::empty(directory.path());
        let message = empty.message.text.clone();
        empty.remove_item(0);
        assert_eq!(empty.message.text, message);
        let error = empty.add_item(1).expect_err("add without a savegame");
        assert!(error.contains("no savegame"), "{error}");
    }

    #[test]
    fn gem_actions_move_remove_and_report_errors() {
        let (directory, mut session) = fixture();
        let index = session.gems[0].struct_index;
        assert_eq!(session.gems[0].location, GemLocation::Inventory);
        session.move_gem(index);
        assert_eq!(session.gems[0].location, GemLocation::Amulet);
        assert!(
            session.message.text.contains("socketed"),
            "{}",
            session.message.text
        );
        session.move_gem(index);
        assert_eq!(session.gems[0].location, GemLocation::Inventory);
        assert!(
            session.message.text.contains("inventory"),
            "{}",
            session.message.text
        );
        session.remove_gem(index);
        assert!(
            session.gems.iter().all(|row| row.empty),
            "the amulet placeholders remain after removing the only gem"
        );
        assert!(session.message.text.contains("Gem removed"));

        let (_, mut session) = fixture();
        let rows = session.gems.len();
        session.remove_gem(9999);
        assert_eq!(session.gems.len(), rows);
        assert!(session.message.text.contains("Cannot remove the gem"));

        let (_, mut session) = fixture();
        let rows = session.gems.len();
        session.add_gem(u16::MAX);
        assert_eq!(session.gems.len(), rows);
        assert!(session.message.text.contains("not in the game data"));

        let mut empty = Session::empty(directory.path());
        empty.move_gem(0);
        empty.remove_gem(0);
    }

    /// Adds a second row to the in-memory style list.
    ///
    /// The real save fixture has one style; the fake row is only there to get
    /// past the "keep at least one" guard so the removal and error paths can be
    /// exercised. The save and the row list intentionally diverge here.
    fn add_fake_second_style(session: &mut Session) -> usize {
        let real = session.styles[0].struct_index;
        let mut fake = session.styles[0].clone();
        fake.struct_index = usize::MAX;
        fake.style_id = 22;
        fake.name = "Other Style".to_owned();
        Rc::make_mut(&mut session.styles).push(fake);
        assert_eq!(session.styles.len(), 2);
        real
    }

    #[test]
    fn style_actions_keep_the_last_style_and_report_errors() {
        let (directory, mut session) = fixture();
        let index = session.styles[0].struct_index;
        let before = session.styles.len();
        session.remove_style(index);
        assert_eq!(session.styles.len(), before, "the last style was removed");
        assert!(session.message.text.contains("at least one"));

        let (_, mut session) = fixture();
        let index = add_fake_second_style(&mut session);
        session.remove_style(index);
        assert_eq!(session.styles.len(), 1);
        assert!(session.message.text.contains("Removed"));

        let (_, mut session) = fixture();
        let index = add_fake_second_style(&mut session);
        session
            .save
            .as_mut()
            .expect("savegame")
            .remove_style(index)
            .expect("remove from save only");
        let rows = session.styles.len();
        session.remove_style(index);
        assert_eq!(session.styles.len(), rows);
        assert!(session.message.text.contains("already gone"));

        let (_, mut session) = fixture();
        let index = add_fake_second_style(&mut session);
        set_field(&mut session, "PlayerCreature.Stats.Styles", Value::Byte(1));
        session.remove_style(index);
        assert!(session.message.text.contains("Cannot remove style"));
        assert_eq!(session.styles.len(), 2);

        let (_, mut session) = fixture();
        session.add_style(u16::MAX);
        assert_eq!(session.styles.len(), 1);
        assert!(session.message.text.contains("Unknown style"));

        let mut empty = Session::empty(directory.path());
        empty.remove_style(0);
        empty.add_style(u16::MAX);
    }

    #[test]
    fn henchman_actions_cover_bounds_and_toggle() {
        let (_, mut session) = fixture();
        let index = session.henchmen[0].struct_index;
        session.set_henchman_level(index, 12);
        assert_eq!(session.henchmen[0].level, 12);
        assert!(
            session.message.text.contains("level set to 12"),
            "{}",
            session.message.text
        );

        session.set_henchman_level(index, 0);
        assert!(session.message.text.contains("1 to 60"));
        assert_eq!(session.henchmen[0].level, 12);
        session.set_henchman_level(index, 61);
        assert!(session.message.text.contains("1 to 60"));
        session.set_henchman_level(999, 12);
        assert!(session.message.text.contains("Cannot change companion"));

        assert!(session.henchmen[0].available);
        session.toggle_henchman_available(index);
        assert!(!session.henchmen[0].available);
        assert!(
            session.message.text.contains("unavailable"),
            "{}",
            session.message.text
        );
        session.toggle_henchman_available(index);
        assert!(session.henchmen[0].available);
        assert!(session.message.text.contains("available"));
        let message = session.message.text.clone();
        session.toggle_henchman_available(999);
        assert_eq!(session.message.text, message);

        let mut empty = Session::empty(std::env::temp_dir());
        empty.set_henchman_level(0, 12);
        empty.toggle_henchman_available(0);
    }

    #[test]
    fn set_error_changes_the_message_kind() {
        let (_, mut session) = fixture();
        session.set_error("broken value");
        assert_eq!(session.message.kind, MessageKind::Error);
        assert_eq!(session.message.text, "broken value");
    }

    #[test]
    fn set_directory_protects_unsaved_changes_and_scans_the_new_folder() {
        crate::config::ensure_test_path();
        let (directory, mut session) = fixture();
        let other = TempDir::new("other");
        let mut form = form_for(&session);
        form.level = "9".to_owned();
        session.apply(&form).expect("apply");
        let original = session.directory.clone();

        session.set_directory(other.path());
        assert_eq!(session.directory, original);
        assert!(
            session.message.text.contains("unsaved"),
            "{}",
            session.message.text
        );
        assert!(session.is_dirty());

        session.revert().expect("revert");
        session.set_directory(other.path());
        assert_eq!(session.directory, other.path().to_path_buf());
        assert!(session.selected.is_none());
        assert!(session.save.is_none());
        assert!(
            session.message.text.contains("No savegames"),
            "{}",
            session.message.text
        );
        assert!(directory.path().exists());
    }
    #[test]
    fn opening_the_already_selected_save_keeps_unsaved_edits() {
        let (_, mut session) = fixture();
        let index = session.items[0].struct_index;
        let before = session.items[0].count;
        session.adjust_item_count(index, 1);
        assert!(session.is_dirty());
        let edited = session.items[0].count;

        assert!(session.open(0), "{}", session.message.text);
        assert!(session.is_dirty());
        assert_eq!(session.items[0].count, edited);
        assert_ne!(edited, before);
    }

    #[test]
    fn stale_scan_results_are_discarded() {
        let (directory, mut session) = fixture();
        let token = session.scan_token();
        let entries = Session::scan_entries(directory.path());
        session.scan();
        assert!(!session.apply_entries_for(directory.path(), token, entries));
        assert!(!session.report_scan_failure_for(directory.path(), token));
    }

    #[test]
    fn opening_a_save_with_a_broken_section_reports_it() {
        let directory = TempDir::new("partial");
        let mut gff = fixture_gff();
        let field = gff.get_mut("PlayerCreature.ItemList").expect("item list");
        field.type_id = 0;
        field.value = Value::Byte(1);
        fs::write(directory.path().join("save.sac"), container(&gff)).expect("write save");
        let mut session = Session::with_directory(directory.path());
        assert!(session.open(0), "{}", session.message.text);
        assert_eq!(session.message.kind, MessageKind::Error);
        assert!(
            session.message.text.contains("items"),
            "{}",
            session.message.text
        );
        assert!(session.character.is_some());
        assert!(session.items.is_empty());
    }

    #[test]
    fn switching_directories_clears_gem_and_amulet_state() {
        let (_, mut session) = fixture();
        assert!(!session.gems.is_empty());
        assert!(session.amulet.is_some());
        let other = TempDir::new("other-empty");
        session.set_directory(other.path());
        assert!(session.gems.is_empty());
        assert!(session.amulet.is_none());
        assert!(session.save.is_none());
    }
}
