//! Translation hook for the status messages the session builds.
//!
//! The session stays independent of the GUI and of any windowing toolkit. The
//! desktop crate installs `crate::i18n::t` once at startup, so every message
//! built on the UI thread follows the selected interface language. Without an
//! installed translator the source strings are returned unchanged, which keeps
//! unit tests and non-UI callers on English.

use std::sync::OnceLock;

/// Translates one catalogued source string.
pub type Translator = fn(&'static str) -> &'static str;

static TRANSLATOR: OnceLock<Translator> = OnceLock::new();

/// Installs the translator used by [`tr`] and [`fill`].
///
/// The first installation wins; later calls are ignored so a test or a second
/// application entry point cannot replace the translator while a window is
/// running.
pub fn install_translator(translator: Translator) {
    let _ = TRANSLATOR.set(translator);
}

/// Translates a static message template, falling back to the source text.
#[must_use]
pub fn tr(source: &'static str) -> &'static str {
    TRANSLATOR
        .get()
        .map_or(source, |translator| translator(source))
}

/// Uppercases the first character of a translated label.
///
/// The catalog keeps shared labels in lower case so a count noun and a card
/// caption can use the same key. A view that needs a sentence-case caption
/// applies this helper instead of storing a second copy of the word.
#[must_use]
pub fn capitalize(text: &str) -> String {
    let mut characters = text.chars();
    match characters.next() {
        Some(first) => first.to_uppercase().chain(characters).collect(),
        None => String::new(),
    }
}

/// Fills `{name}` placeholders in a translated template.
///
/// Values are inserted verbatim, so a path, a character name or an error
/// detail is never translated.
#[must_use]
pub fn fill(template: &'static str, arguments: &[(&str, &str)]) -> String {
    let mut text = tr(template).to_owned();
    for (name, value) in arguments {
        text = text.replace(&format!("{{{name}}}"), value);
    }
    text
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn without_a_translator_the_source_string_is_returned() {
        assert_eq!(tr("No savegame is open."), "No savegame is open.");
        assert_eq!(
            fill("Opened {label}.", &[("label", "Auto 1")]),
            "Opened Auto 1."
        );
    }

    #[test]
    fn a_label_can_be_capitalized_without_changing_the_rest() {
        assert_eq!(capitalize("gems"), "Gems");
        assert_eq!(capitalize("Gesundheit"), "Gesundheit");
        assert_eq!(capitalize(""), "");
    }

    #[test]
    fn every_placeholder_is_replaced() {
        assert_eq!(
            fill(
                "Found {savegames} in {directory}.",
                &[("savegames", "2 savegames"), ("directory", "/saves")]
            ),
            "Found 2 savegames in /saves."
        );
    }
}
