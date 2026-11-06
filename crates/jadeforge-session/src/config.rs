//! The small settings file `JadeForge` keeps between runs.
//!
//! It is TOML (`sort = "played"`), written on a best effort basis: a failed
//! write never stops the editor from working. Settings files written by older
//! versions used a plain `key=value` format; those are still read and are
//! rewritten as TOML on the next save.

use std::cell::RefCell;
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

use serde::{Deserialize, Serialize};

use crate::locale::Locale;
use crate::preferences::{ThemeChoice, normalize_text_scale};

/// Distinguishes the temporary settings directories created by tests.
static TEST_PATH_COUNTER: AtomicUsize = AtomicUsize::new(0);
static STORE_COUNTER: AtomicUsize = AtomicUsize::new(0);

thread_local! {
    /// Settings path used by the UI test harness. Thread local so parallel tests
    /// never share (or clobber) each other's settings.
    static TEST_PATH: RefCell<Option<PathBuf>> = const { RefCell::new(None) };
}

/// Returns a fresh, empty settings path for the current test thread.
///
/// The first call per thread creates the directory and remembers the path; later
/// calls return the same one, so a test can relaunch the app and still see the
/// settings it wrote. While an override exists, `JADEFORGE_CONFIG_DIR` and the
/// user's real settings file are ignored, which keeps a developer's saved sort
/// order out of the tests.
#[doc(hidden)]
pub fn ensure_test_path() -> PathBuf {
    TEST_PATH.with(|cell| {
        let mut slot = cell.borrow_mut();
        if let Some(path) = slot.as_ref() {
            return path.clone();
        }
        let id = TEST_PATH_COUNTER.fetch_add(1, Ordering::Relaxed);
        let directory =
            std::env::temp_dir().join(format!("jadeforge-config-test-{}-{id}", std::process::id()));
        let path = directory.join("config");
        let _ = std::fs::create_dir_all(&directory);
        *slot = Some(path.clone());
        path
    })
}

/// Window rectangle remembered between runs, in logical pixels.
///
/// The position is optional because Wayland compositors do not expose where a
/// window was placed; a missing position lets the compositor place it.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct WindowGeometry {
    /// Window width in logical pixels.
    pub width: u32,
    /// Window height in logical pixels.
    pub height: u32,
    /// Horizontal position in logical pixels.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub x: Option<i32>,
    /// Vertical position in logical pixels.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub y: Option<i32>,
}

impl WindowGeometry {
    /// True when the remembered size is usable.
    #[must_use]
    pub fn is_valid(&self) -> bool {
        self.width >= 1 && self.height >= 1
    }
}

/// Settings persisted between runs.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    /// Folder that was scanned last.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub save_dir: Option<PathBuf>,
    /// Sort key of the savegame list, see `SaveSort::key`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sort: Option<String>,
    /// Whether the savegame list is sorted descending.
    pub sort_descending: bool,
    /// Game installation the user pointed `JadeForge` at.
    ///
    /// `JADEFORGE_GAME_DIR` still wins; this keeps a user-chosen installation
    /// for the next start without hardcoding a machine path in the source.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub game_dir: Option<PathBuf>,
    /// Size and position of the main window.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub window: Option<WindowGeometry>,
    /// Colour theme selected in the About page.
    #[serde(default)]
    pub theme: ThemeChoice,
    /// Text and layout scale in percent, see [`Config::normalized_text_scale`].
    #[serde(default = "default_text_scale")]
    pub text_scale: u16,
    /// Interface language; unsupported languages fall back to English.
    #[serde(default)]
    pub locale: Locale,
    /// Whether the window size and position are remembered between runs.
    #[serde(default = "default_true")]
    pub remember_window: bool,
}

/// Default for boolean settings that are on unless the user turns them off.
#[must_use]
pub fn default_true() -> bool {
    true
}

/// Default text scale in percent.
#[must_use]
pub fn default_text_scale() -> u16 {
    100
}

impl Default for Config {
    fn default() -> Self {
        Self {
            save_dir: None,
            sort: None,
            sort_descending: false,
            game_dir: None,
            window: None,
            theme: ThemeChoice::default(),
            text_scale: default_text_scale(),
            locale: Locale::default(),
            remember_window: default_true(),
        }
    }
}

impl Config {
    /// Theme and text scale after clamping the stored scale to the tested range.
    #[must_use]
    pub fn ui_preferences(&self) -> (ThemeChoice, u16) {
        (self.theme, self.normalized_text_scale())
    }

    /// Text scale after validation; an unsupported stored value becomes 100.
    #[must_use]
    pub fn normalized_text_scale(&self) -> u16 {
        normalize_text_scale(self.text_scale)
    }

    /// Persists the interface language for the next start (best effort).
    pub fn remember_locale(locale: Locale) {
        let mut config = Self::load();
        config.locale = locale;
        let _ = config.store();
    }

    /// Persists whether the window rectangle is remembered (best effort).
    pub fn remember_window_preference(remember: bool) {
        let mut config = Self::load();
        config.remember_window = remember;
        let _ = config.store();
    }

    /// Persists the theme and text scale for the next start (best effort).
    pub fn remember_ui_preferences(theme: ThemeChoice, text_scale: u16) {
        let mut config = Self::load();
        config.theme = theme;
        config.text_scale = normalize_text_scale(text_scale);
        let _ = config.store();
    }

    /// Remembers the window rectangle for the next start (best effort).
    pub fn remember_window(geometry: WindowGeometry) {
        if !geometry.is_valid() {
            return;
        }
        let mut config = Self::load();
        config.window = Some(geometry);
        let _ = config.store();
    }

    /// Loads the settings, falling back to the defaults when the file is
    /// missing or unreadable.
    #[must_use]
    pub fn load() -> Self {
        Self::load_from(&Self::path())
    }

    /// Loads the settings from an explicit file.
    #[must_use]
    pub fn load_from(path: &Option<PathBuf>) -> Self {
        let Some(path) = path.as_ref() else {
            return Self::default();
        };
        let Ok(text) = std::fs::read_to_string(path) else {
            return Self::default();
        };
        Self::parse(&text)
    }

    /// Parses TOML, with a fallback for the older `key=value` settings file.
    #[must_use]
    pub fn parse(text: &str) -> Self {
        if let Ok(config) = toml::from_str::<Config>(text) {
            return config;
        }
        if looks_like_legacy_settings(text) {
            return Self::parse_legacy(text);
        }
        Self::default()
    }

    /// Parses the `key=value` format of older versions.
    fn parse_legacy(text: &str) -> Self {
        let mut config = Self::default();
        for line in text.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let Some((key, value)) = line.split_once('=') else {
                continue;
            };
            match key.trim() {
                "save_dir" => {
                    let value = unquote(value.trim());
                    config.save_dir = (!value.is_empty()).then(|| PathBuf::from(value));
                }
                "sort" => {
                    let value = unquote(value.trim());
                    config.sort = (!value.is_empty()).then(|| value.to_owned());
                }
                "sort_descending" => {
                    config.sort_descending = matches!(value.trim(), "1" | "true" | "yes");
                }
                _ => {}
            }
        }
        config
    }

    /// Renders the settings file.
    ///
    /// A path that cannot be represented as TOML yields only the header here;
    /// [`Config::store`] refuses to write such a file instead of erasing the
    /// previous settings.
    #[must_use]
    pub fn to_text(&self) -> String {
        self.render()
            .unwrap_or_else(|_| "# JadeForge settings\n".to_owned())
    }

    fn render(&self) -> Result<String, toml::ser::Error> {
        let body = toml::to_string_pretty(self)?;
        Ok(format!("# JadeForge settings\n{body}"))
    }

    /// Writes the settings, creating the configuration directory if needed.
    ///
    /// The file is written to an exclusive temporary sibling and renamed into
    /// place, so a crash cannot truncate the previous settings.
    ///
    /// # Errors
    ///
    /// Returns the underlying I/O error when the directory or file cannot be
    /// written.
    pub fn store(&self) -> std::io::Result<()> {
        let Some(path) = Self::path() else {
            return Err(std::io::Error::other("no configuration directory"));
        };
        let parent = path
            .parent()
            .ok_or_else(|| std::io::Error::other("no configuration directory"))?;
        std::fs::create_dir_all(parent)?;
        let text = self.render().map_err(std::io::Error::other)?;
        let counter = STORE_COUNTER.fetch_add(1, Ordering::Relaxed);
        let temporary = parent.join(format!(
            ".{}.tmp-{}-{counter}",
            path.file_name()
                .map_or("config", |name| name.to_str().unwrap_or("config")),
            std::process::id()
        ));
        let result = (|| -> std::io::Result<()> {
            let mut file = std::fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&temporary)?;
            file.write_all(text.as_bytes())?;
            file.sync_all()?;
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt as _;
                std::fs::set_permissions(&temporary, std::fs::Permissions::from_mode(0o600))?;
            }
            drop(file);
            std::fs::rename(&temporary, path)?;
            Ok(())
        })();
        if result.is_err() {
            let _ = std::fs::remove_file(&temporary);
        }
        result
    }

    /// Location of the settings file.
    ///
    /// `JADEFORGE_CONFIG_DIR` points straight at the directory that should hold
    /// the file, which keeps tests and portable installs out of the user's
    /// configuration.
    #[must_use]
    pub fn path() -> Option<PathBuf> {
        if let Some(path) = TEST_PATH.with(|cell| cell.borrow().clone()) {
            return Some(path);
        }
        if let Some(dir) =
            std::env::var_os("JADEFORGE_CONFIG_DIR").filter(|value| !value.is_empty())
        {
            return Some(PathBuf::from(dir).join("config"));
        }
        let xdg = std::env::var_os("XDG_CONFIG_HOME")
            .filter(|value| !value.is_empty())
            .map(PathBuf::from);
        let appdata = std::env::var_os("APPDATA")
            .filter(|value| !value.is_empty())
            .map(PathBuf::from);
        let home = std::env::var_os("HOME")
            .filter(|value| !value.is_empty())
            .map(PathBuf::from);
        let base = directory(xdg.as_deref(), appdata.as_deref(), home.as_deref())?;
        Some(base.join("jadeforge").join("config"))
    }
}

/// Removes one pair of surrounding quotes from a legacy value.
fn unquote(value: &str) -> &str {
    value
        .strip_prefix('"')
        .and_then(|value| value.strip_suffix('"'))
        .unwrap_or(value)
}

/// True when at least one line uses the old unquoted `key=value` format.
///
/// This is deliberately conservative: a settings file that parses as TOML never
/// reaches this check, and a TOML file with a syntax error keeps its values only
/// when it still looks like the legacy format.
fn looks_like_legacy_settings(text: &str) -> bool {
    text.lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .any(|line| {
            let Some((key, value)) = line.split_once('=') else {
                return false;
            };
            matches!(key.trim(), "save_dir" | "sort" | "sort_descending")
                && !value.trim().starts_with('"')
        })
}

/// Picks the configuration directory from the platform variables.
///
/// Split out from [`Config::path`] so it can be tested without touching the
/// environment.
#[must_use]
pub fn directory(
    xdg: Option<&Path>,
    appdata: Option<&Path>,
    home: Option<&Path>,
) -> Option<PathBuf> {
    if let Some(xdg) = xdg {
        return Some(xdg.to_path_buf());
    }
    if cfg!(target_os = "windows") {
        appdata.map(Path::to_path_buf)
    } else if cfg!(target_os = "macos") {
        home.map(|home| home.join("Library/Application Support"))
    } else {
        home.map(|home| home.join(".config"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn settings_round_trip() {
        let config = Config {
            save_dir: Some(PathBuf::from("/games/Jade Empire/Save")),
            sort: Some("played".to_owned()),
            sort_descending: true,
            game_dir: Some(PathBuf::from("/games/Jade Empire")),
            window: Some(WindowGeometry {
                width: 1180,
                height: 760,
                x: Some(40),
                y: Some(32),
            }),

            theme: ThemeChoice::Light,
            text_scale: 150,
            locale: Locale::Pseudo,
            remember_window: true,
        };
        let text = config.to_text();
        assert!(text.contains("save_dir = \"/games/Jade Empire/Save\""));
        assert!(text.contains("sort = \"played\""));
        assert!(text.contains("sort_descending = true"));
        assert!(text.contains("game_dir = \"/games/Jade Empire\""));
        assert!(text.contains("theme = \"light\""), "{text}");
        assert!(text.contains("locale = \"en-xa\""), "{text}");
        assert_eq!(Config::parse(&text), config);
    }

    #[test]
    fn the_window_rectangle_is_remembered_by_default() {
        let config = Config::parse("save_dir = \"/tmp/saves\"\n");
        assert!(config.remember_window);
        let config = Config {
            remember_window: false,
            ..Config::default()
        };
        assert_eq!(Config::parse(&config.to_text()), config);
    }

    #[test]
    fn old_settings_default_to_the_dark_theme_at_full_scale() {
        let config = Config::parse("save_dir = \"/tmp/saves\"\n");
        assert_eq!(config.theme, ThemeChoice::Dark);
        assert_eq!(config.normalized_text_scale(), 100);
        assert_eq!(config.ui_preferences(), (ThemeChoice::Dark, 100));
    }

    #[test]
    fn an_unsupported_stored_scale_falls_back_to_100() {
        let config = Config::parse("text_scale = 400\n");
        assert_eq!(config.text_scale, 400);
        assert_eq!(config.normalized_text_scale(), 100);
    }

    #[test]
    fn ui_preferences_are_written_and_read_back() {
        ensure_test_path();
        Config::remember_ui_preferences(ThemeChoice::HighContrast, 200);
        let config = Config::load();
        assert_eq!(config.theme, ThemeChoice::HighContrast);
        assert_eq!(config.normalized_text_scale(), 200);
        assert_eq!(config.to_text().matches("theme = ").count(), 1);
    }

    #[test]
    fn legacy_settings_are_still_read() {
        let config = Config::parse("save_dir=/tmp/saves\nsort=level\nsort_descending=1\n");
        assert_eq!(config.save_dir.as_deref(), Some(Path::new("/tmp/saves")));
        assert_eq!(config.sort.as_deref(), Some("level"));
        assert!(config.sort_descending);
    }

    #[test]
    fn empty_and_broken_lines_are_ignored_in_legacy_files() {
        let config = Config::parse("# comment\n\nsave_dir=\nbroken\nsave_dir=/tmp/saves\n");
        assert_eq!(config.save_dir.as_deref(), Some(Path::new("/tmp/saves")));
    }

    #[test]
    fn the_window_rectangle_round_trips() {
        let config = Config {
            window: Some(WindowGeometry {
                width: 1180,
                height: 760,
                x: Some(-120),
                y: Some(40),
            }),
            ..Config::default()
        };
        let text = config.to_text();
        assert!(text.contains("width = 1180"), "{text}");
        assert_eq!(Config::parse(&text), config);
        assert!(
            Config::parse("save_dir = \"/tmp/saves\"").window.is_none(),
            "a settings file without a window must not invent one"
        );
    }

    #[test]
    fn partial_toml_uses_defaults_for_missing_keys() {
        let config = Config::parse("sort = \"level\"\nsort_descending = true\n");
        assert_eq!(config.save_dir, None);
        assert_eq!(config.sort.as_deref(), Some("level"));
        assert!(config.sort_descending);
    }

    #[test]
    fn broken_toml_falls_back_to_defaults() {
        assert_eq!(Config::parse("sort = \"level\n"), Config::default());
    }

    #[test]
    fn missing_file_yields_defaults() {
        assert_eq!(Config::load_from(&None), Config::default());
        assert_eq!(
            Config::load_from(&Some(PathBuf::from("/definitely/not/here"))),
            Config::default()
        );
    }

    #[test]
    fn test_override_wins_over_the_environment() {
        let path = ensure_test_path();
        assert_eq!(Config::path(), Some(path));
        let config = Config {
            save_dir: Some(PathBuf::from("/games/Jade Empire/Save")),
            sort: Some("played".to_owned()),
            sort_descending: true,
            game_dir: None,
            window: None,
            theme: ThemeChoice::Dark,
            text_scale: 100,
            locale: Locale::English,
            remember_window: true,
        };
        config.store().expect("store settings");
        assert_eq!(Config::load(), config);
    }

    #[test]
    fn configuration_directory_follows_the_platform() {
        let home = Path::new("/home/player");
        assert_eq!(
            directory(Some(Path::new("/cfg")), None, Some(home)),
            Some(PathBuf::from("/cfg"))
        );
        let expected = if cfg!(target_os = "windows") {
            None
        } else if cfg!(target_os = "macos") {
            Some(home.join("Library/Application Support"))
        } else {
            Some(home.join(".config"))
        };
        assert_eq!(directory(None, None, Some(home)), expected);
    }
    #[test]
    fn legacy_values_may_be_quoted() {
        let config = Config::parse(
            "save_dir=\"/tmp/saves\"\n# mixed file forces the legacy parser\nsort=played\n",
        );
        assert_eq!(config.save_dir.as_deref(), Some(Path::new("/tmp/saves")));
        assert_eq!(config.sort.as_deref(), Some("played"));
    }

    #[test]
    fn storing_settings_leaves_no_temporary_file() {
        ensure_test_path();
        let path = Config::path().expect("test path");
        if let Some(parent) = path.parent() {
            let _ = std::fs::remove_dir_all(parent);
        }
        let config = Config {
            save_dir: Some(PathBuf::from("/games/Jade Empire/Save")),
            sort: Some("name".to_owned()),
            sort_descending: false,
            game_dir: None,
            window: None,
            theme: ThemeChoice::Dark,
            text_scale: 100,
            locale: Locale::English,
            remember_window: true,
        };
        config.store().expect("store");
        let parent = path.parent().expect("parent");
        let temporaries: Vec<_> = std::fs::read_dir(parent)
            .expect("read config dir")
            .flatten()
            .filter(|entry| entry.file_name().to_string_lossy().contains(".tmp-"))
            .collect();
        assert!(temporaries.is_empty());
        assert_eq!(Config::load(), config);
    }

    #[cfg(unix)]
    #[test]
    fn an_unrepresentable_path_does_not_erase_the_settings_file() {
        use std::ffi::OsString;
        use std::os::unix::ffi::OsStringExt as _;

        ensure_test_path();
        let path = Config::path().expect("test path");
        let good = Config {
            save_dir: Some(PathBuf::from("/tmp/saves")),
            sort: None,
            sort_descending: false,
            game_dir: None,
            window: None,
            theme: ThemeChoice::Dark,
            text_scale: 100,
            locale: Locale::English,
            remember_window: true,
        };
        good.store().expect("store good settings");
        let before = std::fs::read(&path).expect("read config");

        let bad = Config {
            save_dir: Some(PathBuf::from(OsString::from_vec(vec![0xff, b'x']))),
            sort: None,
            sort_descending: false,
            game_dir: None,
            window: None,
            theme: ThemeChoice::Dark,
            text_scale: 100,
            locale: Locale::English,
            remember_window: true,
        };
        assert!(bad.store().is_err());
        assert_eq!(std::fs::read(&path).expect("read config"), before);
    }
}
