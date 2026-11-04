//! UI independent session state.
//!
//! Everything the desktop application does with a savegame goes through this
//! module, which keeps the Freya views thin and makes the behaviour testable
//! without a window.

use std::path::{Path, PathBuf};

use jadeforge_core::{Character, Item, SaveGame, SaveOptions};

/// Where Jade Empire stores its savegames on the current platform.
#[must_use]
pub fn default_save_dir() -> PathBuf {
    if let Some(directory) = std::env::var_os("JADEFORGE_SAVE_DIR") {
        return PathBuf::from(directory);
    }
    if cfg!(target_os = "linux") {
        PathBuf::from("/media/Games/Heroic/Jade Empire/Save")
    } else if cfg!(target_os = "windows") {
        PathBuf::from(r"C:\Program Files (x86)\GOG Galaxy\Games\Jade Empire\Save")
    } else {
        PathBuf::from("/Applications/Jade Empire/Save")
    }
}

/// One entry of the savegame list.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SaveEntry {
    /// Folder that contains `save.sac`.
    pub path: PathBuf,
    /// Name shown in the list.
    pub label: String,
    /// Short summary line, for example `Level 12 - 10h 09m`.
    pub summary: String,
    /// Set when the savegame could not be read while scanning.
    pub error: Option<String>,
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
    fn info(text: impl Into<String>) -> Self {
        Self {
            kind: MessageKind::Info,
            text: text.into(),
        }
    }

    fn success(text: impl Into<String>) -> Self {
        Self {
            kind: MessageKind::Success,
            text: text.into(),
        }
    }

    fn error(text: impl Into<String>) -> Self {
        Self {
            kind: MessageKind::Error,
            text: text.into(),
        }
    }
}

/// The editable values of the character form.
///
/// The UI keeps text, the session converts it into a [`Character`] and reports
/// readable errors for invalid input.
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
            health: format_number(character.health),
            health_max: character.health_max.to_string(),
            chi: format_number(character.chi),
            chi_max: character.chi_max.to_string(),
            focus: format_number(character.focus),
            focus_max: format_number(character.focus_max),
            body: character.body.to_string(),
            mind: character.mind.to_string(),
            spirit: character.spirit.to_string(),
            charm: character.charm.to_string(),
            intuition: character.intuition.to_string(),
            intimidate: character.intimidate.to_string(),
            alignment: character.alignment.to_string(),
            chapter: character.chapter.to_string(),
            kills: character.kills.to_string(),
        }
    }

    /// Builds a character by applying the form on top of a base character.
    ///
    /// Values that are not part of the form (play time, appearance, ...) are
    /// taken from the base.
    pub fn to_character(&self, base: &Character) -> Result<Character, String> {
        let mut character = base.clone();
        character.name.clone_from(&self.name);
        character.level = parse_u8("level", &self.level)?;
        character.experience = parse_u32("experience", &self.experience)?;
        character.gold = Some(parse_u32("gold", &self.gold)?);
        character.attribute_points = parse_u32("attribute points", &self.attribute_points)?;
        character.health = parse_f32("health", &self.health)?;
        character.health_max = parse_u32("maximum health", &self.health_max)?;
        character.chi = parse_f32("chi", &self.chi)?;
        character.chi_max = parse_u32("maximum chi", &self.chi_max)?;
        character.focus = parse_f32("focus", &self.focus)?;
        character.focus_max = parse_f32("maximum focus", &self.focus_max)?;
        character.body = parse_u32("body", &self.body)?;
        character.mind = parse_u32("mind", &self.mind)?;
        character.spirit = parse_u32("spirit", &self.spirit)?;
        character.charm = parse_i32("charm", &self.charm)?;
        character.intuition = parse_i32("intuition", &self.intuition)?;
        character.intimidate = parse_i32("intimidate", &self.intimidate)?;
        character.alignment = parse_i32("alignment", &self.alignment)?;
        character.chapter = parse_u32("chapter", &self.chapter)?;
        character.kills = parse_u32("kills", &self.kills)?;
        Ok(character)
    }
}

fn format_number(value: f32) -> String {
    if (value - value.round()).abs() < f32::EPSILON {
        format!("{}", value.round() as i64)
    } else {
        format!("{value}")
    }
}

fn parse_u32(what: &str, raw: &str) -> Result<u32, String> {
    raw.trim()
        .parse()
        .map_err(|_| format!("{what}: {:?} is not a whole number", raw.trim()))
}

fn parse_u8(what: &str, raw: &str) -> Result<u8, String> {
    raw.trim()
        .parse()
        .map_err(|_| format!("{what}: {:?} is not a number between 0 and 255", raw.trim()))
}

fn parse_i32(what: &str, raw: &str) -> Result<i32, String> {
    raw.trim()
        .parse()
        .map_err(|_| format!("{what}: {:?} is not a whole number", raw.trim()))
}

fn parse_f32(what: &str, raw: &str) -> Result<f32, String> {
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
    /// Items of the opened savegame.
    pub items: Vec<Item>,
    /// Status of the last action.
    pub message: Message,
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
        let directory = default_save_dir();
        let mut session = Self {
            entries: Vec::new(),
            directory,
            selected: None,
            save: None,
            character: None,
            items: Vec::new(),
            message: Message::info("Select a savegame to start editing."),
        };
        session.scan();
        session
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
            message: Message::info("Select a savegame to start editing."),
        };
        session.scan();
        session
    }

    /// Changes the directory and scans it.
    ///
    /// Refuses to throw away unsaved changes.
    pub fn set_directory(&mut self, directory: impl Into<PathBuf>) {
        if self.is_dirty() {
            self.message = Message::info(
                "This savegame has unsaved changes: save them or press Revert before scanning.",
            );
            return;
        }
        self.directory = directory.into();
        self.selected = None;
        self.save = None;
        self.character = None;
        self.items.clear();
        self.scan();
    }

    /// Scans the directory for savegames.
    pub fn scan(&mut self) {
        self.entries.clear();
        let directory = self.directory.clone();
        if directory.join("save.sac").is_file() {
            let entry = describe(&directory);
            self.entries.push(entry);
        } else if let Ok(read) = std::fs::read_dir(&directory) {
            let mut candidates: Vec<PathBuf> = read
                .flatten()
                .map(|entry| entry.path())
                .filter(|path| path.join("save.sac").is_file())
                .collect();
            candidates.sort();
            self.entries = candidates.iter().map(|path| describe(path)).collect();
        }
        if self.entries.is_empty() {
            self.message = Message::info(format!(
                "No savegames found in {}.",
                self.directory.display()
            ));
        } else {
            self.message = Message::info(format!(
                "Found {} savegame(s) in {}.",
                self.entries.len(),
                self.directory.display()
            ));
        }
    }

    /// Opens the savegame at `index` of the entry list.
    ///
    /// Refuses to switch away from a savegame with unsaved changes so that an
    /// accidental click cannot throw work away.
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
                let character = save.character().ok();
                let items = save.items().unwrap_or_default();
                let label = entry.label.clone();
                self.save = Some(save);
                self.character = character;
                self.items = items;
                self.selected = Some(index);
                self.message = Message::success(format!("Opened {label}."));
                true
            }
            Err(error) => {
                self.message = Message::error(format!("Cannot open {}: {error}", path.display()));
                false
            }
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
                self.items = save.items().unwrap_or_default();
                self.refresh_entry_summary();
                if changes.is_empty() {
                    self.message = Message::info("Nothing to apply.");
                } else {
                    self.message = Message::success(format!("Changed {} field(s).", changes.len()));
                }
                Ok(changes.len())
            }
            Err(error) => {
                self.message = Message::error(format!("Cannot apply changes: {error}"));
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
                self.message = Message::success(format!(
                    "Saved {} (backup created next to it).",
                    path.display()
                ));
                self.refresh_entry_summary();
                Ok(path)
            }
            Err(error) => {
                self.message = Message::error(format!("Cannot save: {error}"));
                Err(error.to_string())
            }
        }
    }

    /// Discards all changes by reopening the file.
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
        let Some(save) = self.save.as_mut() else {
            return;
        };
        let Some(current) = self
            .items
            .iter()
            .find(|item| item.struct_index == struct_index)
            .map(|item| (item.item_id, item.count))
        else {
            return;
        };
        let (item_id, current_count) = current;
        let new_count = i32::from(current_count) + delta;
        if !(1..=255).contains(&new_count) {
            return;
        }
        if let Err(error) = save.set_item_count(struct_index, new_count as u8) {
            self.message = Message::error(format!("Cannot change item: {error}"));
            return;
        }
        if let Some(item) = self
            .items
            .iter_mut()
            .find(|item| item.struct_index == struct_index)
        {
            item.count = new_count as u8;
        }
        self.message = Message::success(format!("Item {item_id} now has {new_count} pieces."));
    }

    /// Adds an item stack.
    pub fn add_item(&mut self, item_id: u16, kind: u8, count: u8) -> Result<(), String> {
        let Some(save) = self.save.as_mut() else {
            return Err("no savegame is open".to_owned());
        };
        match save.add_item(item_id, kind, count) {
            Ok(_) => {
                self.items = save.items().unwrap_or_default();
                self.message = Message::success(format!("Added item {item_id}."));
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
        let Some(save) = self.save.as_mut() else {
            return;
        };
        match save.remove_item(struct_index) {
            Ok(true) => {
                self.items.retain(|item| item.struct_index != struct_index);
                self.message = Message::success("Item removed.");
            }
            Ok(false) => {
                self.message = Message::info("The item was already gone.");
            }
            Err(error) => {
                self.message = Message::error(format!("Cannot remove item: {error}"));
            }
        }
    }

    /// Reports a validation problem in the status line.
    pub fn set_error(&mut self, text: &str) {
        self.message = Message::error(text);
    }

    /// Refreshes the summary line of the selected entry after an edit.
    fn refresh_entry_summary(&mut self) {
        let Some(index) = self.selected else {
            return;
        };
        let Some(character) = self.character.as_ref() else {
            return;
        };
        if let Some(entry) = self.entries.get_mut(index) {
            entry.summary = summary_for(character);
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
                summary: summary_for(&character),
                error: None,
            },
            Err(error) => SaveEntry {
                path: folder.to_path_buf(),
                label,
                summary: "no player character".to_owned(),
                error: Some(error.to_string()),
            },
        },
        Err(error) => SaveEntry {
            path: folder.to_path_buf(),
            label,
            summary: "cannot be read".to_owned(),
            error: Some(error.to_string()),
        },
    }
}

fn summary_for(character: &Character) -> String {
    let seconds = character.time_played / 1_000_000;
    let hours = seconds / 3600;
    let minutes = (seconds % 3600) / 60;
    format!(
        "Level {} - {} - {hours}h {minutes:02}m",
        character.level, character.name
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn form_round_trips_character() {
        let character = Character {
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
        };
        let form = CharacterForm::from_character(&character);
        let parsed = form.to_character(&character).expect("parse");
        assert_eq!(parsed, character);
    }

    #[test]
    fn form_reports_invalid_numbers() {
        let character = Character {
            name: String::new(),
            level: 1,
            experience: 0,
            gold: None,
            attribute_points: 0,
            health: 1.0,
            health_max: 1,
            chi: 1.0,
            chi_max: 1,
            focus: 1.0,
            focus_max: 1.0,
            body: 1,
            mind: 1,
            spirit: 1,
            charm: 0,
            intuition: 0,
            intimidate: 0,
            alignment: 0,
            appearance: 0,
            chapter: 0,
            time_played: 0,
            kills: 0,
        };
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
}
