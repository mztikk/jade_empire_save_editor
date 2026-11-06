# UI localization

## Current state

Every interface string was a literal in a Freya view. The game data
(`dialog.tlk`) is localized, but the application itself was English-only and
had no way to select another language.

## Approach

- `jadeforge-session::locale::Locale` is persisted as `locale = "en"`. The
  default is English. `Locale::from_environment` implements the usual
  `LC_ALL` > `LC_MESSAGES` > `LANG` precedence and maps an unsupported language
  to English rather than displaying wrong strings. The QA pseudolocale is
  detected as `en-xa` (or `qps`).
- `jadeforge-gui::i18n` installs the active locale in a thread-local, exactly
  like the theme, and exposes `t("English source string")`. The catalog maps
  each source string to its translation. English returns the source; the
  pseudolocale returns a bracketed, accented variant that makes clipping and
  untranslated constants visible.
- The About page has a LANGUAGE group with the available locales. Changing it
  updates the thread-local for the next render and writes the code to the TOML
  settings file.
- Static interface text is wrapped in `t(...)` at the call site. The module
  test walks every `.rs` file under `crates/jadeforge-gui/src`, extracts every
  `t("...")` literal (including multi-line Rust string continuations) and
  fails if the catalog does not contain it. A second test asserts every
  catalog entry has a distinct pseudolocale value. That is the missing-key
  test: adding a new wrapped string without a catalog row fails the suite.
- No translated locale ships yet. Inventing unreviewed translations would be
  worse than the English fallback, and the issue explicitly forbids it. A
  reviewed language is added by adding its catalog rows (one row per source
  string), selecting it from `Locale::ALL`, and adding its language tag to
  `Locale::from_tag`; the source-scan test then proves completeness.

## Scope note

The catalog covers the application chrome and the static text in the views
(headings, hints, buttons, labels, filter placeholders, status actions,
settings). Strings assembled dynamically in the session (for example
`Saved <path>.` or `Cannot open <name>: <error>`) still use English templates:
localizing them needs a reviewed template catalog and argument ordering per
language. They remain on English fallback and are the follow-up work for a
translated release. The pseudolocale test shows the boundary: catalogued
chrome expands, the English fallback stays readable.

## Upstream references

- Fluent is the format Freya's optional i18n feature builds on, but JadeForge
  uses a dependency-free catalog so a missing key is a test failure rather than
  a runtime panic:
  <https://projectfluent.org/>
- Locale tag matching follows the POSIX locale variables:
  <https://pubs.opengroup.org/onlinepubs/9699919799/basedefs/V1_chap08.html>

## Test plan

- Session: locale tags round trip (`en`, `en-xa`, `qps`, `de_DE.UTF-8`), and
  unsupported languages resolve to English.
- GUI: the source scan plus catalog completeness tests above; unknown strings
  fall back to the source.
- Headless UI: launch with `locale = "en-xa"` and assert a catalogued action is
  pseudolocalized; open About, click the pseudolocale chip, assert the active
  locale and the persisted `locale` value.
- The snapshot example keeps English as the default; a pseudolocale screenshot
  is a QA image, not tracked documentation.

## Accessibility, contrast and reduced motion

- The language chips are radio buttons with selected state and a spoken
  description; switching is instant.
- Pseudolocale text keeps the same colours and contrast as English.
- No animation.

## Risk

- A string added in a variable or a `format!` result is not caught by the
  source scan until it is converted to a `t(...)` call. The scope note records
  that limitation instead of pretending the UI is fully translated.
