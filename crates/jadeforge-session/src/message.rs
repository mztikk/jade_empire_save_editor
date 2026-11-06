//! Translation helpers for the status messages the session builds.
//!
//! The session stays independent of the GUI. It calls the shared catalog crate
//! directly, so the active locale is the same one the views installed for this
//! render pass. A non-UI caller gets the English catalog.

/// Translates a stable message id.
#[must_use]
pub fn tr(id: &'static str) -> &'static str {
    jadeforge_i18n::t(id)
}

/// Fills `{name}` placeholders in a translated message template.
///
/// Values are inserted verbatim, so a path, a character name or an error
/// detail is never translated.
#[must_use]
pub fn fill(id: &'static str, arguments: &[(&str, &str)]) -> String {
    jadeforge_i18n::fill(id, arguments)
}

/// Uppercases the first character of a translated label.
///
/// The catalog keeps shared labels in lower case so a count noun and a card
/// caption can use the same id. A view that needs a sentence-case caption
/// applies this helper instead of storing a second copy of the word.
#[must_use]
pub fn capitalize(text: &str) -> String {
    let mut characters = text.chars();
    match characters.next() {
        Some(first) => first.to_uppercase().chain(characters).collect(),
        None => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_english_catalog_is_the_default() {
        assert_eq!(tr("status.no.savegame.is.open"), "No savegame is open.");
        assert_eq!(
            fill("status.opened", &[("label", "Auto 1")]),
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
                "status.found.in",
                &[("savegames", "2 savegames"), ("directory", "/saves")]
            ),
            "Found 2 savegames in /saves."
        );
    }
}
