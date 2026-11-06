//! UI independent session state.
//!
//! Everything the desktop application does with a savegame goes through this
//! module. It keeps the Freya views thin and makes the behaviour testable
//! without a window.

use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::time::{Duration, Instant, SystemTime};

use jadeforge_core::{
    AmuletInfo, Change, Character, GemLocation, Henchman, Item, SaveGame, SaveImage, SaveOptions,
    Style, StyleIdentity, list_backups, restore_backup,
};
use jadeforge_gamedata::{BundledItem, GameData, NameIndex};

use crate::message;

mod error;
mod form;
mod model;

pub use error::SessionError;
pub use form::{CharacterField, CharacterForm, FieldError, FormError};
pub use model::{
    BackupRow, GameStatus, GemRow, GemSuggestion, HenchRow, ItemRow, ItemSuggestion, Message,
    MessageKind, PendingChange, PendingChangeGroup, Preview, SaveEntry, Section, SectionError,
    StyleRow, StyleSuggestion, StyleUpgrade,
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
            std::cmp::Ordering::Greater => crate::message::tr("Way of the Open Palm"),
            std::cmp::Ordering::Less => crate::message::tr("Way of the Closed Fist"),
            std::cmp::Ordering::Equal => crate::message::tr("Balanced"),
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

    /// Formats a file modification time as `YYYY-MM-DD HH:MM` in UTC.
    ///
    /// The editor has no time zone database; the savegame list only needs a
    /// stable, sortable stamp, so the timestamp is documented as UTC.
    #[must_use]
    pub fn timestamp(time: std::time::SystemTime) -> String {
        let Ok(duration) = time.duration_since(std::time::UNIX_EPOCH) else {
            return crate::message::tr("unknown").to_owned();
        };
        let seconds = duration.as_secs();
        let days = (seconds / 86_400) as i64;
        let rest = seconds % 86_400;
        let (year, month, day) = super::civil_from_days(days);
        format!(
            "{year:04}-{month:02}-{day:02} {:02}:{:02} UTC",
            rest / 3600,
            (rest % 3600) / 60
        )
    }

    /// Formats a count with the correct singular or plural noun.
    ///
    /// Every count shown to the user goes through this helper so a list can
    /// never read "1 gems" or a singular row can never read "1 effects".
    #[must_use]
    pub fn plural(count: usize, singular: &str, plural: &str) -> String {
        format!("{count} {}", if count == 1 { singular } else { plural })
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
    ///
    /// The same wording ("Time played") appears in the save rows and the
    /// character sheet; the settings key stays `played` for compatibility.
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::Name => message::tr("Name"),
            Self::Level => message::tr("Level"),
            Self::Played => message::tr("Time played"),
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

/// Friendly label for an `items.2da` category.
///
/// The game table writes lower-case ids (`plot_items`, `improve`, ...); the
/// interface never shows those raw strings. Unknown categories fall back to a
/// title-cased copy of the id so new game data still reads sensibly.
#[must_use]
pub fn item_category_label(category: &str) -> String {
    match category.to_lowercase().as_str() {
        "gems" => message::capitalize(message::tr("gems")),
        "powerups" => message::tr("Power-ups").to_owned(),
        "weapons" => message::capitalize(message::tr("weapons")),
        "plot_items" => message::tr("Plot items").to_owned(),
        "mgweapons" => message::tr("Magic weapons").to_owned(),
        "improve" => message::tr("Improvements").to_owned(),
        _ => title_case(category),
    }
}

/// Friendly label for a style kind.
///
/// [`jadeforge_gamedata::StyleKind::label`] deliberately returns lower-case
/// ids; badges and pickers show the same kind in title case instead.
#[must_use]
pub fn style_kind_label(kind: &str) -> String {
    match kind.to_lowercase().as_str() {
        "martial" => message::tr("Martial").to_owned(),
        "weapon" => message::capitalize(message::tr("weapon")),
        "magic" => message::tr("Magic").to_owned(),
        "support" => message::tr("Support").to_owned(),
        "transformation" => message::tr("Transformation").to_owned(),
        "improvised" => message::tr("Improvised").to_owned(),
        "unique" => message::tr("Unique").to_owned(),
        _ => title_case(kind),
    }
}

/// Turns an underscore id into a title-cased label.
fn title_case(value: &str) -> String {
    value
        .split(['_', '-'])
        .filter(|word| !word.is_empty())
        .map(|word| {
            let mut characters = word.chars();
            characters.next().map_or_else(String::new, |first| {
                first
                    .to_uppercase()
                    .chain(characters.flat_map(char::to_lowercase))
                    .collect()
            })
        })
        .collect::<Vec<_>>()
        .join(" ")
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

/// Converts days since 1970-01-01 to a civil calendar date.
///
/// Howard Hinnant's `civil_from_days` algorithm, valid for the whole range of
/// file timestamps before the year 10000.
fn civil_from_days(days: i64) -> (i64, u32, u32) {
    let days = days + 719_468;
    let era = if days >= 0 { days } else { days - 146_096 } / 146_097;
    let day_of_era = days - era * 146_097;
    let year_of_era =
        (day_of_era - day_of_era / 1460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let year = year_of_era + era * 400;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_piece = (5 * day_of_year + 2) / 153;
    let day = (day_of_year - (153 * month_piece + 2) / 5 + 1) as u32;
    let month = if month_piece < 10 {
        month_piece + 3
    } else {
        month_piece - 9
    } as u32;
    (year + i64::from(month <= 2), month, day)
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

thread_local! {
    /// Artificial delay applied to worker scans started from this thread.
    ///
    /// The headless UI test uses it to observe the in-flight state; it is
    /// captured into the request on the UI thread and applied on the worker, so
    /// it can never slow down another thread or the real application.
    static SCAN_DELAY: std::cell::Cell<std::time::Duration> =
        const { std::cell::Cell::new(std::time::Duration::ZERO) };
}

/// Delays worker scans started from the current thread (tests only).
#[doc(hidden)]
pub fn set_scan_delay_for_tests(delay: std::time::Duration) {
    SCAN_DELAY.with(|cell| cell.set(delay));
}

/// Environment a scan worker needs to behave exactly like the UI thread.
///
/// The gamedata test root and the settings file path are thread local for test
/// isolation, so a worker cannot read them itself. They are captured when the
/// scan starts and installed on the worker for the duration of the lookup.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GameDataContext {
    test_root: Option<PathBuf>,
    config_path: Option<PathBuf>,
    scan_delay: std::time::Duration,
}

impl GameDataContext {
    /// Captures the current thread's test overrides and settings path.
    #[must_use]
    pub fn capture() -> Self {
        Self {
            test_root: jadeforge_gamedata::test_game_root(),
            config_path: crate::config::Config::path(),
            scan_delay: SCAN_DELAY.with(std::cell::Cell::get),
        }
    }

    /// Resolves the installation for `directory`, exactly like the UI would.
    ///
    /// `JADEFORGE_GAME_DIR` and a broken discovery still win, and a
    /// user-chosen installation from the settings file is the fallback, so the
    /// worker keeps the documented precedence.
    fn load_game_data(&self, directory: &Path) -> jadeforge_gamedata::Result<Option<GameData>> {
        // Restore the previous override instead of clearing it: tests point the
        // whole fixture at a game root, and a probe session must not disarm the
        // root that the UI session started from.
        let previous = jadeforge_gamedata::test_game_root();
        jadeforge_gamedata::set_test_game_root(self.test_root.clone());
        let configured = self
            .config_path
            .as_ref()
            .and_then(|path| crate::config::Config::load_from(&Some(path.clone())).game_dir);
        let result = match GameData::discover_result(directory) {
            Ok(Some(game)) => Ok(Some(game)),
            Ok(None) => configured.map_or(Ok(None), |root| GameData::open(root).map(Some)),
            Err(error) => Err(error),
        };
        jadeforge_gamedata::set_test_game_root(previous);
        result
    }
}

/// A scan that has started; the UI hands it to a worker and back.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScanRequest {
    directory: PathBuf,
    token: u64,
    context: GameDataContext,
}

impl ScanRequest {
    /// Folder this scan reads.
    #[must_use]
    pub fn directory(&self) -> &Path {
        &self.directory
    }

    /// Scan generation; a newer scan always has a higher token.
    #[must_use]
    pub fn token(&self) -> u64 {
        self.token
    }
}

/// Everything a scan worker produces for one request.
pub struct ScanOutcome {
    entries: Vec<SaveEntry>,
    game: Result<Option<GameData>, String>,
}

impl ScanOutcome {
    /// Runs the worker half of `request`: list savegames and resolve game data.
    ///
    /// This touches no session state and can run on any thread.
    #[must_use]
    pub fn run(request: &ScanRequest) -> Self {
        if !request.context.scan_delay.is_zero() {
            std::thread::sleep(request.context.scan_delay);
        }
        let entries = Session::scan_entries(&request.directory);
        let game = request
            .context
            .load_game_data(&request.directory)
            .map_err(|error| error.to_string());
        Self { entries, game }
    }
}

/// What a completed scan committed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScanCompletion {
    /// True when the scan replaced the folder instead of refreshing it.
    pub directory_changed: bool,
}

/// Result of applying a worker scan to the session.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ScanApply {
    /// The result was current and was committed.
    Applied(ScanCompletion),
    /// The result was current but the folder could not be read any more.
    Failed,
    /// A newer scan or a cancellation superseded the result.
    Stale,
}

/// Fingerprint of one canonical block payload.
///
/// Two independently salted 64-bit hashes rather than the canonical bytes: the
/// dirty check only needs equality, and keeping every block's bytes duplicated
/// the savegame in memory. A collision would hide an unsaved edit, so the two
/// hashes are combined into 128 bits. The value is process-local; nothing
/// persists or compares it across runs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ContentFingerprint {
    first: u64,
    second: u64,
}

impl ContentFingerprint {
    fn of(bytes: &[u8]) -> Self {
        use std::hash::{Hash, Hasher};

        let mut first = std::collections::hash_map::DefaultHasher::new();
        (0_u8, bytes).hash(&mut first);
        let mut second = std::collections::hash_map::DefaultHasher::new();
        (1_u8, bytes).hash(&mut second);
        Self {
            first: first.finish(),
            second: second.finish(),
        }
    }
}

/// Budget for the canonical block payloads kept by the undo history.
///
/// Only blocks an edit touched are captured, and a savegame's `save.sav` is a
/// few hundred kilobytes, so this is many steps in practice. The cap keeps a
/// long editing session from growing the editor's memory without bound; older
/// entries are dropped first.
const HISTORY_MAX_BYTES: usize = 32 * 1024 * 1024;
/// Maximum number of undo steps kept, independent of the byte budget.
const HISTORY_MAX_ENTRIES: usize = 100;
/// Consecutive edits to the same character fields within this window become
/// one undo step, so typing `123` is one undo instead of three.
const HISTORY_COALESCE_WINDOW: Duration = Duration::from_millis(1500);

/// Canonical payload of one block before or after an action.
#[derive(Debug, Clone)]
struct BlockState {
    index: usize,
    /// `None` means "the file as it was opened"; restoring it uses the
    /// container's original bytes and marks the block clean again.
    bytes: Option<Vec<u8>>,
}

impl BlockState {
    fn cost(&self) -> usize {
        self.bytes.as_ref().map_or(0, Vec::len)
    }
}

/// One undoable user action.
#[derive(Debug, Clone)]
struct HistoryEntry {
    /// Short action label shown by undo/redo and the diff screen.
    label: String,
    /// Field and row level before/after values for the diff screen.
    changes: Vec<PendingChange>,
    /// State of every block the action changed, before the action.
    before: Vec<BlockState>,
    /// State of every block the action changed, after the action.
    after: Vec<BlockState>,
    /// Same-key actions within the coalescing window merge into this entry.
    coalesce: Option<String>,
    recorded: Instant,
}

impl HistoryEntry {
    fn cost(&self) -> usize {
        self.before.iter().map(BlockState::cost).sum::<usize>()
            + self.after.iter().map(BlockState::cost).sum::<usize>()
    }
}

/// Identity of the savegame file on disk, for external-change detection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct DiskStamp {
    len: u64,
    modified: Option<SystemTime>,
}

impl DiskStamp {
    fn of(path: &Path) -> Option<Self> {
        let metadata = std::fs::metadata(path).ok()?;
        Some(Self {
            len: metadata.len(),
            modified: metadata.modified().ok(),
        })
    }
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
    /// Sections that could not be read when the savegame was opened.
    pub(crate) section_errors: Vec<SectionError>,
    /// Character-section error of the savegame being loaded.
    character_error: Option<SectionError>,
    /// Screenshot of the opened savegame.
    ///
    /// Shared so rendering can clone the handle without copying the RGBA
    /// buffer.
    pub(crate) preview: Option<Rc<Preview>>,
    /// Status of the last action.
    pub(crate) message: Message,
    /// Backup written by the last successful save of the open savegame.
    pub(crate) last_backup: Option<PathBuf>,
    /// Fingerprints of the canonical block payloads of the opened savegame.
    ///
    /// Kept as two 64-bit hashes per block instead of the canonical bytes
    /// themselves: a savegame is a few megabytes and the full baseline was a
    /// large part of the editor's idle memory, while a 16-byte fingerprint
    /// answers the same question. Two differently salted hashes make the
    /// combined collision probability negligible (128 bits).
    baseline: Option<Vec<ContentFingerprint>>,
    /// True when the in-memory savegame differs from the file on disk.
    dirty: bool,
    /// Undo history, oldest first, with a cursor for undone entries.
    history: Vec<HistoryEntry>,
    /// Number of history entries that are currently applied.
    history_cursor: usize,
    /// Memory held by [`Session::history`].
    history_bytes: usize,
    /// Current canonical payload of every block that has edits.
    history_current: Vec<Option<Vec<u8>>>,
    /// Identity of the open savegame on disk when it was read.
    disk_stamp: Option<DiskStamp>,
    /// True when the open savegame changed on disk outside the editor.
    disk_changed: bool,
    file_size: Option<u64>,
    preview_generation: u64,
    scan_token: u64,
    /// Directory and token of a scan that is running on a worker.
    ///
    /// The session keeps its current directory, entries and open savegame until
    /// the matching result is applied, so a cancelled or failed scan leaves the
    /// screen untouched.
    scanning: Option<(PathBuf, u64)>,
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
    /// The game installation is read on the calling thread. The UI uses
    /// [`Session::pending`] instead, so opening the window neither scans the
    /// savegames nor reads the game data synchronously.
    #[must_use]
    pub fn empty(directory: impl Into<PathBuf>) -> Self {
        let mut session = Self::unloaded(directory);
        session.load_game_data();
        session
    }

    /// Creates a session whose first scan the caller runs on a worker.
    ///
    /// Unlike [`Session::empty`] this never reads a savegame or the game data on
    /// the calling thread: the game data is [`GameStatus::Loading`] until the
    /// matching [`Session::complete_scan`] applies the worker result. The UI
    /// renders its first frame from this state and starts the scan itself.
    #[must_use]
    pub fn pending(directory: impl Into<PathBuf>) -> Self {
        let mut session = Self::unloaded(directory);
        if session.directory.as_os_str().is_empty() {
            session.game_status = GameStatus::Missing;
            session.message = Message::info(message::tr(
                "Choose the folder that contains your Jade Empire savegames.",
            ));
        } else {
            session.game_status = GameStatus::Loading;
            let directory = session.directory.display().to_string();
            session.message = Message::info(message::fill(
                "Scanning {directory} for savegames…",
                &[("directory", &directory)],
            ));
        }
        session
    }

    /// Builds the empty session state without reading the game installation.
    fn unloaded(directory: impl Into<PathBuf>) -> Self {
        Self {
            directory: directory.into(),
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
            section_errors: Vec::new(),
            character_error: None,
            preview: None,
            message: Message::info(message::tr("Scanning for savegames…")),
            last_backup: None,
            baseline: None,
            dirty: false,
            history: Vec::new(),
            history_cursor: 0,
            history_bytes: 0,
            history_current: Vec::new(),
            disk_stamp: None,
            disk_changed: false,
            file_size: None,
            preview_generation: 0,
            scan_token: 0,
            scanning: None,
        }
    }

    /// Changes the directory and scans it synchronously.
    ///
    /// This is the convenience path for tests and non-UI callers. The desktop
    /// UI uses [`Session::begin_scan`] plus [`Session::complete_scan`] on a
    /// worker, so no savegame or game data is read on the UI thread. The
    /// semantics are otherwise identical: unsaved changes and unreadable
    /// folders are rejected, a same-folder refresh keeps the open savegame, and
    /// only a successful scan replaces the remembered folder.
    pub fn set_directory(&mut self, directory: impl Into<PathBuf>) -> bool {
        let Some(request) = self.begin_scan(directory) else {
            return false;
        };
        let outcome = ScanOutcome::run(&request);
        matches!(self.complete_scan(&request, outcome), ScanApply::Applied(_))
    }

    /// Drops the opened savegame but keeps the entry list.
    fn clear_open_save(&mut self) {
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
        self.last_backup = None;
        self.baseline = None;
        self.dirty = false;
        self.clear_history();
        self.disk_stamp = None;
        self.disk_changed = false;
    }

    /// Replaces the entry list with the result of [`Session::scan_entries`].
    ///
    /// An opened savegame is re-located by its folder, so a refresh of the same
    /// folder keeps it open even when the entry order changed.
    pub fn apply_entries(&mut self, entries: Vec<SaveEntry>) {
        let open = self
            .selected
            .and_then(|index| self.entries.get(index))
            .map(|entry| entry.path.clone());
        let total = entries.len();
        let broken = entries.iter().filter(|entry| entry.error.is_some()).count();
        self.entries = Rc::new(entries);
        if let Some(path) = open {
            self.selected = self.entries.iter().position(|entry| entry.path == path);
            if self.selected.is_some() {
                self.refresh_entry();
            } else {
                self.clear_open_save();
            }
        }
        let directory = self.directory.display().to_string();
        let savegames = format::plural(total, message::tr("savegame"), message::tr("savegames"));
        self.message = if total == 0 {
            Message::info(message::fill(
                "No savegames found in {directory}.",
                &[("directory", &directory)],
            ))
        } else if broken == 0 {
            Message::success(message::fill(
                "Found {savegames} in {directory}.",
                &[("savegames", &savegames), ("directory", &directory)],
            ))
        } else if broken == total {
            Message::error(message::fill(
                "Found {savegames} in {directory}, but none of them can be read.",
                &[("savegames", &savegames), ("directory", &directory)],
            ))
        } else {
            let broken = broken.to_string();
            Message::error(message::fill(
                "Found {savegames} in {directory}; {broken} of them cannot be read.",
                &[
                    ("savegames", &savegames),
                    ("directory", &directory),
                    ("broken", &broken),
                ],
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

    /// Scans the directory for savegames synchronously.
    ///
    /// Tests and non-UI callers use this; the desktop UI starts the same work
    /// with [`Session::begin_scan`] on a worker. The game data is refreshed by
    /// [`Session::complete_scan`], never here.
    pub fn scan(&mut self) {
        let Some(request) = self.begin_scan(self.directory.clone()) else {
            return;
        };
        let outcome = ScanOutcome::run(&request);
        let _ = self.complete_scan(&request, outcome);
    }

    /// Token identifying the current scan generation.
    #[must_use]
    pub fn scan_token(&self) -> u64 {
        self.scan_token
    }

    /// True while a worker scan is in flight.
    #[must_use]
    pub fn is_scanning(&self) -> bool {
        self.scanning.is_some()
    }

    /// Folder the in-flight scan is reading, when one is running.
    #[must_use]
    pub fn scanning_directory(&self) -> Option<&Path> {
        self.scanning
            .as_ref()
            .map(|(directory, _)| directory.as_path())
    }

    /// Starts a scan of `directory` and returns the request a worker should run.
    ///
    /// The current directory, entry list and open savegame are left untouched
    /// until the matching [`Session::complete_scan`] applies the result, so the
    /// UI can keep showing the old savegame while the scan is in flight. A
    /// newer request supersedes an older one and discards its result. No folder
    /// is persisted here; only a successful scan does that.
    ///
    /// `None` means the scan was refused; the status message explains why and
    /// the previous state is unchanged.
    pub fn begin_scan(&mut self, directory: impl Into<PathBuf>) -> Option<ScanRequest> {
        let directory = directory.into();
        if self.is_dirty() {
            self.message = Message::info(message::tr(
                "This savegame has unsaved changes: save them or press Revert before scanning.",
            ));
            self.settle_loading_game_data();
            return None;
        }
        if directory.as_os_str().is_empty() {
            self.message = Message::info(message::tr(
                "Choose the folder that contains your Jade Empire savegames.",
            ));
            self.settle_loading_game_data();
            return None;
        }
        if let Some(problem) = directory_problem(&directory) {
            self.message = Message::error(problem);
            self.settle_loading_game_data();
            return None;
        }
        self.scan_token = self.scan_token.wrapping_add(1);
        let token = self.scan_token;
        self.scanning = Some((directory.clone(), token));
        let directory_label = directory.display().to_string();
        self.message = Message::info(message::fill(
            "Scanning {directory} for savegames…",
            &[("directory", &directory_label)],
        ));
        Some(ScanRequest {
            directory,
            token,
            context: GameDataContext::capture(),
        })
    }

    /// Applies the result of [`ScanOutcome::run`] to the session.
    ///
    /// Only the request that is still in flight is applied. The directory,
    /// entries and game data are committed together, an open savegame is kept
    /// when the folder did not change, and only this successful path persists
    /// the folder.
    pub fn complete_scan(&mut self, request: &ScanRequest, outcome: ScanOutcome) -> ScanApply {
        if !self.is_current_scan(request) {
            return ScanApply::Stale;
        }
        if let Some(problem) = directory_problem(&request.directory) {
            self.message = Message::error(problem);
            self.settle_loading_game_data();
            self.finish_scan();
            return ScanApply::Failed;
        }
        let directory_changed = !same_directory(&self.directory, &request.directory);
        if directory_changed {
            self.clear_open_save();
        }
        self.directory.clone_from(&request.directory);
        self.apply_game_data(outcome.game);
        self.apply_entries(outcome.entries);
        if self.save.is_some() {
            // The game data may resolve different names and upgrade tables.
            self.rebuild_rows();
        }
        self.persist_directory();
        self.finish_scan();
        ScanApply::Applied(ScanCompletion { directory_changed })
    }

    /// Reports a worker that could not produce a result.
    ///
    /// Returns whether the request was still current. The previous entries,
    /// savegame and directory are kept and nothing is persisted.
    pub fn fail_scan(&mut self, request: &ScanRequest) -> bool {
        if !self.is_current_scan(request) {
            return false;
        }
        let directory = request.directory.display().to_string();
        self.message = Message::error(message::fill(
            "Could not scan {directory}: the savegame reader stopped unexpectedly.",
            &[("directory", &directory)],
        ));
        self.settle_loading_game_data();
        self.finish_scan();
        true
    }

    /// Cancels the in-flight scan, keeping the entries and savegame on screen.
    ///
    /// The matching worker result is discarded through the bumped token. Returns
    /// whether a scan was actually in flight.
    pub fn cancel_scan(&mut self) -> bool {
        if self.scanning.take().is_none() {
            return false;
        }
        self.scan_token = self.scan_token.wrapping_add(1);
        self.settle_loading_game_data();
        self.message = Message::info(message::tr(
            "Scan cancelled; the savegames on screen are unchanged.",
        ));
        true
    }

    fn is_current_scan(&self, request: &ScanRequest) -> bool {
        self.scanning.as_ref().is_some_and(|(directory, token)| {
            directory == &request.directory && *token == request.token
        })
    }

    fn finish_scan(&mut self) {
        self.scanning = None;
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
    ///
    /// Synchronous callers use this; a worker scan goes through
    /// [`ScanOutcome::run`] and [`Session::complete_scan`] instead.
    fn load_game_data(&mut self) -> Vec<String> {
        let context = GameDataContext::capture();
        let result = context
            .load_game_data(&self.directory)
            .map_err(|error| error.to_string());
        self.apply_game_data(result)
    }

    /// Applies a resolved game installation, or its error, to the session.
    fn apply_game_data(&mut self, result: Result<Option<GameData>, String>) -> Vec<String> {
        match result {
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
                self.game_status = bundled_game_status(self.name_index);
                Vec::new()
            }
            Err(reason) => {
                self.game = None;
                self.game_status = GameStatus::Error(reason.clone());
                vec![message::fill(
                    "{label}: {value}",
                    &[("label", message::tr("Game data")), ("value", &reason)],
                )]
            }
        }
    }

    /// Replaces a pending "loading" status with the best state available.
    ///
    /// A startup scan that is rejected, cancelled or cannot complete must not
    /// leave the header claiming that game data is still being read.
    fn settle_loading_game_data(&mut self) {
        if matches!(self.game_status, GameStatus::Loading) {
            self.game_status = bundled_game_status(self.name_index);
        }
    }

    /// Points `JadeForge` at a game installation chosen by the user.
    ///
    /// The installation is only read. The path is remembered in the settings
    /// file (never in the source), and an invalid folder is reported instead of
    /// replacing the working game data. Returns whether the folder was used.
    pub fn set_game_directory(&mut self, root: impl Into<PathBuf>) -> bool {
        let root = root.into();
        match GameData::open(&root) {
            Ok(game) => {
                self.game_status = GameStatus::Loaded {
                    items: game.playable_item_count(),
                    styles: game.style_count(),
                    root: game.root().to_path_buf(),
                };
                self.game = Some(Rc::new(game));
                let mut config = crate::config::Config::load();
                config.game_dir = Some(root.clone());
                let _ = config.store();
                // Names, descriptions and upgrade tables all become resolvable
                // now, so every derived row has to be rebuilt.
                self.rebuild_rows();
                let root = root.display().to_string();
                self.message = Message::success(message::fill(
                    "Using the game data in {root}.",
                    &[("root", &root)],
                ));
                true
            }
            Err(error) => {
                let root = root.display().to_string();
                self.message = Message::error(message::fill(
                    "{root} is not a readable Jade Empire installation: {error}",
                    &[("root", &root), ("error", &error.to_string())],
                ));
                false
            }
        }
    }

    /// Error of one section, when it could not be read.
    #[must_use]
    pub fn section_error(&self, section: Section) -> Option<&SectionError> {
        self.section_errors
            .iter()
            .find(|error| error.section == section)
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
            self.message = Message::info(message::tr(
                "This savegame has unsaved changes: save them or press Revert first.",
            ));
            return false;
        }
        let Some(entry) = self.entries.get(index) else {
            self.message = Message::error(message::tr(
                "That savegame no longer exists; scan the folder again.",
            ));
            return false;
        };
        if let Some(error) = &entry.error {
            self.message = Message::error(message::fill(
                "Cannot open {label}: the scan found it unreadable ({error}).",
                &[("label", &entry.label), ("error", error)],
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
                self.disk_stamp = DiskStamp::of(&path);
                self.disk_changed = false;
                let label = entry.label.clone();
                let mut warnings = Vec::new();
                self.character_error = None;
                self.character = match save.character() {
                    Ok(character) => Some(character),
                    Err(error) => {
                        self.character_error = Some(SectionError {
                            section: Section::Character,
                            message: error.to_string(),
                        });
                        None
                    }
                };
                self.save = Some(save);
                self.selected = Some(index);
                self.last_backup = None;
                self.accept_as_clean();
                let mut section_errors = self.rebuild_rows();
                if let Some(character) = &self.character_error {
                    section_errors.insert(0, character.clone());
                }
                self.section_errors = section_errors;
                self.load_preview(&path);
                if self.game.is_none() {
                    warnings.extend(self.load_game_data());
                }
                self.message = if warnings.is_empty() && self.section_errors.is_empty() {
                    Message::success(message::fill("Opened {label}.", &[("label", &label)]))
                } else {
                    let mut problems: Vec<String> = self
                        .section_errors
                        .iter()
                        .map(|error| {
                            format!(
                                "{}: {}",
                                message::capitalize(error.section.label()),
                                error.message
                            )
                        })
                        .collect();
                    problems.extend(warnings);
                    let problems = problems.join("; ");
                    Message::error(message::fill(
                        "Opened {label}, but some sections could not be read: {problems}",
                        &[("label", &label), ("problems", &problems)],
                    ))
                };
                true
            }
            Err(error) => {
                let path = path.display().to_string();
                self.message = Message::error(message::fill(
                    "Cannot open {path}: {error}",
                    &[("path", &path), ("error", &error.to_string())],
                ));
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
    fn rebuild_rows(&mut self) -> Vec<SectionError> {
        let (items, styles, henchmen, mut warnings) = self.build_rows();
        self.items = Rc::new(items);
        self.styles = Rc::new(styles);
        self.henchmen = Rc::new(henchmen);
        let (gems, gem_warnings) = self.build_gem_rows();
        self.gems = Rc::new(gems);
        warnings.extend(gem_warnings);
        self.section_errors.clone_from(&warnings);
        warnings
    }

    fn build_rows(
        &self,
    ) -> (
        Vec<ItemRow>,
        Vec<StyleRow>,
        Vec<HenchRow>,
        Vec<SectionError>,
    ) {
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
                warnings.push(SectionError {
                    section: Section::Items,
                    message: error.to_string(),
                });
                Vec::new()
            }
        };
        let styles = match save.styles() {
            Ok(styles) => styles
                .into_iter()
                .map(|style| self.describe_style(&style))
                .collect(),
            Err(error) => {
                warnings.push(SectionError {
                    section: Section::Styles,
                    message: error.to_string(),
                });
                Vec::new()
            }
        };
        let henchmen = match save.henchmen() {
            Ok(henchmen) => henchmen.into_iter().map(describe_henchman).collect(),
            Err(error) => {
                warnings.push(SectionError {
                    section: Section::Companions,
                    message: error.to_string(),
                });
                Vec::new()
            }
        };
        (items, styles, henchmen, warnings)
    }

    fn describe_item(&self, item: &Item) -> ItemRow {
        let info = self.game.as_ref().and_then(|game| game.item(item.item_id));
        let bundled = self.bundled_item(item.item_id);
        let fallback = message::fill("Item {item_id}", &[("item_id", &item.item_id.to_string())]);
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
        let fallback = message::fill(
            "Style {style_id}",
            &[("style_id", &style.style_id.to_string())],
        );
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

    /// True when the opened savegame differs from the file on disk.
    ///
    /// This is a content comparison, not the sticky per-block edit flag: typing
    /// a value and restoring it clears the dirty state again, so the close
    /// guard and the unsaved badge cannot report changes that are not there.
    #[must_use]
    pub fn is_dirty(&self) -> bool {
        self.dirty
    }

    /// Recomputes [`Session::is_dirty`] after a mutation.
    ///
    /// Only edited blocks are re-serialized, and they are compared with the
    /// fingerprint of the same block at load time. Restoring a value therefore
    /// clears the flag instead of leaving a sticky "was touched" marker behind.
    fn refresh_dirty(&mut self) {
        self.dirty = match (self.save.as_ref(), self.baseline.as_ref()) {
            (Some(save), Some(baseline)) => {
                save.blocks().iter().enumerate().any(|(index, block)| {
                    block.is_modified()
                        && block.to_canonical_bytes().is_ok_and(|bytes| {
                            baseline.get(index) != Some(&ContentFingerprint::of(&bytes))
                        })
                })
            }
            (Some(save), None) => save.is_modified(),
            (None, _) => false,
        };
    }

    /// Remembers the current in-memory content as the clean baseline.
    fn accept_as_clean(&mut self) {
        self.baseline = self.save.as_ref().and_then(|save| {
            save.blocks()
                .iter()
                .map(|block| {
                    block
                        .to_canonical_bytes()
                        .ok()
                        .map(|bytes| ContentFingerprint::of(&bytes))
                })
                .collect::<Option<Vec<_>>>()
        });
        self.dirty = false;
        self.clear_history();
    }

    /// Canonical payload of every block after the current edit.
    ///
    /// `None` means the block is still the file as opened. Serialization only
    /// runs for edited blocks, which is normally a single `save.sav`.
    fn current_block_states(&self) -> Option<Vec<Option<Vec<u8>>>> {
        let save = self.save.as_ref()?;
        let mut states = Vec::with_capacity(save.blocks().len());
        for (index, block) in save.blocks().iter().enumerate() {
            if block.is_modified() {
                states.push(Some(save.block_canonical_payload(index).ok()?));
            } else {
                states.push(None);
            }
        }
        Some(states)
    }

    /// Records the block payloads and field detail of one applied action.
    ///
    /// A no-op (the block payloads did not change) records nothing. Consecutive
    /// edits to the same character fields within
    /// [`HISTORY_COALESCE_WINDOW`] merge into the previous entry, so typing a
    /// number is one undo step. The history is bounded by
    /// [`HISTORY_MAX_BYTES`] and [`HISTORY_MAX_ENTRIES`].
    fn remember_edit(
        &mut self,
        label: &str,
        changes: Vec<PendingChange>,
        coalesce: Option<String>,
    ) {
        if changes.is_empty() {
            return;
        }
        let Some(states) = self.current_block_states() else {
            return;
        };
        let previous = std::mem::replace(&mut self.history_current, states.clone());
        if previous == states {
            return;
        }
        let mut before = Vec::new();
        let mut after = Vec::new();
        for (index, (old, new)) in previous.iter().zip(&states).enumerate() {
            if old != new {
                before.push(BlockState {
                    index,
                    bytes: old.clone(),
                });
                after.push(BlockState {
                    index,
                    bytes: new.clone(),
                });
            }
        }
        if before.is_empty() {
            return;
        }
        let now = Instant::now();
        let merge = coalesce.as_deref().is_some_and(|key| {
            self.history_cursor == self.history.len()
                && self.history.last().is_some_and(|last| {
                    last.coalesce.as_deref() == Some(key)
                        && now.duration_since(last.recorded) <= HISTORY_COALESCE_WINDOW
                })
        });
        if merge && let Some(last) = self.history.last_mut() {
            let old_cost = last.cost();
            for state in before {
                if !last.before.iter().any(|known| known.index == state.index) {
                    last.before.push(state);
                }
            }
            last.after = after;
            for change in changes {
                if let Some(existing) = last
                    .changes
                    .iter_mut()
                    .find(|existing| existing.field == change.field)
                {
                    existing.after = change.after;
                } else {
                    last.changes.push(change);
                }
            }
            last.recorded = now;
            let new_cost = last.cost();
            self.history_bytes = self.history_bytes.saturating_sub(old_cost) + new_cost;
            return;
        }
        let entry = HistoryEntry {
            label: label.to_owned(),
            changes,
            before,
            after,
            coalesce,
            recorded: now,
        };
        self.history_bytes += entry.cost();
        while self.history.len() > self.history_cursor {
            if let Some(removed) = self.history.pop() {
                self.history_bytes = self.history_bytes.saturating_sub(removed.cost());
            }
        }
        self.history.push(entry);
        self.history_cursor += 1;
        while self.history.len() > HISTORY_MAX_ENTRIES || self.history_bytes > HISTORY_MAX_BYTES {
            if self.history.is_empty() {
                break;
            }
            let removed = self.history.remove(0);
            self.history_bytes = self.history_bytes.saturating_sub(removed.cost());
            self.history_cursor = self.history_cursor.saturating_sub(1);
        }
    }

    /// Drops the undo history, for example after a save or a reload.
    fn clear_history(&mut self) {
        self.history.clear();
        self.history_cursor = 0;
        self.history_bytes = 0;
        self.history_current = self
            .save
            .as_ref()
            .map(|save| vec![None; save.blocks().len()])
            .unwrap_or_default();
    }

    /// True when there is an applied action to undo.
    #[must_use]
    pub fn can_undo(&self) -> bool {
        self.history_cursor > 0
    }

    /// True when an undone action can be reapplied.
    #[must_use]
    pub fn can_redo(&self) -> bool {
        self.history_cursor < self.history.len()
    }

    /// Restores the block payloads of `states`.
    fn restore_states(&mut self, states: &[BlockState]) -> bool {
        let Some(save) = self.save.as_mut() else {
            return false;
        };
        let mut failure = None;
        for state in states {
            let result = match &state.bytes {
                Some(bytes) => save.restore_block_payload(state.index, bytes),
                None => save.restore_block_from_source(state.index),
            };
            if let Err(error) = result {
                failure = Some(error);
                break;
            }
        }
        if let Some(error) = failure {
            self.message = Message::error(message::fill(
                "Cannot undo that step: {error}",
                &[("error", &error.to_string())],
            ));
            return false;
        }
        true
    }

    /// Re-reads the typed views after undo/redo replaced block payloads.
    fn refresh_after_history_restore(&mut self) {
        self.character = self.save.as_ref().and_then(|save| save.character().ok());
        self.rebuild_rows();
        self.refresh_entry();
        self.refresh_dirty();
        self.history_current = self.current_block_states().unwrap_or_default();
    }

    /// Undoes the last applied action and restores the typed views.
    pub fn undo(&mut self) -> bool {
        if self.history_cursor == 0 {
            self.message = Message::info(message::tr("There is nothing to undo."));
            return false;
        }
        let entry = self.history[self.history_cursor - 1].clone();
        if !self.restore_states(&entry.before) {
            return false;
        }
        self.history_cursor -= 1;
        self.refresh_after_history_restore();
        self.message = Message::info(message::fill("Undid {label}.", &[("label", &entry.label)]));
        true
    }

    /// Reapplies the most recently undone action.
    pub fn redo(&mut self) -> bool {
        if self.history_cursor >= self.history.len() {
            self.message = Message::info(message::tr("There is nothing to redo."));
            return false;
        }
        let entry = self.history[self.history_cursor].clone();
        if !self.restore_states(&entry.after) {
            return false;
        }
        self.history_cursor += 1;
        self.refresh_after_history_restore();
        self.message = Message::info(message::fill("Redid {label}.", &[("label", &entry.label)]));
        true
    }

    /// Pending changes since the last save, grouped by action.
    ///
    /// This is what the review screen shows before a write. A structural edit
    /// that leaves an orphan struct can keep the savegame dirty after every
    /// described change has been undone; the screen then shows a generic
    /// structural note instead of pretending there is nothing to write.
    #[must_use]
    pub fn pending_changes(&self) -> Vec<PendingChangeGroup> {
        let applied = &self.history[..self.history_cursor.min(self.history.len())];
        let mut groups: Vec<PendingChangeGroup> = Vec::new();
        for entry in applied {
            if let Some(last) = groups.last_mut()
                && last.label == entry.label
            {
                last.changes.extend(entry.changes.iter().cloned());
            } else {
                groups.push(PendingChangeGroup {
                    label: entry.label.clone(),
                    changes: entry.changes.clone(),
                });
            }
        }
        if groups.is_empty() && self.dirty {
            groups.push(PendingChangeGroup {
                label: message::tr("Savegame structure").to_owned(),
                changes: vec![PendingChange::new(
                    message::tr("Block layout"),
                    message::tr("as opened"),
                    message::tr("structs were added or orphaned"),
                )],
            });
        }
        groups
    }

    /// Checks whether the open savegame changed on disk.
    ///
    /// The check compares size and modification time, the same identity the
    /// core uses before a save. It is cheap enough to run once per render, and
    /// `save` performs the authoritative check before writing. A file replaced
    /// with identical size and timestamp is not detected until the save
    /// conflict check; that caveat is documented in the design note.
    pub fn check_disk_state(&mut self) -> bool {
        let Some(path) = self.current_path() else {
            self.disk_changed = false;
            self.disk_stamp = None;
            return false;
        };
        let stamp = DiskStamp::of(&path);
        if self.disk_stamp.is_none() {
            self.disk_stamp = stamp;
            self.disk_changed = false;
            return false;
        }
        self.disk_changed = stamp != self.disk_stamp;
        self.disk_changed
    }

    /// True when the last [`Session::check_disk_state`] found an external edit.
    #[must_use]
    pub fn disk_changed(&self) -> bool {
        self.disk_changed
    }

    /// Re-reads the open savegame from disk.
    ///
    /// Refuses while the in-memory savegame has unsaved changes: the caller
    /// must offer an explicit discard first, which is what the review screen
    /// and the external-change banner do.
    pub fn reload_from_disk(&mut self) -> Result<(), SessionError> {
        let Some(index) = self.selected else {
            self.message = Message::error(message::tr("No savegame is open."));
            return Err(SessionError::NoSavegame);
        };
        if self.dirty {
            self.message = Message::error(message::tr(
                "Reloading from disk would discard the unsaved changes. Save or revert them first.",
            ));
            return Err(SessionError::CannotReload);
        }
        if !self.load(index) {
            return Err(SessionError::CannotReload);
        }
        self.message = Message::info(message::tr("Reloaded the savegame from disk."));
        Ok(())
    }

    /// Backups of the open savegame, newest first.
    #[must_use]
    pub fn backups(&self) -> Vec<BackupRow> {
        let Some(path) = self.current_path() else {
            return Vec::new();
        };
        list_backups(&path)
            .unwrap_or_default()
            .into_iter()
            .map(|backup| BackupRow {
                path: backup.path,
                original: backup.original,
                created: backup.created,
                size: backup.size,
            })
            .collect()
    }

    /// Restores one of the open savegame's backups.
    ///
    /// Refuses when the current savegame has unsaved changes or when `backup`
    /// does not belong to it. The core verifies both files and writes a new
    /// backup of the current file first; the returned path is that safety copy.
    pub fn restore_backup(&mut self, backup: &Path) -> Result<PathBuf, SessionError> {
        let Some(path) = self.current_path() else {
            self.message = Message::error(message::tr("No savegame is open."));
            return Err(SessionError::NoSavegame);
        };
        if self.dirty {
            self.message = Message::error(message::tr(
                "Save or revert the current edits before restoring a backup.",
            ));
            return Err(SessionError::CannotReload);
        }
        let known = self
            .backups()
            .into_iter()
            .any(|candidate| candidate.path == backup);
        if !known {
            self.message = Message::error(message::tr(
                "That file is not a backup of the open savegame.",
            ));
            return Err(SessionError::CannotReload);
        }
        match restore_backup(backup, &path) {
            Ok(safety) => {
                let index = self.selected;
                if let Some(index) = index {
                    let _ = self.load(index);
                }
                self.last_backup = Some(safety.clone());
                let backup_path = backup.display().to_string();
                let safety_path = safety.display().to_string();
                self.message = Message::success(message::fill(
                    "Restored {backup}. The file it replaced is kept as {safety}.",
                    &[("backup", &backup_path), ("safety", &safety_path)],
                ));
                Ok(safety)
            }
            Err(error) => {
                self.message = Message::error(message::fill(
                    "Cannot restore the backup: {error}",
                    &[("error", &error.to_string())],
                ));
                Err(SessionError::CannotReload)
            }
        }
    }

    /// Every field of `form` that cannot be applied, in field order.
    #[must_use]
    pub fn form_errors(&self, form: &CharacterForm) -> Vec<FieldError> {
        self.character
            .as_ref()
            .map_or_else(Vec::new, |character| form.validate_against(character))
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
            self.message = Message::error(message::tr("No savegame is open."));
            return Err(SessionError::NoSavegame);
        };
        match save.apply_character(&character) {
            Ok(changes) => {
                self.character = Some(character);
                self.rebuild_rows();
                self.refresh_entry();
                self.refresh_dirty();
                if changes.is_empty() {
                    self.message = Message::info(message::tr(
                        "Nothing changed, so there was nothing to apply.",
                    ));
                } else {
                    self.remember_edit(message::tr("Character"), pending_from_core(&changes), None);
                    let changes_label = format::plural(
                        changes.len(),
                        message::tr("change"),
                        message::tr("changes"),
                    );
                    self.message = Message::success(message::fill(
                        "Applied {changes} in memory.",
                        &[("changes", &changes_label)],
                    ));
                }
                Ok(changes.len())
            }
            Err(error) => {
                self.message = Message::error(error.to_string());
                Err(SessionError::Save(error.to_string()))
            }
        }
    }

    /// Applies every valid field of the form to the in-memory savegame.
    ///
    /// Used by the shell to apply character edits as soon as they are valid, so
    /// every edit behaves like the item/style/gem edits. One mistyped field
    /// does not stop the other fields from applying; the invalid field is left
    /// in the form for the user to fix.
    pub fn apply_form_silently(&mut self, form: &CharacterForm) -> bool {
        let Some(base) = self.character.as_ref() else {
            return false;
        };
        let (character, _) = form.to_character_partial(base);
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
                self.refresh_dirty();
                let pending = pending_from_core(&changes);
                let key = pending
                    .iter()
                    .map(|change| change.field.as_str())
                    .collect::<Vec<_>>()
                    .join("|");
                self.remember_edit(
                    message::tr("Character"),
                    pending,
                    (!key.is_empty()).then_some(key),
                );
                true
            }
            _ => false,
        }
    }

    /// Writes the opened savegame back to disk.
    ///
    /// A successful save records the backup created next to the savegame, so
    /// the UI can show the exact pre-save file instead of asking the user to
    /// search the folder.
    pub fn save(&mut self) -> Result<PathBuf, SessionError> {
        let Some(save) = self.save.as_mut() else {
            return Err(SessionError::NoSavegame);
        };
        if !self.dirty {
            self.message = Message::info(message::tr(
                "Nothing changed, so there is nothing to write.",
            ));
            return Err(SessionError::NothingToSave);
        }
        match save.save(SaveOptions { backup: true }) {
            Ok(path) => {
                let backup = save.last_backup().map(Path::to_path_buf);
                self.file_size = std::fs::metadata(&path).ok().map(|meta| meta.len());
                self.disk_stamp = DiskStamp::of(&path);
                self.disk_changed = false;
                self.last_backup = backup;
                let path_label = path.display().to_string();
                self.message = match &self.last_backup {
                    Some(backup) => {
                        let backup = backup.display().to_string();
                        Message::success(message::fill(
                            "Saved {path}. The previous file is kept as {backup}.",
                            &[("path", &path_label), ("backup", &backup)],
                        ))
                    }
                    None => {
                        Message::success(message::fill("Saved {path}.", &[("path", &path_label)]))
                    }
                };
                self.refresh_entry();
                self.accept_as_clean();
                Ok(path)
            }
            Err(error) => {
                if matches!(error, jadeforge_core::Error::Conflict { .. }) {
                    self.disk_changed = true;
                    self.message = Message::error(message::fill(
                        "The savegame changed on disk since it was opened, so it was not overwritten. \
                         Reload it to keep the file on disk, or restore a backup. ({error})",
                        &[("error", &error.to_string())],
                    ));
                } else {
                    self.message = Message::error(message::fill(
                        "Cannot save: {error}",
                        &[("error", &error.to_string())],
                    ));
                }
                Err(SessionError::Save(error.to_string()))
            }
        }
    }

    /// Backup kept by the last successful save of the open savegame.
    ///
    /// `None` after opening a savegame, using Revert or saving a new file
    /// without a previous copy.
    #[must_use]
    pub fn last_backup(&self) -> Option<&Path> {
        self.last_backup.as_deref()
    }

    /// Saves every applied edit even when the character form is invalid.
    ///
    /// The form's valid fields are applied first; an invalid field is reported
    /// separately and stays in the form. This keeps item, style and gem edits
    /// savable while one character number is mistyped.
    pub fn save_applied(&mut self, form: &CharacterForm) -> Result<PathBuf, SessionError> {
        let invalid = self.form_errors(form);
        self.apply_form_silently(form);
        match self.save() {
            Ok(path) => {
                if let Some(first) = invalid.first() {
                    let path = path.display().to_string();
                    self.message = Message::info(message::fill(
                        "Saved {path}. The invalid character field was not applied: {error}",
                        &[("path", &path), ("error", &first.message)],
                    ));
                }
                Ok(path)
            }
            Err(error) => {
                if let Some(first) = invalid.first() {
                    self.message = Message::error(message::fill(
                        "{message} The character form also has an invalid field: {error}",
                        &[("message", &self.message.text), ("error", &first.message)],
                    ));
                }
                Err(error)
            }
        }
    }

    /// Discards all changes by reloading the file.
    pub fn revert(&mut self) -> Result<(), SessionError> {
        let Some(index) = self.selected else {
            return Err(SessionError::NoSavegame);
        };
        if self.load(index) {
            self.message = Message::info(message::tr("Reloaded the savegame from disk."));
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
            self.message = Message::info(message::fill(
                "{name}: a stack cannot hold more than 255.",
                &[("name", &name)],
            ));
            return;
        }
        if new_count < 1 {
            self.message = Message::info(message::tr(
                "A stack needs at least one item; use Remove instead.",
            ));
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
                self.remember_edit(
                    message::tr("Item stack"),
                    vec![PendingChange::new(
                        format!("ItemList[{struct_index}].Count ({name})"),
                        current.to_string(),
                        new_count.to_string(),
                    )],
                    None,
                );
                let count = format::plural(
                    usize::try_from(new_count).unwrap_or(usize::MAX),
                    message::tr("item"),
                    message::tr("items"),
                );
                self.message = Message::success(message::fill(
                    "{name}: {count} in the stack.",
                    &[("name", &name), ("count", &count)],
                ));
            }
            Err(error) => {
                self.message = Message::error(message::fill(
                    "Cannot change item: {error}",
                    &[("error", &error.to_string())],
                ));
            }
        }
        self.refresh_dirty();
    }

    /// Sets a stack size directly, so a large count is one edit instead of
    /// hundreds of stepper clicks.
    pub fn set_item_count(&mut self, struct_index: usize, count: u8) {
        let Some(name) = self
            .items
            .iter()
            .find(|item| item.struct_index == struct_index)
            .map(|item| item.name.clone())
        else {
            return;
        };
        if count == 0 {
            self.message = Message::info(message::tr(
                "A stack needs at least one item; use Remove instead.",
            ));
            return;
        }
        let Some(save) = self.save.as_mut() else {
            return;
        };
        let previous = self
            .items
            .iter()
            .find(|item| item.struct_index == struct_index)
            .map_or(count, |item| item.count);
        match save.set_item_count(struct_index, count) {
            Ok(()) => {
                if let Some(item) = Rc::make_mut(&mut self.items)
                    .iter_mut()
                    .find(|item| item.struct_index == struct_index)
                {
                    item.count = count;
                }
                self.remember_edit(
                    message::tr("Item stack"),
                    vec![PendingChange::new(
                        format!("ItemList[{struct_index}].Count ({name})"),
                        previous.to_string(),
                        count.to_string(),
                    )],
                    None,
                );
                let count_label = format::plural(
                    usize::from(count),
                    message::tr("item"),
                    message::tr("items"),
                );
                self.message = Message::success(message::fill(
                    "{name}: {count} in the stack.",
                    &[("name", &name), ("count", &count_label)],
                ));
            }
            Err(error) => {
                self.message = Message::error(message::fill(
                    "Cannot change item: {error}",
                    &[("error", &error.to_string())],
                ));
            }
        }
        self.refresh_dirty();
    }

    /// Adds an item stack by its `items.2da` row.
    pub fn add_item(&mut self, item_id: u16) -> Result<(), SessionError> {
        let bundled = self.bundled_item(item_id);
        let (name, save_type) =
            if let Some(info) = self.game.as_ref().and_then(|game| game.item(item_id)) {
                let Some(save_type) = info.save_type else {
                    let error = message::fill(
                        "item {item_id} has no savegame category type",
                        &[("item_id", &item_id.to_string())],
                    );
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
                (
                    message::fill("Item {item_id}", &[("item_id", &item_id.to_string())]),
                    8,
                )
            };
        let Some(save) = self.save.as_mut() else {
            return Err(SessionError::NoSavegame);
        };
        match save.add_item(item_id, save_type, 1) {
            Ok(index) => {
                self.rebuild_rows();
                self.remember_edit(
                    message::tr("Item added"),
                    vec![PendingChange::new(
                        format!("ItemList[{index}] ({name})"),
                        message::tr("not present"),
                        format!("#{item_id} x1"),
                    )],
                    None,
                );
                self.message = Message::success(message::fill("Added {name}.", &[("name", &name)]));
                Ok(())
            }
            Err(error) => {
                self.message = Message::error(message::fill(
                    "Cannot add item: {error}",
                    &[("error", &error.to_string())],
                ));
                Err(SessionError::Item(error.to_string()))
            }
        }
    }

    /// Removes an item stack.
    pub fn remove_item(&mut self, struct_index: usize) {
        let Some((name, item_id, count)) = self
            .items
            .iter()
            .find(|item| item.struct_index == struct_index)
            .map(|item| (item.name.clone(), item.item_id, item.count))
        else {
            return;
        };
        let Some(save) = self.save.as_mut() else {
            return;
        };
        match save.remove_item(struct_index) {
            Ok(true) => {
                Rc::make_mut(&mut self.items).retain(|item| item.struct_index != struct_index);
                self.remember_edit(
                    message::tr("Item removed"),
                    vec![PendingChange::new(
                        format!("ItemList[{struct_index}] ({name})"),
                        format!("#{item_id} x{count}"),
                        message::tr("removed"),
                    )],
                    None,
                );
                self.message =
                    Message::success(message::fill("Removed {what}.", &[("what", &name)]));
            }
            Ok(false) => self.message = Message::info(message::tr("The item was already gone.")),
            Err(error) => {
                self.message = Message::error(message::fill(
                    "Cannot remove item: {error}",
                    &[("error", &error.to_string())],
                ));
            }
        }
        self.refresh_dirty();
    }

    /// Searches the item database.
    ///
    /// An empty query lists the whole database so the picker doubles as a
    /// browsable list.
    #[must_use]
    pub fn search_items(&self, query: &str, limit: usize) -> (Vec<ItemSuggestion>, usize) {
        self.search_items_filtered(query, None, limit)
    }

    /// Searches the item database, optionally inside one category.
    ///
    /// The category is the raw `items.2da` id (`gems`, `plot_items`, ...); the
    /// picker builds its chips from [`Session::item_categories`] and shows the
    /// friendly labels from [`item_category_label`].
    #[must_use]
    pub fn search_items_filtered(
        &self,
        query: &str,
        category: Option<&str>,
        limit: usize,
    ) -> (Vec<ItemSuggestion>, usize) {
        let wanted = category
            .map(|category| category.trim().to_lowercase())
            .filter(|category| !category.is_empty());
        let in_category = |candidate: Option<&String>| {
            wanted.as_deref().is_none_or(|wanted| {
                candidate.is_some_and(|candidate| candidate.eq_ignore_ascii_case(wanted))
            })
        };

        if let Some(game) = self.game.as_ref() {
            let matches: Vec<&jadeforge_gamedata::ItemInfo> = if query.trim().is_empty() {
                let mut all: Vec<(&jadeforge_gamedata::ItemInfo, String)> = game
                    .playable_items()
                    .filter(|item| item.save_type.is_some() && in_category(item.category.as_ref()))
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
                    .filter(|item| item.save_type.is_some() && in_category(item.category.as_ref()))
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
        let (matches, _) = names.search_items(query, usize::MAX);
        let matches: Vec<&jadeforge_gamedata::BundledItem> = matches
            .into_iter()
            .filter(|item| in_category(item.category.as_ref()))
            .collect();
        let total = matches.len();
        let rows = matches
            .into_iter()
            .take(limit)
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

    /// Categories of the items the picker can add, as raw `items.2da` ids.
    ///
    /// Sorted so the chips are stable between renders and launches.
    #[must_use]
    pub fn item_categories(&self) -> Vec<String> {
        let mut categories = std::collections::BTreeSet::new();
        if let Some(game) = self.game.as_ref() {
            for item in game.playable_items() {
                if item.save_type.is_some()
                    && let Some(category) = &item.category
                {
                    categories.insert(category.to_lowercase());
                }
            }
        } else if let Some(names) = self.name_index {
            for item in names.addable_items() {
                if let Some(category) = &item.category {
                    categories.insert(category.to_lowercase());
                }
            }
        }
        categories.into_iter().collect()
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
                        || {
                            message::fill(
                                "Upgrade {advance}",
                                &[("advance", &choice.advance.to_string())],
                            )
                        },
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
        let before_mask = style.mask;
        let before_level = style.level;
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
        let changed = match save.set_style_family_slot(struct_index, &family_slots, slot) {
            Ok(changed) => {
                self.rebuild_rows();
                if changed > 0 {
                    let after = self
                        .styles
                        .iter()
                        .find(|row| row.struct_index == struct_index)
                        .map_or((before_mask, before_level), |row| (row.mask, row.level));
                    self.remember_edit(
                        message::tr("Style upgrade"),
                        vec![
                            PendingChange::new(
                                format!(
                                    "StyleList[{struct_index}].StyleLevelInfo ({})",
                                    style.name
                                ),
                                format!("0x{before_mask:016X}"),
                                format!("0x{:016X}", after.0),
                            ),
                            PendingChange::new(
                                format!("StyleList[{struct_index}].StyleLevel ({})", style.name),
                                before_level.to_string(),
                                after.1.to_string(),
                            ),
                        ],
                        None,
                    );
                }
                self.message = if changed == 0 {
                    Message::info(message::tr(
                        "That upgrade is already in the requested state.",
                    ))
                } else {
                    let upgrades = format::plural(
                        usize::try_from(changed).unwrap_or(usize::MAX),
                        message::tr("upgrade"),
                        message::tr("upgrades"),
                    );
                    if taken {
                        Message::success(message::fill(
                            "Bought {upgrades}.",
                            &[("upgrades", &upgrades)],
                        ))
                    } else {
                        Message::success(message::fill("Removed {what}.", &[("what", &upgrades)]))
                    }
                };
                changed > 0
            }
            Err(error) => {
                self.rebuild_rows();
                self.message = Message::error(message::fill(
                    "Cannot change the upgrade: {error}",
                    &[("error", &error.to_string())],
                ));
                false
            }
        };
        self.refresh_dirty();
        changed
    }

    fn build_gem_rows(&mut self) -> (Vec<GemRow>, Vec<SectionError>) {
        let Some(save) = self.save.as_ref() else {
            return (Vec::new(), Vec::new());
        };
        let mut warnings = Vec::new();
        let gems = match save.gems() {
            Ok(gems) => gems,
            Err(error) => {
                warnings.push(SectionError {
                    section: Section::Gems,
                    message: error.to_string(),
                });
                Vec::new()
            }
        };
        let amulet = match save.amulet() {
            Ok(amulet) => amulet,
            Err(error) => {
                warnings.push(SectionError {
                    section: Section::Amulet,
                    message: error.to_string(),
                });
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
                let empty = gem.is_empty();
                let effects: Vec<String> = gem
                    .effects
                    .iter()
                    .map(|effect| {
                        self.game.as_ref().map_or_else(
                            || describe_unsaved_effect(effect),
                            |game| describe_effect(game, effect),
                        )
                    })
                    .collect();
                let name = if empty {
                    message::tr("Empty socket").to_owned()
                } else {
                    info.map(|item| item.name.clone())
                        .or_else(|| bundled.map(|item| item.name.clone()))
                        .unwrap_or_else(|| {
                            message::fill("Gem {item_id}", &[("item_id", &gem.item_id.to_string())])
                        })
                };
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

    /// Gems that can be added, with the effect lines shown before adding one.
    #[must_use]
    pub fn available_gems(&self) -> Vec<GemSuggestion> {
        self.game.as_ref().map_or_else(Vec::new, |game| {
            game.gems()
                .iter()
                .map(|gem| GemSuggestion {
                    item_id: gem.item_id,
                    name: gem.name.clone(),
                    effects: gem
                        .effects
                        .iter()
                        .map(|(kind, value)| describe_gem_effect(game, *kind, *value, gem.duration))
                        .collect(),
                })
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
            self.message = Message::error(message::tr("That gem is not in the game data."));
            return;
        };
        let Some(save) = self.save.as_mut() else {
            return;
        };
        match save.add_gem(item_id, &gem.effects, gem.duration) {
            Ok(index) => {
                self.gems = Rc::new(self.build_gem_rows().0);
                self.remember_edit(
                    message::tr("Gem added"),
                    vec![PendingChange::new(
                        format!("GemList[{index}] ({})", gem.name),
                        message::tr("not present"),
                        format!("gem #{item_id}"),
                    )],
                    None,
                );
                self.message =
                    Message::success(message::fill("Added {name}.", &[("name", &gem.name)]));
            }
            Err(error) => {
                self.message = Message::error(message::fill(
                    "Cannot add the gem: {error}",
                    &[("error", &error.to_string())],
                ));
            }
        }
        self.refresh_dirty();
    }

    /// Moves a gem between the amulet and the inventory.
    pub fn move_gem(&mut self, struct_index: usize) {
        let before = self
            .gems
            .iter()
            .find(|gem| gem.struct_index == struct_index)
            .map(|gem| (gem.name.clone(), gem.location));
        let Some(save) = self.save.as_mut() else {
            return;
        };
        match save.move_gem(struct_index) {
            Ok(location) => {
                self.gems = Rc::new(self.build_gem_rows().0);
                if let Some((name, from)) = before {
                    let inventory = message::tr("inventory");
                    let amulet = message::tr("amulet");
                    let (from, to) = match (from, location) {
                        (GemLocation::Inventory, GemLocation::Amulet) => (inventory, amulet),
                        (GemLocation::Amulet, GemLocation::Inventory) => (amulet, inventory),
                        (GemLocation::Inventory, GemLocation::Inventory) => (inventory, inventory),
                        (GemLocation::Amulet, GemLocation::Amulet) => (amulet, amulet),
                    };
                    self.remember_edit(
                        message::tr("Gem moved"),
                        vec![PendingChange::new(
                            format!("GemList[{struct_index}] ({name})"),
                            from,
                            to,
                        )],
                        None,
                    );
                }
                self.message = Message::success(message::tr(match location {
                    GemLocation::Amulet => "Gem socketed into the amulet.",
                    GemLocation::Inventory => "Gem moved to the inventory.",
                }));
            }
            Err(error) => {
                self.message = Message::error(message::fill(
                    "Cannot move the gem: {error}",
                    &[("error", &error.to_string())],
                ));
            }
        }
        self.refresh_dirty();
    }

    /// Removes a gem from the inventory or the amulet.
    pub fn remove_gem(&mut self, struct_index: usize) {
        let before = self
            .gems
            .iter()
            .find(|gem| gem.struct_index == struct_index)
            .map(|gem| (gem.name.clone(), gem.location));
        let Some(save) = self.save.as_mut() else {
            return;
        };
        match save.remove_gem(struct_index) {
            Ok(()) => {
                self.gems = Rc::new(self.build_gem_rows().0);
                if let Some((name, location)) = before {
                    self.remember_edit(
                        message::tr("Gem removed"),
                        vec![PendingChange::new(
                            format!("GemList[{struct_index}] ({name})"),
                            format!("{location:?}"),
                            message::tr("removed"),
                        )],
                        None,
                    );
                }
                self.message = Message::success(format!("{}.", message::tr("Gem removed")));
            }
            Err(error) => {
                self.message = Message::error(message::fill(
                    "Cannot remove the gem: {error}",
                    &[("error", &error.to_string())],
                ));
            }
        }
        self.refresh_dirty();
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
            self.message = Message::error(message::tr("That style is not in the game data."));
            return;
        };
        let Some(template) = self.styles.first().map(|style| style.struct_index) else {
            self.message = Message::error(message::tr(
                "Learn a style first so JadeForge can copy the field layout.",
            ));
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
            Ok(index) => {
                self.rebuild_rows();
                self.remember_edit(
                    message::tr("Style added"),
                    vec![PendingChange::new(
                        format!("StyleList[{index}] ({})", info.name),
                        message::tr("not present"),
                        format!("style #{}", info.id),
                    )],
                    None,
                );
                self.message =
                    Message::success(message::fill("Added {name}.", &[("name", &info.name)]));
            }
            Err(error) => {
                self.message = Message::error(message::fill(
                    "Cannot add style: {error}",
                    &[("error", &error.to_string())],
                ));
            }
        }
        self.refresh_dirty();
    }

    /// Removes a style from the player.
    pub fn remove_style(&mut self, struct_index: usize) {
        let Some((name, style_id)) = self
            .styles
            .iter()
            .find(|style| style.struct_index == struct_index)
            .map(|style| (style.name.clone(), style.style_id))
        else {
            return;
        };
        if self.styles.len() <= 1 {
            self.message = Message::error(message::tr(
                "Keep at least one style: JadeForge needs an existing style as the layout template for learning one.",
            ));
            return;
        }
        let Some(save) = self.save.as_mut() else {
            return;
        };
        match save.remove_style(struct_index) {
            Ok(true) => {
                Rc::make_mut(&mut self.styles).retain(|style| style.struct_index != struct_index);
                self.remember_edit(
                    message::tr("Style removed"),
                    vec![PendingChange::new(
                        format!("StyleList[{struct_index}] ({name})"),
                        format!("style #{style_id}"),
                        message::tr("removed"),
                    )],
                    None,
                );
                self.message =
                    Message::success(message::fill("Removed {what}.", &[("what", &name)]));
            }
            Ok(false) => self.message = Message::info(message::tr("The style was already gone.")),
            Err(error) => {
                self.message = Message::error(message::fill(
                    "Cannot remove style: {error}",
                    &[("error", &error.to_string())],
                ));
            }
        }
        self.refresh_dirty();
    }

    /// Changes the level of a companion.
    pub fn set_henchman_level(&mut self, struct_index: usize, level: u8) {
        if !(1..=60).contains(&level) {
            self.message = Message::info(message::tr("Companion levels go from 1 to 60."));
            return;
        }
        let known = self
            .henchmen
            .iter()
            .find(|henchman| henchman.struct_index == struct_index)
            .map(|henchman| (henchman.name.clone(), henchman.level));
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
                if let Some((name, previous)) = known {
                    self.remember_edit(
                        message::tr("Companion level"),
                        vec![PendingChange::new(
                            format!("HenchmanList[{struct_index}].Level ({name})"),
                            previous.to_string(),
                            level.to_string(),
                        )],
                        None,
                    );
                }
                self.message = Message::success(message::fill(
                    "Companion level set to {level}.",
                    &[("level", &level.to_string())],
                ));
            }
            Err(error) => {
                self.message = Message::error(message::fill(
                    "Cannot change companion: {error}",
                    &[("error", &error.to_string())],
                ));
            }
        }
        self.refresh_dirty();
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
                    message::tr("unavailable")
                } else {
                    message::tr("available")
                };
                self.remember_edit(
                    message::tr("Companion availability"),
                    vec![PendingChange::new(
                        format!("HenchmanList[{struct_index}].Available ({name})"),
                        (!available).to_string(),
                        available.to_string(),
                    )],
                    None,
                );
                self.message = Message::success(message::fill(
                    "{name} is now {state}.",
                    &[("name", &name), ("state", state)],
                ));
            }
            Err(error) => {
                self.message = Message::error(message::fill(
                    "Cannot change companion: {error}",
                    &[("error", &error.to_string())],
                ));
            }
        }
        self.refresh_dirty();
    }

    /// Reports a validation problem in the status line.
    pub fn set_error(&mut self, text: &str) {
        self.message = Message::error(text);
    }

    /// Reports a neutral note in the status line.
    ///
    /// Used for things the user should see but that are neither an error nor a
    /// completed save, for example closing the folder picker without choosing.
    pub fn inform(&mut self, text: &str) {
        self.message = Message::info(text);
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

/// Converts core field changes into the review screen's rows.
fn pending_from_core(changes: &[Change]) -> Vec<PendingChange> {
    changes
        .iter()
        .map(|change| {
            PendingChange::new(
                change.field.clone(),
                change.before.clone(),
                change.after.clone(),
            )
        })
        .collect()
}

/// Formats one predefined gem effect as `+2 Body` or `+1 Fire (5s)`.
fn describe_gem_effect(game: &GameData, kind: u32, value: u32, duration: f32) -> String {
    let name = game.effect_name(kind).map_or_else(
        || message::fill("Effect {kind}", &[("kind", &kind.to_string())]),
        ToOwned::to_owned,
    );
    let signed = i64::from(value);
    if duration > 0.0 {
        format!("{signed:+} {name} ({duration:.0}s)")
    } else {
        format!("{signed:+} {name}")
    }
}

/// Formats an effect that only exists in the savegame, without game data names.
fn describe_unsaved_effect(effect: &jadeforge_core::GemEffect) -> String {
    let value = format!("{:+}", effect.value);
    message::fill(
        "{value} effect {kind}",
        &[("value", &value), ("kind", &effect.kind.to_string())],
    )
}

/// Formats one gem effect as `+2 Body` or `-1 Fire (-5s)`.
fn describe_effect(game: &GameData, effect: &jadeforge_core::GemEffect) -> String {
    let name = game
        .effect_name(u32::try_from(effect.kind).unwrap_or(u32::MAX))
        .map_or_else(
            || message::fill("Effect {kind}", &[("kind", &effect.kind.to_string())]),
            ToOwned::to_owned,
        );
    if effect.duration > 0.0 {
        format!("{:+} {} ({:.0}s)", effect.value, name, effect.duration)
    } else {
        format!("{:+} {}", effect.value, name)
    }
}

/// Status to show when no installation is available.
fn bundled_game_status(name_index: Option<&'static NameIndex>) -> GameStatus {
    match name_index {
        Some(names) if names.item_count() > 0 || names.style_count() > 0 => GameStatus::NamesOnly {
            items: names.item_count(),
            styles: names.style_count(),
        },
        _ => GameStatus::Missing,
    }
}

/// Describes why `directory` cannot be scanned as a save folder.
///
/// An existing but empty folder is not a problem: the scan reports that it
/// holds no savegames, which is a different state from a missing folder.
fn directory_problem(directory: &Path) -> Option<String> {
    match std::fs::metadata(directory) {
        Err(error) => Some(match error.kind() {
            std::io::ErrorKind::NotFound => {
                let directory = directory.display().to_string();
                message::fill(
                    "The save folder {directory} does not exist. Check the path or use Browse.",
                    &[("directory", &directory)],
                )
            }
            std::io::ErrorKind::PermissionDenied => {
                let directory = directory.display().to_string();
                message::fill(
                    "The save folder {directory} cannot be read: permission denied.",
                    &[("directory", &directory)],
                )
            }
            _ => {
                let directory = directory.display().to_string();
                message::fill(
                    "The save folder {directory} cannot be read: {error}.",
                    &[("directory", &directory), ("error", &error.to_string())],
                )
            }
        }),
        Ok(metadata) if !metadata.is_dir() => {
            let directory = directory.display().to_string();
            Some(message::fill(
                "{directory} is a file, not a save folder.",
                &[("directory", &directory)],
            ))
        }
        Ok(_) => std::fs::read_dir(directory).err().map(|error| {
            let directory = directory.display().to_string();
            message::fill(
                "The save folder {directory} cannot be listed: {error}.",
                &[("directory", &directory), ("error", &error.to_string())],
            )
        }),
    }
}

/// True when both paths name the same folder, resolving symlinks when possible.
fn same_directory(left: &Path, right: &Path) -> bool {
    if left == right {
        return true;
    }
    match (std::fs::canonicalize(left), std::fs::canonicalize(right)) {
        (Ok(left), Ok(right)) => left == right,
        _ => false,
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
            modified: std::fs::metadata(&path)
                .ok()
                .and_then(|meta| meta.modified().ok()),
            error: None,
        },
        Err(error) => SaveEntry {
            path: folder.to_path_buf(),
            label,
            character: None,
            level: None,
            time_played: 0,
            modified: std::fs::metadata(&path)
                .ok()
                .and_then(|meta| meta.modified().ok()),
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
    fn a_missing_startup_folder_is_reported_as_missing() {
        let session = Session::with_directory("/definitely/not/a/save/directory");
        assert_eq!(session.entries.len(), 0, "no savegames expected here");
        assert!(
            session.message.text.contains("does not exist"),
            "{}",
            session.message.text
        );
    }

    #[test]
    fn loading_game_data_restores_the_previous_test_root() {
        let root = PathBuf::from("/nonexistent/jadeforge-game-root");
        jadeforge_gamedata::set_test_game_root(Some(root.clone()));
        let _ = Session::empty("/definitely/not/a/save/directory");
        assert_eq!(
            jadeforge_gamedata::test_game_root(),
            Some(root),
            "a probe session cleared the fixture's game root"
        );
        jadeforge_gamedata::set_test_game_root(None);
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
            modified: None,
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
        let directory = TempDir::new("empty-scan");
        let mut session = Session::pending(directory.path());
        assert!(matches!(session.game_status(), GameStatus::Loading));
        let request = session.begin_scan(directory.path()).expect("begin");
        assert!(session.is_scanning());
        let outcome = ScanOutcome::run(&request);
        assert!(matches!(
            session.complete_scan(&request, outcome),
            ScanApply::Applied(ScanCompletion {
                directory_changed: false
            })
        ));
        assert!(!session.is_scanning());
        assert!(session.message.text.contains("No savegames found"));
    }

    #[test]
    fn a_worker_scan_keeps_the_save_for_the_same_folder() {
        let (directory, mut session) = fixture();
        let path = session.current_path();
        let request = session.begin_scan(directory.path()).expect("begin");
        assert!(session.save.is_some(), "the save disappears while scanning");
        let applied = session.complete_scan(&request, ScanOutcome::run(&request));
        let ScanApply::Applied(completion) = applied else {
            panic!("the scan was not applied: {applied:?}");
        };
        assert!(!completion.directory_changed);
        assert_eq!(session.selected(), Some(0));
        assert_eq!(session.current_path(), path);
        assert!(session.character.is_some());
    }

    #[test]
    fn a_new_folder_scan_clears_the_save_only_when_it_completes() {
        let (_directory, mut session) = fixture();
        let other = TempDir::new("new-folder");
        let request = session.begin_scan(other.path()).expect("begin");
        assert!(
            session.save.is_some(),
            "the old save must stay visible while the scan runs"
        );
        let applied = session.complete_scan(&request, ScanOutcome::run(&request));
        let ScanApply::Applied(completion) = applied else {
            panic!("the scan was not applied: {applied:?}");
        };
        assert!(completion.directory_changed);
        assert_eq!(session.directory(), other.path());
        assert!(session.save.is_none());
        assert!(session.selected.is_none());
    }

    #[test]
    fn a_cancelled_scan_keeps_the_previous_entries_and_save() {
        let (directory, mut session) = fixture();
        let other = TempDir::new("cancel-other");
        fs::copy(
            directory.path().join("save.sac"),
            other.path().join("save.sac"),
        )
        .expect("copy save");
        let request = session.begin_scan(other.path()).expect("begin");
        assert!(session.is_scanning());
        assert!(session.cancel_scan());
        assert!(!session.is_scanning());
        assert_eq!(session.directory(), directory.path());
        assert_eq!(session.selected(), Some(0));
        assert_eq!(session.entries().len(), 1);
        assert!(session.message().text.contains("cancelled"));

        // The worker result that arrives afterwards must be discarded.
        let applied = session.complete_scan(&request, ScanOutcome::run(&request));
        assert_eq!(applied, ScanApply::Stale);
        assert_eq!(session.directory(), directory.path());
        assert!(session.save.is_some());
        assert!(!session.is_scanning());
    }

    #[test]
    fn a_failed_scan_keeps_the_previous_entries_and_save() {
        let (directory, mut session) = fixture();
        let other = TempDir::new("failed-other");
        fs::copy(
            directory.path().join("save.sac"),
            other.path().join("save.sac"),
        )
        .expect("copy save");
        let request = session.begin_scan(other.path()).expect("begin");
        fs::remove_dir_all(other.path()).expect("remove folder");
        let applied = session.complete_scan(&request, ScanOutcome::run(&request));
        assert_eq!(applied, ScanApply::Failed);
        assert!(!session.is_scanning());
        assert_eq!(session.directory(), directory.path());
        assert_eq!(session.selected(), Some(0));
        assert_eq!(session.entries().len(), 1);
        assert!(
            session.message().text.contains("does not exist"),
            "{}",
            session.message().text
        );
    }

    #[test]
    fn a_worker_failure_reports_without_clearing() {
        let (directory, mut session) = fixture();
        let other = TempDir::new("panic-other");
        let request = session.begin_scan(other.path()).expect("begin");
        assert!(session.fail_scan(&request));
        assert!(!session.is_scanning());
        assert_eq!(session.directory(), directory.path());
        assert!(session.save.is_some());
        assert!(
            session.message().text.contains("Could not scan"),
            "{}",
            session.message().text
        );
        // The request is no longer current.
        assert!(!session.fail_scan(&request));
    }

    #[test]
    fn a_scan_persists_the_folder_only_when_it_completes() {
        crate::config::ensure_test_path();
        let (directory, mut session) = fixture();
        let mut config = crate::config::Config::load();
        config.save_dir = Some(directory.path().to_path_buf());
        config.store().expect("store settings");

        let other = TempDir::new("persist-other");
        let request = session.begin_scan(other.path()).expect("begin");
        assert_eq!(
            crate::config::Config::load().save_dir,
            Some(directory.path().to_path_buf()),
            "beginning a scan must not replace the remembered folder"
        );
        let applied = session.complete_scan(&request, ScanOutcome::run(&request));
        assert!(matches!(applied, ScanApply::Applied(_)));
        assert_eq!(
            crate::config::Config::load().save_dir,
            Some(other.path().to_path_buf())
        );
    }

    #[test]
    fn begin_scan_rejects_dirty_and_unreadable_folders() {
        let (directory, mut session) = fixture();
        let index = session.items[0].struct_index;
        session.adjust_item_count(index, 1);
        assert!(session.begin_scan(directory.path()).is_none());
        assert!(
            session.message().text.contains("unsaved"),
            "{}",
            session.message().text
        );
        assert!(!session.is_scanning());

        let mut clean = Session::empty("/definitely/not/here");
        assert!(clean.begin_scan("/definitely/not/here").is_none());
        assert!(
            clean.message().text.contains("does not exist"),
            "{}",
            clean.message().text
        );
        assert!(
            clean.begin_scan("").is_none(),
            "an empty path is not a scan"
        );
        assert!(
            clean.message().text.contains("Choose the folder"),
            "{}",
            clean.message().text
        );
        assert!(!clean.is_scanning());
    }

    #[test]
    fn pending_without_a_folder_asks_for_one() {
        let session = Session::pending("");
        assert!(matches!(session.game_status(), GameStatus::Missing));
        assert!(
            session.message().text.contains("Choose the folder"),
            "{}",
            session.message().text
        );
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
    fn plural_counts_use_the_singular_only_for_one() {
        assert_eq!(format::plural(0, "gem", "gems"), "0 gems");
        assert_eq!(format::plural(1, "gem", "gems"), "1 gem");
        assert_eq!(format::plural(2, "socket", "sockets"), "2 sockets");
    }

    #[test]
    fn friendly_labels_never_leak_raw_ids() {
        assert_eq!(item_category_label("plot_items"), "Plot items");
        assert_eq!(item_category_label("improve"), "Improvements");
        assert_eq!(item_category_label("mgweapons"), "Magic weapons");
        assert_eq!(item_category_label("powerups"), "Power-ups");
        assert_eq!(item_category_label("some_new_kind"), "Some New Kind");
        assert_eq!(style_kind_label("martial"), "Martial");
        assert_eq!(style_kind_label("transformation"), "Transformation");
        assert_eq!(style_kind_label("other"), "Other");
    }

    #[test]
    fn the_sort_labels_use_the_same_time_wording_as_the_rows() {
        assert_eq!(SaveSort::Played.label(), "Time played");
        assert_eq!(SaveSort::Played.key(), "played");
        assert_eq!(SaveSort::from_key("played"), Some(SaveSort::Played));
    }

    #[test]
    fn item_categories_filter_the_picker() {
        let (_directory, session) = fixture();
        let categories = session.item_categories();
        assert!(
            categories.contains(&"gems".to_owned()),
            "the bundled names have no gem category: {categories:?}"
        );
        let (all, all_total) = session.search_items("", 500);
        let (gems, gems_total) = session.search_items_filtered("", Some("gems"), 500);
        assert!(all_total >= gems_total);
        assert_eq!(gems.len(), gems_total);
        assert!(!gems.is_empty());
        assert!(gems.iter().all(|item| item.category == "gems"));
        assert!(all.len() >= gems.len());
        let (missing, missing_total) =
            session.search_items_filtered("", Some("not-a-category"), 10);
        assert!(missing.is_empty());
        assert_eq!(missing_total, 0);
        // The unfiltered search is exactly the filtered one without a category.
        let (unfiltered, unfiltered_total) = session.search_items("", 10);
        assert_eq!(unfiltered.len(), all.len().min(10));
        assert_eq!(unfiltered_total, all_total);
    }

    #[test]
    fn a_fresh_save_has_no_backup_to_show() {
        let (_directory, session) = fixture();
        assert!(session.last_backup().is_none());
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
        let backup = session
            .last_backup()
            .expect("a save must surface its backup path")
            .to_path_buf();
        assert!(backup.exists(), "the surfaced backup does not exist");
        assert!(
            session.message.text.contains("jadeforge-backup"),
            "the save message does not name the backup: {}",
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
        assert!(session.message.text.contains("Nothing changed"));
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
            session.message.text.contains("nothing to apply"),
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
            session.message.text.contains("6 items in the stack"),
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
        assert!(session.message.text.contains("not in the game data"));

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
        let first = session.begin_scan(directory.path()).expect("first");
        let second = session.begin_scan(directory.path()).expect("second");
        assert_ne!(first.token(), second.token());
        assert_eq!(
            session.complete_scan(&first, ScanOutcome::run(&first)),
            ScanApply::Stale
        );
        assert!(
            matches!(
                session.complete_scan(&second, ScanOutcome::run(&second)),
                ScanApply::Applied(_)
            ),
            "the newest request should have been applied"
        );
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
            session.message.text.contains("Inventory"),
            "{}",
            session.message.text
        );
        assert_eq!(
            session
                .section_error(Section::Items)
                .map(|error| error.section),
            Some(Section::Items)
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
    #[test]
    fn dirty_is_a_content_comparison_not_a_sticky_flag() {
        let (_directory, mut session) = fixture();
        assert!(!session.is_dirty());
        let character = session.character.as_ref().expect("character").clone();

        let mut form = CharacterForm::from_character(&character);
        form.level = "9".to_owned();
        assert!(session.apply_form_silently(&form));
        assert!(session.is_dirty(), "an edit must mark the save dirty");

        form.level = character.level.to_string();
        assert!(session.apply_form_silently(&form));
        assert!(
            !session.is_dirty(),
            "restoring the original value must clear the dirty state"
        );
    }

    #[test]
    fn scanning_the_same_folder_keeps_the_save_open() {
        let (directory, mut session) = fixture();
        let path = session.current_path();
        assert!(session.set_directory(directory.path().to_path_buf()));
        assert_eq!(session.selected(), Some(0), "{}", session.message.text);
        assert_eq!(session.current_path(), path);
        assert!(session.character.is_some());
    }

    #[test]
    fn scanning_another_folder_clears_the_save() {
        let (_directory, mut session) = fixture();
        let other = TempDir::new("other");
        assert!(session.set_directory(other.path()));
        assert_eq!(session.selected(), None);
        assert!(session.character.is_none());
        assert_eq!(session.directory(), other.path());
    }

    #[test]
    fn a_missing_folder_is_rejected_and_not_persisted() {
        let (directory, mut session) = fixture();
        crate::config::ensure_test_path();
        let original = session.directory().to_path_buf();
        let mut config = crate::config::Config::load();
        config.save_dir = Some(original.clone());
        config.store().expect("store settings");

        let missing = directory.path().join("does-not-exist");
        assert!(!session.set_directory(missing));
        assert_eq!(session.directory(), original);
        assert!(
            session.message().text.contains("does not exist"),
            "{}",
            session.message().text
        );
        assert_eq!(
            crate::config::Config::load().save_dir,
            Some(original),
            "a rejected scan must not replace the remembered folder"
        );
    }

    #[test]
    fn a_file_is_not_a_save_folder() {
        let (directory, mut session) = fixture();
        let file = directory.path().join("not-a-folder");
        fs::write(&file, b"x").expect("write file");
        let original = session.directory().to_path_buf();
        assert!(!session.set_directory(file));
        assert_eq!(session.directory(), original);
        assert!(
            session.message().text.contains("is a file"),
            "{}",
            session.message().text
        );
    }

    #[test]
    fn an_empty_folder_is_reported_as_empty() {
        let (_directory, mut session) = fixture();
        let empty = TempDir::new("empty");
        assert!(session.set_directory(empty.path()));
        assert_eq!(session.entries.len(), 0);
        assert!(
            session.message.text.contains("No savegames found"),
            "{}",
            session.message.text
        );
    }

    #[test]
    fn a_valid_edit_saves_while_the_form_is_invalid() {
        let (_directory, mut session) = fixture();
        let path = session.current_path().expect("path");
        let item_index = session.items[0].struct_index;
        let before = session.items[0].count;
        session.adjust_item_count(item_index, 1);
        let character = session.character.as_ref().expect("character").clone();
        let mut form = CharacterForm::from_character(&character);
        form.level = "abc".to_owned();

        session.save_applied(&form).expect("the applied edit saves");
        assert!(
            session
                .message
                .text
                .contains("invalid character field was not applied"),
            "{}",
            session.message.text
        );
        assert!(!session.is_dirty());

        let reopened = SaveGame::open(&path).expect("reopen");
        assert_eq!(
            reopened.items().expect("items")[0].count,
            before + 1,
            "the item edit was not written"
        );
        assert_eq!(
            reopened.character().expect("character").level,
            character.level,
            "the invalid level was written"
        );
    }

    #[test]
    fn valid_fields_apply_even_when_another_field_is_invalid() {
        let (_directory, mut session) = fixture();
        let character = session.character.as_ref().expect("character").clone();
        let mut form = CharacterForm::from_character(&character);
        form.level = "abc".to_owned();
        form.gold = "1234".to_owned();

        assert!(session.apply_form_silently(&form));
        let updated = session.character.as_ref().expect("character");
        assert_eq!(updated.gold, Some(1234));
        assert_eq!(updated.level, character.level, "the invalid level applied");

        let errors = session.form_errors(&form);
        assert_eq!(errors.len(), 1, "{errors:?}");
        assert_eq!(errors[0].field, CharacterField::Level);
        assert!(errors[0].message.contains("level"), "{}", errors[0].message);
    }

    #[test]
    fn out_of_range_fields_report_their_allowed_range() {
        let character = character();
        let mut form = CharacterForm::from_character(&character);
        form.level = "0".to_owned();
        form.health_max = "0".to_owned();
        let errors = form.validate_against(&character);
        assert_eq!(errors.len(), 2, "{errors:?}");
        assert_eq!(errors[0].field, CharacterField::Level);
        assert!(
            errors[0].message.contains("1 to 255"),
            "{}",
            errors[0].message
        );
        assert_eq!(errors[1].field, CharacterField::HealthMax);
        assert!(form.to_character(&character).is_err());
        assert_eq!(
            CharacterField::Level.range(),
            "1 to 255 (level)",
            "the UI shows the range and the unit next to the field"
        );
        assert!(
            CharacterField::HealthMax.range().contains("health points"),
            "health ranges must carry their unit: {}",
            CharacterField::HealthMax.range()
        );
        assert!(
            CharacterField::Chapter.range().contains("chapter"),
            "the chapter range must say what the number counts: {}",
            CharacterField::Chapter.range()
        );
    }

    #[test]
    fn a_failed_section_is_reported_apart_from_an_empty_list() {
        let (directory, _session) = fixture();
        let path = directory.path().join("save.sac");
        {
            let mut save = SaveGame::open(&path).expect("open fixture");
            let field = save
                .block_mut("save.sav")
                .expect("save block")
                .gff_mut()
                .get_mut("PlayerCreature.ItemList")
                .expect("item list");
            field.type_id = 0;
            field.value = Value::Byte(1);
            save.save(SaveOptions { backup: false })
                .expect("write fixture");
        }
        let mut session = Session::with_directory(directory.path());
        assert!(session.open(0));
        let error = session
            .section_error(Section::Items)
            .expect("the inventory section should be reported unreadable");
        assert_eq!(error.section, Section::Items);
        assert!(session.items().is_empty());
        assert!(session.section_error(Section::Styles).is_none());
    }

    #[test]
    fn a_readable_save_has_no_section_errors() {
        let (_directory, session) = fixture();
        assert!(session.section_error(Section::Items).is_none());
        assert!(session.section_error(Section::Character).is_none());
    }

    #[test]
    fn a_user_installation_is_rejected_when_it_is_not_a_game() {
        let (_directory, mut session) = fixture();
        let missing = std::env::temp_dir().join("jadeforge-not-a-game-installation");
        assert!(!session.set_game_directory(missing));
        assert!(
            session.message.text.contains("not a readable"),
            "{}",
            session.message.text
        );
    }

    #[test]
    fn restoring_the_item_count_clears_dirty() {
        let (_directory, mut session) = fixture();
        let index = session.items[0].struct_index;
        let before = session.items[0].count;
        session.adjust_item_count(index, 1);
        assert!(session.is_dirty(), "{}", session.message.text);
        session.adjust_item_count(index, -1);
        assert_eq!(session.items[0].count, before);
        assert!(
            !session.is_dirty(),
            "restoring the count left the save dirty: {}",
            session.message.text
        );
    }
    #[test]
    fn timestamps_are_formatted_in_utc() {
        use std::time::{Duration, UNIX_EPOCH};
        assert_eq!(format::timestamp(UNIX_EPOCH), "1970-01-01 00:00 UTC");
        assert_eq!(
            format::timestamp(UNIX_EPOCH + Duration::from_secs(1_700_000_000)),
            "2023-11-14 22:13 UTC"
        );
    }
    #[test]
    fn a_rejected_startup_scan_does_not_stay_loading() {
        let mut session = Session::pending("/definitely/not/here");
        assert!(matches!(session.game_status(), GameStatus::Loading));
        assert!(session.begin_scan("/definitely/not/here").is_none());
        assert!(
            !matches!(session.game_status(), GameStatus::Loading),
            "a rejected startup scan left the loading state behind"
        );
    }

    #[test]
    fn a_cancelled_startup_scan_does_not_stay_loading() {
        let directory = TempDir::new("cancel-startup");
        let mut session = Session::pending(directory.path());
        assert!(matches!(session.game_status(), GameStatus::Loading));
        let request = session.begin_scan(directory.path()).expect("begin");
        assert!(session.cancel_scan());
        assert!(!session.is_scanning());
        assert!(
            !matches!(session.game_status(), GameStatus::Loading),
            "a cancelled startup scan left the loading state behind"
        );
        assert_eq!(
            session.complete_scan(&request, ScanOutcome::run(&request)),
            ScanApply::Stale
        );
    }

    #[test]
    fn undo_and_redo_restore_character_fields_and_the_dirty_state() {
        let (_directory, mut session) = fixture();
        let mut form = form_for(&session);
        form.level = "9".to_owned();
        let mut character = form
            .to_character(session.character.as_ref().expect("character"))
            .expect("character");
        assert!(session.apply(&form).is_ok());
        assert_eq!(session.character.as_ref().expect("character").level, 9);
        assert!(session.is_dirty());
        assert!(session.can_undo());

        character.level = 10;
        form.level = "10".to_owned();
        assert!(session.apply(&form).is_ok());
        assert_eq!(session.character.as_ref().expect("character").level, 10);

        assert!(session.undo());
        assert_eq!(session.character.as_ref().expect("character").level, 9);
        assert!(session.undo());
        assert_eq!(session.character.as_ref().expect("character").level, 7);
        assert!(
            !session.is_dirty(),
            "undoing every edit must clear the dirty state"
        );
        assert!(!session.can_undo());

        assert!(session.redo());
        assert_eq!(session.character.as_ref().expect("character").level, 9);
        assert!(session.redo());
        assert_eq!(session.character.as_ref().expect("character").level, 10);
        assert!(session.is_dirty());
    }

    #[test]
    fn consecutive_character_typing_is_one_undo_step() {
        let (_directory, mut session) = fixture();
        let mut form = form_for(&session);
        form.level = "8".to_owned();
        assert!(session.apply_form_silently(&form));
        form.level = "80".to_owned();
        assert!(session.apply_form_silently(&form));
        assert_eq!(session.character.as_ref().expect("character").level, 80);
        assert!(session.undo());
        assert_eq!(
            session.character.as_ref().expect("character").level,
            7,
            "typing two digits must coalesce into one undo step"
        );
    }

    #[test]
    fn pending_changes_list_the_fields_and_clear_after_save() {
        let (_directory, mut session) = fixture();
        let mut form = form_for(&session);
        form.level = "9".to_owned();
        assert!(session.apply(&form).is_ok());
        let groups = session.pending_changes();
        assert_eq!(groups.len(), 1);
        assert!(
            groups[0]
                .changes
                .iter()
                .any(|change| change.field.contains("Level")
                    && change.before == "7"
                    && change.after == "9"),
            "the diff does not describe the level edit: {groups:?}"
        );
        session.save().expect("save");
        assert!(session.pending_changes().is_empty());
        assert!(!session.can_undo());
    }

    #[test]
    fn undoing_an_item_stack_change_restores_the_count() {
        let (_directory, mut session) = fixture();
        let index = session.items[0].struct_index;
        session.adjust_item_count(index, 2);
        assert_eq!(session.items[0].count, 7);
        assert!(session.undo());
        assert_eq!(session.items[0].count, 5);
        assert!(!session.is_dirty());
    }

    #[test]
    fn an_external_change_is_detected_and_a_dirty_reload_is_refused() {
        let (directory, mut session) = fixture();
        assert!(!session.check_disk_state());
        let path = directory.path().join("save.sac");
        let mut gff = fixture_gff();
        gff.get_mut("PlayerCreature.Level").expect("level").value = Value::Byte(42);
        std::thread::sleep(std::time::Duration::from_millis(10));
        fs::write(&path, container(&gff)).expect("write external change");
        assert!(session.check_disk_state(), "the external change was missed");
        assert!(session.disk_changed());

        let mut form = form_for(&session);
        form.level = "9".to_owned();
        assert!(session.apply(&form).is_ok());
        assert!(session.reload_from_disk().is_err());
        assert!(
            session.message.text.contains("unsaved changes"),
            "the refusal does not explain itself: {}",
            session.message.text
        );

        assert!(session.undo());
        assert!(session.reload_from_disk().is_ok());
        assert_eq!(session.character.as_ref().expect("character").level, 42);
        assert!(!session.disk_changed());
    }

    #[test]
    fn a_backup_can_be_listed_and_restored() {
        let (_directory, mut session) = fixture();
        let mut form = form_for(&session);
        form.level = "9".to_owned();
        assert!(session.apply(&form).is_ok());
        let saved = session.save().expect("save");
        let backups = session.backups();
        assert_eq!(backups.len(), 1, "the save did not create a backup");
        assert_eq!(backups[0].original, saved);

        assert!(session.restore_backup(&backups[0].path).is_ok());
        assert_eq!(
            session.character.as_ref().expect("character").level,
            7,
            "restoring the backup did not restore the pre-save file"
        );
        assert!(
            session.last_backup().is_some(),
            "the safety copy is missing"
        );
        assert_eq!(session.backups().len(), 2);
    }

    #[test]
    fn restoring_a_backup_with_unsaved_changes_is_refused() {
        let (_directory, mut session) = fixture();
        let mut form = form_for(&session);
        form.level = "9".to_owned();
        assert!(session.apply(&form).is_ok());
        session.save().expect("save");
        let backup = session.backups().remove(0).path;
        form.level = "11".to_owned();
        assert!(session.apply(&form).is_ok());
        assert!(session.restore_backup(&backup).is_err());
        assert_eq!(session.character.as_ref().expect("character").level, 11);
    }

    #[test]
    fn content_fingerprints_match_equal_blocks_and_split_different_ones() {
        assert_eq!(
            ContentFingerprint::of(b"canonical block"),
            ContentFingerprint::of(b"canonical block")
        );
        assert_ne!(
            ContentFingerprint::of(b"canonical block"),
            ContentFingerprint::of(b"canonical block ")
        );
    }
}
