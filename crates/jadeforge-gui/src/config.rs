//! The small settings file `JadeForge` keeps between runs.
//!
//! Only the save folder is stored. The file lives in the platform's user
//! configuration directory (`XDG_CONFIG_HOME`, `%APPDATA%` or
//! `~/Library/Application Support`) and is written on a best effort basis: a
//! failed write never stops the editor from working.

use std::path::{Path, PathBuf};

/// Settings persisted between runs.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Config {
    /// Folder that was scanned last.
    pub save_dir: Option<PathBuf>,
}

impl Config {
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

    /// Parses the `key=value` format used by the settings file.
    #[must_use]
    pub fn parse(text: &str) -> Self {
        let mut config = Self::default();
        for line in text.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let Some((key, value)) = line.split_once('=') else {
                continue;
            };
            if key.trim() == "save_dir" {
                let value = value.trim();
                config.save_dir = (!value.is_empty()).then(|| PathBuf::from(value));
            }
        }
        config
    }

    /// Renders the settings file.
    #[must_use]
    pub fn to_text(&self) -> String {
        use std::fmt::Write as _;

        let mut text = String::from("# JadeForge settings\n");
        if let Some(dir) = &self.save_dir {
            let _ = writeln!(text, "save_dir={}", dir.display());
        }
        text
    }

    /// Writes the settings, creating the configuration directory if needed.
    ///
    /// # Errors
    ///
    /// Returns the underlying I/O error when the directory or file cannot be
    /// written.
    pub fn store(&self) -> std::io::Result<()> {
        let Some(path) = Self::path() else {
            return Err(std::io::Error::other("no configuration directory"));
        };
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(path, self.to_text())
    }

    /// Location of the settings file.
    #[must_use]
    pub fn path() -> Option<PathBuf> {
        let xdg = std::env::var_os("XDG_CONFIG_HOME")
            .filter(|value| !value.is_empty())
            .map(PathBuf::from);
        let appdata = std::env::var_os("APPDATA").map(PathBuf::from);
        let home = std::env::var_os("HOME").map(PathBuf::from);
        let base = directory(xdg.as_deref(), appdata.as_deref(), home.as_deref())?;
        Some(base.join("jadeforge").join("config"))
    }
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
        };
        let text = config.to_text();
        assert!(text.contains("save_dir=/games/Jade Empire/Save"));
        assert_eq!(Config::parse(&text), config);
    }

    #[test]
    fn empty_and_broken_lines_are_ignored() {
        let config = Config::parse("# comment\n\nsave_dir=\nbroken\nsave_dir=/tmp/saves\n");
        assert_eq!(config.save_dir.as_deref(), Some(Path::new("/tmp/saves")));
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
}
