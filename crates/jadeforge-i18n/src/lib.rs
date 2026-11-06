//! Interface catalogs and locale selection for `JadeForge`.
//!
//! The catalogs are keyed by stable semantic ids, never by English source
//! text. English is the source-of-truth catalog; German is a peer catalog with
//! the same ids. A missing or empty value renders as a bracketed id marker, so
//! an incomplete catalog is visible instead of silently falling back to
//! English.
//!
//! The active locale is thread local, exactly like the theme. Views install it
//! before rendering; tests can render several languages without leaking a
//! choice between threads.

use std::cell::Cell;
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

use serde::Deserialize;

/// A catalogued interface id.
///
/// The generated [`keys`] module is the only source of values, so a call site
/// cannot pass an arbitrary string to [`t`], [`fill`] or [`plural`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Key(&'static str);

impl Key {
    /// Wraps a catalogued id. The generated [`keys`] module uses this.
    pub(crate) const fn new(id: &'static str) -> Self {
        Self(id)
    }

    /// The stable catalog id.
    #[must_use]
    pub fn id(self) -> &'static str {
        self.0
    }
}

pub mod keys {
    //! Typed constants for every id in `locales/en.toml`.

    include!(concat!(env!("OUT_DIR"), "/keys.rs"));
}

/// Language bundle the interface uses.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Locale {
    /// English source strings.
    #[default]
    #[serde(rename = "en")]
    English,
    /// German interface catalog.
    #[serde(rename = "de")]
    German,
    /// QA pseudolocale generated from the English catalog.
    #[serde(rename = "en-xa")]
    Pseudo,
}

impl Locale {
    /// Every selectable locale, in the order the settings UI shows them.
    pub const ALL: [Self; 3] = [Self::English, Self::German, Self::Pseudo];

    /// Stable settings-file code.
    #[must_use]
    pub fn code(self) -> &'static str {
        match self {
            Self::English => "en",
            Self::German => "de",
            Self::Pseudo => "en-xa",
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
        } else if normalized == "de" || normalized.starts_with("de-") {
            Self::German
        } else {
            Self::English
        }
    }

    /// Selects the locale from the environment, with the usual precedence
    /// `LC_ALL`, `LC_MESSAGES`, `LANG`.
    ///
    /// An unset, unsupported or incomplete language is English. German stays
    /// preview-only until its catalog is marked complete.
    #[must_use]
    pub fn from_environment() -> Self {
        for key in ["LC_ALL", "LC_MESSAGES", "LANG"] {
            let Ok(value) = std::env::var(key) else {
                continue;
            };
            if value.is_empty() || value == "C" || value == "POSIX" {
                continue;
            }
            return environment_tag(&value);
        }
        Self::English
    }
}

fn environment_tag(tag: &str) -> Locale {
    let locale = Locale::from_tag(tag);
    if locale == Locale::German && !is_complete(Locale::German) {
        Locale::English
    } else {
        locale
    }
}

thread_local! {
    static CURRENT: Cell<Locale> = const { Cell::new(Locale::English) };
}

/// Installs the locale for the following render pass.
pub fn install(locale: Locale) {
    CURRENT.with(|current| current.set(locale));
}

/// The active locale.
#[must_use]
pub fn current() -> Locale {
    CURRENT.with(Cell::get)
}

/// Human readable label for the language selector.
///
/// German is shown as a preview while its catalog is incomplete.
#[must_use]
pub fn locale_label(locale: Locale) -> &'static str {
    match locale {
        Locale::English => "English",
        Locale::German if is_complete(Locale::German) => "Deutsch",
        Locale::German => "Deutsch (Vorschau)",
        Locale::Pseudo => "English (pseudolocale)",
    }
}

/// Whether a catalog is marked complete and can be selected from the
/// environment.
#[must_use]
pub fn is_complete(locale: Locale) -> bool {
    match locale {
        Locale::English | Locale::Pseudo => english().complete,
        Locale::German => german().complete,
    }
}

/// Whether the English source catalog contains `id`.
#[must_use]
pub fn has_id(id: &str) -> bool {
    english().messages.contains_key(id)
}

/// Translates a catalogued interface key.
///
/// The [`Key`] type makes an arbitrary string unrepresentable:
///
/// ```compile_fail,E0308
/// let _ = jadeforge_i18n::t("shell.about");
/// ```
///
/// A missing or empty value returns a cached `⟦id⟧` marker. There is no
/// English fallback: an incomplete translation must be visible.
#[must_use]
pub fn t(key: Key) -> &'static str {
    let id = key.id();
    match current() {
        Locale::English => english()
            .messages
            .get(id)
            .filter(|value| !value.is_empty())
            .map_or_else(|| marker(id), String::as_str),
        Locale::German => german()
            .messages
            .get(id)
            .filter(|value| !value.is_empty())
            .map_or_else(|| marker(id), String::as_str),
        Locale::Pseudo => pseudo().get(id).map_or_else(|| marker(id), String::as_str),
    }
}

/// Fills `{name}` placeholders in a translated template.
///
/// The key is catalogued and typed, so a raw source string cannot be passed:
///
/// ```compile_fail,E0308
/// let _ = jadeforge_i18n::fill("status.opened", &[]);
/// ```
///
/// Values are inserted verbatim, so a path, a character name or an error
/// detail is never translated.
#[must_use]
pub fn fill(key: Key, arguments: &[(&str, &str)]) -> String {
    let mut text = t(key).to_owned();
    for (name, value) in arguments {
        text = text.replace(&format!("{{{name}}}"), value);
    }
    text
}

/// A count followed by the singular or plural catalogued noun.
///
/// Both nouns are typed keys:
///
/// ```compile_fail,E0308
/// let _ = jadeforge_i18n::plural(1, "count.item.one", "count.item.other");
/// ```
#[must_use]
pub fn plural(count: usize, singular: Key, plural: Key) -> String {
    let noun = if count == 1 { t(singular) } else { t(plural) };
    format!("{count} {noun}")
}

/// A cached `⟦id⟧` marker for a missing or empty value.
fn marker(id: &'static str) -> &'static str {
    static MARKERS: OnceLock<Mutex<HashMap<&'static str, &'static str>>> = OnceLock::new();
    let markers = MARKERS.get_or_init(|| Mutex::new(HashMap::new()));
    if let Ok(mut markers) = markers.lock() {
        if let Some(marker) = markers.get(id) {
            return marker;
        }
        let marker: &'static str = Box::leak(format!("⟦{id}⟧").into_boxed_str());
        markers.insert(id, marker);
        return marker;
    }
    Box::leak(format!("⟦{id}⟧").into_boxed_str())
}

/// The English catalog and its completeness flag.
fn english() -> &'static Catalog {
    static CATALOG: OnceLock<Catalog> = OnceLock::new();
    CATALOG.get_or_init(|| load(include_str!("../locales/en.toml")))
}

/// The German catalog and its completeness flag.
fn german() -> &'static Catalog {
    static CATALOG: OnceLock<Catalog> = OnceLock::new();
    CATALOG.get_or_init(|| load(include_str!("../locales/de.toml")))
}

/// QA pseudolocale generated from the English values.
fn pseudo() -> &'static HashMap<String, String> {
    static CATALOG: OnceLock<HashMap<String, String>> = OnceLock::new();
    CATALOG.get_or_init(|| {
        english()
            .messages
            .iter()
            .map(|(id, value)| (id.clone(), pseudo_text(value)))
            .collect()
    })
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Catalog {
    complete: bool,
    messages: HashMap<String, String>,
}

/// Parses a catalog, degrading to an empty one instead of English.
fn load(source: &str) -> Catalog {
    toml::from_str::<Catalog>(source).unwrap_or_else(|_| Catalog {
        complete: false,
        messages: HashMap::new(),
    })
}

/// Accents and brackets one English value for the QA pseudolocale.
fn pseudo_text(value: &str) -> String {
    let accented: String = value
        .chars()
        .map(|character| match character {
            'a' => 'á',
            'A' => 'Á',
            'e' => 'é',
            'E' => 'É',
            'i' => 'í',
            'I' => 'Í',
            'o' => 'ó',
            'O' => 'Ó',
            'u' => 'ú',
            'U' => 'Ú',
            'n' => 'ñ',
            'N' => 'Ñ',
            'c' => 'ç',
            'C' => 'Ç',
            other => other,
        })
        .collect();
    format!("⟦{accented}⟧")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn placeholders(value: &str) -> Vec<String> {
        let mut names: Vec<String> = value
            .split('{')
            .skip(1)
            .filter_map(|part| part.split_once('}').map(|(name, _)| name.to_owned()))
            .collect();
        names.sort_unstable();
        names
    }

    fn deutsch_is_empty(id: &str) -> bool {
        german().messages.get(id).is_some_and(String::is_empty)
    }

    #[test]
    fn both_catalogs_parse_and_have_the_same_ids() {
        let en = english();
        let de = german();
        assert!(!en.messages.is_empty(), "en.toml did not parse");
        assert!(!de.messages.is_empty(), "de.toml did not parse");
        let mut english_ids: Vec<&String> = en.messages.keys().collect();
        let mut german_ids: Vec<&String> = de.messages.keys().collect();
        english_ids.sort_unstable();
        german_ids.sort_unstable();
        assert_eq!(english_ids, german_ids, "the catalogs have different ids");
    }

    #[test]
    fn the_english_catalog_is_complete_and_non_empty() {
        let en = english();
        assert!(en.complete, "en.toml must be marked complete");
        let empty: Vec<&str> = en
            .messages
            .iter()
            .filter(|(_, value)| value.is_empty())
            .map(|(id, _)| id.as_str())
            .collect();
        assert!(empty.is_empty(), "empty English ids: {empty:?}");
        assert!(is_complete(Locale::English));
    }

    #[test]
    fn placeholder_sets_match_across_catalogs() {
        let en = english();
        let de = german();
        for (id, value) in &en.messages {
            let Some(german) = de.messages.get(id) else {
                continue;
            };
            if german.is_empty() {
                continue;
            }
            assert_eq!(
                placeholders(value),
                placeholders(german),
                "placeholder mismatch for {id}"
            );
        }
    }

    #[test]
    fn german_completeness_is_only_enforced_when_marked_complete() {
        let de = german();
        if de.complete {
            let empty: Vec<&str> = de
                .messages
                .iter()
                .filter(|(_, value)| value.is_empty())
                .map(|(id, _)| id.as_str())
                .collect();
            assert!(empty.is_empty(), "empty German ids: {empty:?}");
        } else {
            assert!(!is_complete(Locale::German));
            assert!(de.messages.values().any(String::is_empty));
        }
    }

    #[test]
    fn preview_returns_markers_instead_of_english() {
        install(Locale::German);
        assert_eq!(t(keys::SHELL_ABOUT), "Über JadeForge");
        assert_eq!(t(Key::new("unknown.id")), "⟦unknown.id⟧");
        if !is_complete(Locale::German) {
            let empty = keys::ALL
                .iter()
                .copied()
                .find(|key| deutsch_is_empty(key.id()));
            if let Some(key) = empty {
                assert_eq!(t(key), format!("⟦{}⟧", key.id()).as_str());
            }
        }
        install(Locale::English);
    }

    #[test]
    fn english_missing_ids_are_markers_too() {
        install(Locale::English);
        assert_eq!(t(Key::new("unknown.id")), "⟦unknown.id⟧");
    }

    #[test]
    fn a_malformed_catalog_degrades_to_markers() {
        let catalog = load("complete = true\nunknown = 1\n[messages]\n");
        assert!(catalog.messages.is_empty());
        assert!(!catalog.complete);
    }

    #[test]
    #[ignore = "run after filling locales/de.toml"]
    fn every_german_value_is_filled() {
        let mut empty: Vec<&str> = german()
            .messages
            .iter()
            .filter(|(_, value)| value.is_empty())
            .map(|(id, _)| id.as_str())
            .collect();
        empty.sort_unstable();
        assert!(
            empty.is_empty(),
            "{} untranslated German ids:\n{}",
            empty.len(),
            empty.join("\n")
        );
    }

    #[test]
    fn pseudo_is_generated_from_english() {
        install(Locale::Pseudo);
        let value = t(keys::SHELL_ABOUT);
        assert!(value.starts_with('⟦') && value.ends_with('⟧'));
        assert_ne!(value, "About");
        install(Locale::English);
    }

    #[test]
    fn tags_select_the_catalogued_locales() {
        assert_eq!(Locale::from_tag("de"), Locale::German);
        assert_eq!(Locale::from_tag("de-DE"), Locale::German);
        assert_eq!(Locale::from_tag("de_AT"), Locale::German);
        assert_eq!(Locale::from_tag("en"), Locale::English);
        assert_eq!(Locale::from_tag("en-xa"), Locale::Pseudo);
        assert_eq!(Locale::from_tag("qps"), Locale::Pseudo);
        assert_eq!(Locale::from_tag("den"), Locale::English);
        let from_german_tag = environment_tag("de_DE.UTF-8");
        if is_complete(Locale::German) {
            assert_eq!(from_german_tag, Locale::German);
        } else {
            assert_eq!(from_german_tag, Locale::English);
        }
    }

    #[test]
    fn the_german_label_tracks_the_catalog_completeness() {
        assert_eq!(locale_label(Locale::English), "English");
        if is_complete(Locale::German) {
            assert_eq!(locale_label(Locale::German), "Deutsch");
        } else {
            assert_eq!(locale_label(Locale::German), "Deutsch (Vorschau)");
        }
        assert_eq!(locale_label(Locale::Pseudo), "English (pseudolocale)");
    }

    #[test]
    fn generated_keys_match_the_english_catalog() {
        let catalog = load(include_str!("../locales/en.toml"));
        let mut catalogued: Vec<&str> = catalog.messages.keys().map(String::as_str).collect();
        catalogued.sort_unstable();
        let mut generated: Vec<&str> = keys::ALL.iter().map(|key| key.id()).collect();
        generated.sort_unstable();
        assert_eq!(generated, catalogued, "keys.rs and en.toml differ");
        let unique: std::collections::HashSet<&str> = generated.iter().copied().collect();
        assert_eq!(unique.len(), generated.len(), "duplicate generated keys");
        for key in keys::ALL {
            assert!(
                catalog.messages.contains_key(key.id()),
                "uncatalogued key {}",
                key.id()
            );
        }
    }

    #[test]
    fn known_ids_are_present_and_unknown_ids_are_not() {
        assert!(has_id("shell.about"));
        assert!(has_id("count.item.one"));
        assert!(!has_id("not.a.real.id"));
    }
}
