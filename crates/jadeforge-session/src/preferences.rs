//! User interface preferences that are shared by the settings file and the GUI.
//!
//! Keeping the enums here lets `jadeforge-session` validate and persist them
//! without depending on Freya; the desktop crate maps them to its palette and
//! layout scales.

use serde::{Deserialize, Serialize};

/// Colour theme selected by the user.
///
/// [`ThemeChoice::System`] follows the operating system preference reported by
/// the windowing layer. There is deliberately no automatic high-contrast
/// variant: Freya 0.5 exposes light/dark through `PlatformWindow.preferred_theme`
/// but no forced-colours preference, so high contrast is a manual choice until
/// the upstream API exists.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ThemeChoice {
    /// Follow the operating system's light/dark setting.
    System,
    /// The original dark jade-on-charcoal theme.
    #[default]
    Dark,
    /// A light parchment theme.
    Light,
    /// Maximum contrast, black background and white/yellow accents.
    HighContrast,
}

impl ThemeChoice {
    /// Every choice, in the order the settings UI shows them.
    pub const ALL: [Self; 4] = [Self::System, Self::Dark, Self::Light, Self::HighContrast];

    /// Stable settings-file key.
    #[must_use]
    pub fn key(self) -> &'static str {
        match self {
            Self::System => "system",
            Self::Dark => "dark",
            Self::Light => "light",
            Self::HighContrast => "high-contrast",
        }
    }

    /// Parses [`ThemeChoice::key`], falling back to the default.
    #[must_use]
    pub fn from_key(key: &str) -> Self {
        Self::ALL
            .into_iter()
            .find(|choice| choice.key().eq_ignore_ascii_case(key.trim()))
            .unwrap_or_default()
    }

    /// Human readable label for the settings UI.
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::System => "System",
            Self::Dark => "Dark",
            Self::Light => "Light",
            Self::HighContrast => "High contrast",
        }
    }

    /// One sentence that explains what the choice does.
    #[must_use]
    pub fn description(self) -> &'static str {
        match self {
            Self::System => "Follow the operating system's light or dark preference",
            Self::Dark => "The original dark jade theme",
            Self::Light => "A light parchment theme for bright rooms",
            Self::HighContrast => "Black and white with a yellow focus ring",
        }
    }

    /// Resolves [`ThemeChoice::System`] against the OS preference.
    #[must_use]
    pub fn resolve(self, system_prefers_light: bool) -> Self {
        match self {
            Self::System if system_prefers_light => Self::Light,
            Self::System => Self::Dark,
            other => other,
        }
    }
}

/// Text scale choices offered by the settings UI.
///
/// The values are percentages. The GUI scales the type, spacing and fixed row
/// dimensions by this factor, so 200 % never clips a row or the status slot.
pub const TEXT_SCALES: [u16; 4] = [100, 125, 150, 200];

/// Smallest accepted text scale in percent.
pub const MIN_TEXT_SCALE: u16 = 100;
/// Largest accepted text scale in percent.
pub const MAX_TEXT_SCALE: u16 = 200;

/// Clamps a stored scale to the range the layout is tested at.
///
/// Older or hand-edited settings files may hold any number; an unsupported
/// value falls back to 100 % instead of producing an untested layout.
#[must_use]
pub fn normalize_text_scale(percent: u16) -> u16 {
    if (MIN_TEXT_SCALE..=MAX_TEXT_SCALE).contains(&percent) {
        percent
    } else {
        100
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_theme_choice_round_trips_through_its_key() {
        for choice in ThemeChoice::ALL {
            assert_eq!(ThemeChoice::from_key(choice.key()), choice);
        }
        assert_eq!(ThemeChoice::from_key("LIGHT"), ThemeChoice::Light);
        assert_eq!(ThemeChoice::from_key("not-a-theme"), ThemeChoice::Dark);
    }

    #[test]
    fn system_follows_the_platform_preference() {
        assert_eq!(ThemeChoice::System.resolve(true), ThemeChoice::Light);
        assert_eq!(ThemeChoice::System.resolve(false), ThemeChoice::Dark);
        assert_eq!(
            ThemeChoice::HighContrast.resolve(true),
            ThemeChoice::HighContrast
        );
    }

    #[test]
    fn text_scale_stays_inside_the_tested_range() {
        for percent in TEXT_SCALES {
            assert_eq!(normalize_text_scale(percent), percent);
        }
        assert_eq!(normalize_text_scale(0), 100);
        assert_eq!(normalize_text_scale(99), 100);
        assert_eq!(normalize_text_scale(201), 100);
        assert_eq!(normalize_text_scale(u16::MAX), 100);
    }
}
