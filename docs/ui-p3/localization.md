# UI localization

## Current state

The application ships an English source catalog, a bundled German catalog and
a QA pseudolocale. Every user-facing GUI string and every session status
template is catalogued; the German values that existed before this pass are preserved, and 380 new keys are waiting for translation with an empty value.
An empty German value falls back to English, so the interface stays usable
while the catalog is filled in.

## Layers

### GUI catalog (`jadeforge-gui::i18n`)

- `Locale` lives in `jadeforge-session` and has English, German and the QA
  pseudolocale. It is persisted as `locale = "en"`, `"de"` or `"en-xa"`.
  `from_tag` accepts `de`, `de-DE`, `de_AT` and the usual `en-xa`/`qps` QA
  tags; unsupported languages fall back to English rather than showing wrong
  strings.
- The active locale is thread local, exactly like the theme. The root installs
  it before rendering, and the headless suite can render several languages
  without leaking one test's choice into another.
- `t("English source string")` translates a static string.
- `fill("Opened {label}.", &[("label", &label)])` translates a template and
  substitutes named placeholders. The template is catalogued; the values are
  inserted verbatim, so a character name, an id or a path is never translated.
- `plural(count, "gem", "gems")` translates the noun pair and formats
  `"<count> <noun>"`. English and German both use the one/other distinction;
  a language with more plural forms needs a richer helper before it can be
  added.
- German lives in `crates/jadeforge-gui/locales/de.toml`, is embedded with
  `include_str!` and parsed once into a `HashMap`. The TOML keys are the exact
  English source strings; the values are grouped by screen with comments.
  Product and technical names stay unchanged, and game terms should follow the
  German game release (Kampfstil, Edelstein, Amulett, Gefährte, Fokus, Chi).

### Session message hook (`jadeforge-session::message`)

The session builds status messages without depending on Freya:

- `Translator` is a function pointer from a static source string to a static
  translated string.
- `install_translator(translator)` stores it in a `OnceLock`; the first
  installation wins.
- `tr(source)` returns the translated template or the source.
- `fill(template, arguments)` is the session-side equivalent of the GUI helper.

`app_with_directory` installs `crate::i18n::t` once through `use_hook`. The
session actions run on the UI thread, so the thread-local locale is correct.
A session unit test or a non-UI caller never installs a translator and
therefore keeps the English source strings.

The same hook translates the labels the session owns rather than the views:
sort labels, item categories, style kinds, alignment names, section names,
theme choices and `SaveEntry` summaries. Character field captions and ranges
are translated by the widget that renders them, and a test asserts that every
caption and range is catalogued.

## Catalog bookkeeping

- A test walks every `.rs` file under `crates/jadeforge-gui/src`, extracts
  every `t("...")`, `fill("...")` and `plural("...", "...")` literal (including
  multi-line Rust string continuations) and fails if the source catalog does
  not contain it.
- A second scan covers `tr("...")` and `fill("...")` literals under
  `crates/jadeforge-session/src`.
- A third test asserts that every character field caption and range and every
  filter placeholder is catalogued; those values are passed as variables, not
  as literals at the call site.
- The German coverage test asserts that every catalogued key exists exactly
  once in `de.toml` and that no stale German key remains.
- The opt-in `every_german_value_is_filled` test lists the untranslated values:
  `cargo test -p jadeforge-gui --lib every_german_value_is_filled -- --ignored`.
- A headless test renders the real shell with `locale = "de"` and asserts the
  existing German About button and empty state. It deliberately only asserts
  values that are already translated.

## Adding a language

1. Add a `Locale` variant and update `code`, `label`, `ALL`, `from_tag` and the
   serde rename in `crates/jadeforge-session/src/locale.rs`.
2. Add `crates/jadeforge-gui/locales/<code>.toml` with the same keys as the
   English catalog.
3. Parse that file in `jadeforge-gui::i18n` and add its lookup to `t`. The
   German catalog is the template: a `OnceLock<HashMap<String, String>>`, an
   empty-value fallback and a coverage test.
4. Extend the GUI and session source scans if the new language uses a
   different template mechanism.
5. If the language has more than the one/other plural distinction, replace
   `plural` with a plural-rules helper first.

No view code changes are needed for a language with the same placeholders.

## Deliberately untranslated

- Dynamic values: character names, savegame folder labels, file paths,
  resource ids, game data names and descriptions.
- Technical error details appended to a translated wrapper, for example the
  `{error}` part of `Cannot save: {error}`. The wrapper sentence is
  catalogued; the underlying reader error stays verbatim.
- Product and platform names: JadeForge, Jade Empire, TOML, Wayland, KDE,
  save.sac, dialog.tlk and the shortcut keys.
- Language names in the language selector are endonyms (English, Deutsch);
  the QA label `English (pseudolocale)` is a test-only name.
- Pure format fragments such as `{} / {}`, `#{}` and `ItemList[...].Count`
  stay as they are.

## Upstream references

- TOML 1.0 (the catalog format and its string escaping):
  <https://toml.io/en/v1.0.0>
- Locale tag matching follows the POSIX locale variables:
  <https://pubs.opengroup.org/onlinepubs/9699919799/basedefs/V1_chap08.html>
- Fluent, evaluated for a future plural/gender template layer:
  <https://projectfluent.org/>

## Test plan

- Session: locale tags round trip (`en`, `de`, `de_DE.UTF-8`, `de-AT`,
  `en-xa`, `qps`), `den` is not German, and unsupported languages resolve to
  English.
- Session messages: without an installed translator, `tr` and `fill` return the
  English source; existing session message assertions stay green.
- GUI: the German TOML parses; it covers every catalogued source string and has
  no stale keys; the opt-in completeness test reports empty values; unknown
  strings fall back to the source in every locale; `fill` substitutes every
  named placeholder.
- Headless UI: launch with `locale = "en-xa"` and assert a catalogued action is
  pseudolocalized; open About, click the pseudolocale chip and then the German
  chip, assert the active locale and the persisted `locale` value; launch with
  `locale = "de"` and assert the translated About button and empty state.
- The snapshot example keeps English as the default; a pseudolocale screenshot
  is a QA image, not tracked documentation.

## Accessibility, contrast and reduced motion

- The language chips are radio buttons with selected state and a spoken
  description; switching is instant.
- Pseudolocale and German text keep the same colours and contrast as English;
  the width tests already cover the longer pseudolocale.
- No animation.

## Risk

- A string added directly in a variable or a `format!` result is not caught by
  the source scan until it is converted to a `t(...)` or `fill(...)` call.
  Keep new user-facing text on one of those helpers.
- A malformed `de.toml` degrades to English at runtime; the coverage test fails
  the build instead of letting it ship.
- Placeholder names are a contract between the English template and every
  translation. A translation that renames or drops a placeholder shows the
  literal `{name}`; keep the source template and the translation in sync.
