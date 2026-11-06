# UI localization

## Current state

Every interface string was a literal in a Freya view. The game data
(`dialog.tlk`) is localized, but the application itself was English-only and
had no way to select another language.

## Approach

- `jadeforge-session::locale::Locale` has the variants English, German and the
  QA pseudolocale, and is persisted as `locale = "en"`, `"de"` or `"en-xa"`.
  The default stays English so screenshots and first runs are deterministic.
  `from_tag` accepts `de`, `de-DE`, `de_AT` and the usual `en-xa`/`qps` QA
  tags; unsupported languages fall back to English rather than showing wrong
  strings.
- `jadeforge-gui::i18n` installs the active locale in a thread-local, exactly
  like the theme, and exposes `t("English source string")`.
- English is the source language. German lives in
  `crates/jadeforge-gui/locales/de.toml`, is embedded with `include_str!` and
  parsed once into a `HashMap`. Empty or missing values fall back to the
  English source, so the catalog can be filled in progressively without ever
  showing an invented string. The German locale is selectable in About while
  the translation is in progress.
- The TOML keys are the exact English source strings. The values are grouped by
  screen with comments, so a translator can work section by section. Product
  and technical names stay unchanged; game terms should follow the German game
  release (Kampfstil, Edelstein, Amulett, Gefährte, Fokus, Chi).
- Static interface text is wrapped in `t(...)` at the call site. A test walks
  every `.rs` file under `crates/jadeforge-gui/src`, extracts every `t("...")`
  literal (including multi-line Rust string continuations) and fails if the
  source catalog does not contain it. A second test asserts every catalogued
  key exists exactly once in `de.toml` and that no stale German keys remain.
  The opt-in `every_german_value_is_filled` test lists the untranslated values
  and is run after the catalog is filled.
- Adding a language: add a `Locale` variant (`code`, `label`, `ALL`,
  `from_tag`), add `locales/<code>.toml` with the same keys, add its parse
  entry to `i18n`, and extend the coverage test. No view code changes.

TOML was chosen over Fluent because the catalog is a flat source-string table
with no placeholders, and the existing `toml` dependency already parses the
settings file. A language needing plurals or gendered sentences will need a
template format; the current strings avoid both.

## Scope note

The catalog covers the application chrome and the static text in the views
(headings, hints, buttons, labels, filter placeholders, status actions,
settings). Strings assembled dynamically in the session (for example
`Saved <path>.` or `Cannot open <name>: <error>`) still use English templates:
localizing them needs a reviewed template catalog and argument ordering per
language. They remain on English fallback and are the remaining follow-up for a
fully German interface. The pseudolocale test shows the boundary: catalogued
chrome expands, the English fallback stays readable.

## Upstream references

- TOML 1.0 (the catalog format and its string escaping):
  <https://toml.io/en/v1.0.0>
- Locale tag matching follows the POSIX locale variables:
  <https://pubs.opengroup.org/onlinepubs/9699919799/basedefs/V1_chap08.html>
- Fluent, evaluated for the future template work:
  <https://projectfluent.org/>

## Test plan

- Session: locale tags round trip (`en`, `de`, `de_DE.UTF-8`, `de-AT`,
  `en-xa`, `qps`), `den` is not German, and unsupported languages resolve to
  English.
- GUI: the German TOML parses; it covers every catalogued source string and has
  no stale keys; the opt-in completeness test reports empty values; unknown
  strings fall back to the source in every locale.
- Headless UI: launch with `locale = "en-xa"` and assert a catalogued action is
  pseudolocalized; open About, click the pseudolocale chip and then the German
  chip, assert the active locale and the persisted `locale` value. Once the
  German values are filled, assert a known German label is rendered.
- The snapshot example keeps English as the default; a pseudolocale screenshot
  is a QA image, not tracked documentation.

## Accessibility, contrast and reduced motion

- The language chips are radio buttons with selected state and a spoken
  description; switching is instant.
- Pseudolocale and German text keep the same colours and contrast as English;
  the width tests already cover the longer pseudolocale.
- No animation.

## Risk

- A string added in a variable or a `format!` result is not caught by the
  source scan until it is converted to a `t(...)` call. The scope note records
  that limitation instead of pretending the UI is fully translated.
- A malformed `de.toml` degrades to English at runtime; the coverage test fails
  the build instead of letting it ship.
