//! Application language selection.
//!
//! The game data is already localized; the application itself ships an English
//! source catalog plus a QA pseudolocale that expands every string so layout
//! regressions are visible. A real translation is added as another catalog;
//! until a reviewed translation exists, every language falls back to English.
//! That is deliberate: inventing unreviewed translations would be worse than
//! falling back.

use serde::{Deserialize, Serialize};

/// Language bundle the interface uses.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Locale {
    /// English source strings.
    #[default]
    #[serde(rename = "en")]
    English,
    /// QA pseudolocale: every catalogued string is bracketed and accented to
    /// expose clipping and hardcoded text.
    #[serde(rename = "en-xa")]
    Pseudo,
}

impl Locale {
    /// Every selectable locale, in the order the settings UI shows them.
    pub const ALL: [Self; 2] = [Self::English, Self::Pseudo];

    /// Stable settings-file code.
    #[must_use]
    pub fn code(self) -> &'static str {
        match self {
            Self::English => "en",
            Self::Pseudo => "en-xa",
        }
    }

    /// Human readable label for the settings UI.
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::English => "English",
            Self::Pseudo => "English (pseudolocale)",
        }
    }

    /// Parses a language tag such as `de-DE`, `en_XA` or `en`.
    ///
    /// Only catalogued languages are selected; anything else falls back to
    /// English rather than showing wrong strings.
    #[must_use]
    pub fn from_tag(tag: &str) -> Self {
        let normalized = tag
            .split(['.', '@'])
            .next()
            .unwrap_or(tag)
            .replace('_', "-")
            .to_lowercase();
        if normalized == "en-xa" || normalized == "qps" || normalized.starts_with("en-xa-") {
            Self::Pseudo
        } else {
            Self::English
        }
    }

    /// Selects the locale from the environment, with the usual precedence
    /// `LC_ALL`, `LC_MESSAGES`, `LANG`.
    ///
    /// An unset or unsupported language is English, which is also the fallback
    /// for every untranslated key.
    #[must_use]
    pub fn from_environment() -> Self {
        for key in ["LC_ALL", "LC_MESSAGES", "LANG"] {
            let Ok(value) = std::env::var(key) else {
                continue;
            };
            if value.is_empty() || value == "C" || value == "POSIX" {
                continue;
            }
            return Self::from_tag(&value);
        }
        Self::English
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn locale_codes_round_trip() {
        for locale in Locale::ALL {
            assert_eq!(Locale::from_tag(locale.code()), locale);
        }
        assert_eq!(Locale::from_tag("en_US.UTF-8"), Locale::English);
        assert_eq!(Locale::from_tag("en-xa"), Locale::Pseudo);
        assert_eq!(Locale::from_tag("qps"), Locale::Pseudo);
        assert_eq!(Locale::from_tag("de_DE.UTF-8"), Locale::English);
        assert_eq!(Locale::from_tag(""), Locale::English);
    }
}
