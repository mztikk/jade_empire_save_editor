# UI localization

## Current state

The interface is keyed by stable semantic ids. English is the source-of-truth
catalog and is complete; the German peer catalog is complete too (472 ids,
`complete = true`) and is offered as **Deutsch**. A filled id renders the active
locale; a missing or empty id renders a bracketed id marker and never falls
back to English. The preview mechanism remains for a future incomplete catalog.

The catalog no longer uses English source strings as keys, so editing an
English label cannot orphan a translation. The German text is preserved,
including the merged labels that were prefilled from existing translations. The
final consolidation renamed sentence-slug ids and merged repeated wrappers; the
last reuse pass removed the two combined stat labels and composes them from the
six shared `field.*` ids plus the catalogued `common.list_separator`.

## Crate layout

`crates/jadeforge-i18n` owns the locale, catalogs and strictness:

```
crates/jadeforge-i18n/
  Cargo.toml
  build.rs          generates the typed keys module from en.toml
  locales/en.toml   complete = true
  locales/de.toml   complete = true
  src/lib.rs        Key, Locale, catalogs, t/fill/plural, markers, pseudo
```

It depends only on `serde` and `toml` at runtime, plus `toml` as a build
dependency, with no Freya and no session types. `jadeforge-session` re-exports
`Locale`, `Key` and the catalog API for the settings file and session
messages; `jadeforge-gui::i18n` re-exports the same API so the views keep using
`crate::i18n::*`.

## Format

```toml
complete = false

[messages]
"shell.save" = "Save to disk"
```

Both files use `complete` plus a `messages` table. Parsing uses
`deny_unknown_fields`. A malformed catalog degrades to an empty catalog, so
every id becomes a marker; it never falls back to English.

`en.toml` is the source of truth and must be `complete = true`. `de.toml` is
complete as well; a new language starts with `complete = false` until every id
has a non-empty value. Key parity between the catalogs and placeholder parity
for every non-empty value are enforced by tests.

Id comments:

- `# args: name, count` lists the placeholders of a template.
- `# context: ...` explains an id that is used in more than one place.
- Section comments name the screen or subsystem that owns the ids.

## Ids

Ids are stable, lowercase, dot-separated and match `^[a-z0-9_.]+$`. They are
semantic, not numeric and not English sentences:

- `shell.about`, `shell.save`
- `shell.loading_savegames`, `save_browser.sort_by`
- `overview.time_played`, `character.level`
- `field.name.range`, `field.range`, `field.max`
- `count.item.one`, `count.item.other`
- `dialog.unsaved_changes.explanation`
- `about.diagnostics.version`
- `status.opened`, `status.nothing_to_undo`
- `status.cannot_add`, `status.added_label`
- `error.section_unreadable`, `error.clipboard_unavailable`
- `save_browser.no_match`, `shell.empty.pick_save`
- `count.weapon.other`, `theme.system`

Long paragraphs use a short semantic id such as `about.backup.explanation` or
`about.license.summary`. The full English text lives in the value, not in the
key.

Sentence-slug ids were a transitional artifact of the source-string migration.
The final consolidation renamed them to `area.subject.action` names and merged
repeated wrappers into one template with a placeholder: the five section-read
wrappers share `error.section_unreadable`, the add/remove/change failures share
`status.cannot_add`/`status.cannot_remove`/`status.cannot_change`, and the
pending labels share `status.added_label`/`status.removed_label_template`.
A new id that repeats an English sentence is a bug, not a naming choice.

## Id reuse policy

- Reuse an existing id when the meaning, grammatical role and context are
  identical; do not add a per-screen copy of the same message.
- Do not add an id for a case or punctuation variant. Store the display case
  in the catalog, or apply the case transform at the call site.
- Compose invariant noun lists from their individual ids and a catalogued
  separator (`join_list`, `common.list_separator`) instead of adding a combined
  id. The Overview stat lines reuse `field.body`/`field.mind`/`field.spirit` and
  `field.charm`/`field.intuition`/`field.intimidate`.
- Do not compose sentences or fragments when word order, case, gender or
  grammar can differ; keep a whole-message id and pass named arguments.
- Add a new id only when no existing id expresses the message correctly; add a
  `# context:` comment explaining why it is distinct.
- `english_values_are_unique` fails if two ids have the same English value, and
  the key/placeholder/parity tests run on every change.

## Runtime

- `install(locale)` stores the active locale in a thread-local; `current()`
  reads it. Every render pass installs the configured locale.
- `t(keys::SHELL_ABOUT)` looks up the active catalog only. A missing or empty
  value returns a cached `⟦id⟧` marker. English missing ids are markers too.
- `fill(keys::STATUS_OPENED, arguments)` translates a template and substitutes
  named placeholders. Values are inserted verbatim, so paths, names and error
  details are never translated.
- `plural(count, keys::COUNT_ITEM_ONE, keys::COUNT_ITEM_OTHER)` translates the
  noun and formats `<count> <noun>`.
- `has_id(id)` checks the English catalog by its plain `&str` id; every other
  lookup takes the typed `Key`.
- `Key` is a newtype over a `'static` catalog id. The `keys` module is generated
  from `en.toml` at build time, so `keys::ALL` and every constant are
  available during `cargo check`, clippy and rustdoc.
- A malformed `en.toml`, a duplicate id or two ids that map to the same Rust
  constant makes `build.rs` write a `compile_error!` into the generated module
  instead of panicking or emitting invalid Rust.
- `is_complete(locale)` reads the catalog flag. `locale_label(locale)` returns
  `Deutsch (Vorschau)` for an incomplete catalog and `Deutsch` once it is
  complete (the current German catalog is complete).
- The QA pseudolocale is generated from the English values at runtime (accent
  plus brackets); there is no hand-written pseudo catalog.

No GUI or session code passes an English source string to `t`, `fill`,
`plural`, `tr` or `message::fill`. Those functions only accept `Key`, so a raw
string is a compile error; `compile_fail` doctests on each entry point keep
that guarantee visible.

## Session

`jadeforge-session/src/message.rs` is a thin wrapper over the shared crate:

- `message::tr(key)` calls `jadeforge_i18n::t(key)`.
- `message::fill(key, arguments)` calls `jadeforge_i18n::fill`.
- There is no translator installation hook and no UI callback.

Session tests get the English catalog by default. The GUI installs the locale
each render, exactly as before. Session-owned labels return `Key`:
`CharacterField::label` and `range`, `SaveSort::label`, `ThemeChoice::label`
and `description`, `format::alignment`, `Section::label`, and the known
`item_category_id`/`style_kind_id` values. Views call `t` on those keys.

## Preview behavior

- German is complete: About shows **Deutsch**, the choice persists as
  `locale = "de"`, and `Locale::from_environment` may select it.
- The preview path remains for an incomplete catalog. While `complete = false`,
  About shows **Deutsch (Vorschau)**, filled ids render German, empty ids render
  `⟦id⟧`, and the incomplete language is not selected from the environment.
- The debug/headless tests select German through the isolated test config and
  assert translated strings from the catalog; a synthetic missing key keeps the
  marker and no-English-fallback behavior covered.

## Adding a language

1. Add a `Locale` variant in `crates/jadeforge-i18n/src/lib.rs` and update
   `code`, `ALL`, `from_tag`, `from_environment` and the serde rename.
2. Add `crates/jadeforge-i18n/locales/<code>.toml` with `complete = false` and
   the same ids as `en.toml`.
3. Add the catalog loader and the `is_complete`/`locale_label` branches.
4. Extend the locale tests and the About selector. `build.rs` regenerates the
   typed keys from `en.toml`, so a new language does not change the call shape.
5. Set `complete = true` only after every value is non-empty and its
   placeholders match the English template.

A language with more than the one/other plural distinction needs a plural
rules helper before it can be added. The current `plural` helper assumes the
same one/other distinction as English and German.

## Fluent upgrade path

The ids and the two peer catalogs map directly onto Fluent message ids. A future
migration can add Fluent as the template engine behind the same `t`/`fill`
facade, import `en.toml` and `de.toml` as message files, and move plural and
gender rules into Fluent selectors. The GUI and session call sites do not need
to change, and the typed `Key` still guarantees that no source-string keys are
introduced.

## Deliberately untranslated

- Dynamic values: character names, save folder labels, file paths, resource
  ids, game data names and descriptions.
- Technical error details after a translated wrapper, for example the
  `{error}` part of `Cannot save: {error}`.
- Product and platform names and shortcut keys.
- Language names in the selector are endonyms; the preview suffix is managed by
  `locale_label`.
- Pure format fragments and diagnostic protocol values.

## Tests

- Catalog parse, key parity, no duplicate ids, unique English values, English
  completeness and non-empty values, placeholder parity, German completeness and
  the `complete = true` gate.
- Preview markers for empty and unknown ids, no English fallback, pseudo
  generation, tag parsing and environment selection.
- Generated keys: `keys::ALL` has exactly the ids in `en.toml`, no duplicates,
  and every `Key::id()` is catalogued.
- Compile-fail doctests prove that `t`, `fill`, `plural` and the session `tr`
  reject a raw string.
- Session label and range functions: every returned id exists.
- GUI headless tests: English UI unchanged, About shows **Deutsch**, selecting it
  renders German and config persists `de`; the catalog labels are read from the
  typed keys instead of hardcoded German strings.
- `every_german_value_is_filled` is a normal gate now, not an ignored check.
- `join_list` reuses the shared separator and the shared field ids.

## Risk

- A value that renames or drops a placeholder shows its literal `{name}`; the
  placeholder test catches it.
- A malformed catalog degrades to markers on purpose. A release must not ship
  `complete = true` until the tests pass.
- Dynamic game-data categories and style kinds with no catalogued id still fall
  back to a title-cased copy of the game id; that is an intentional non-id
  fallback for unknown data.
