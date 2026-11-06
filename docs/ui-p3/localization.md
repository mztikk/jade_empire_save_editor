# UI localization

## Current state

The interface is keyed by stable semantic ids. English is the source-of-truth
catalog and is complete; the German catalog currently has 354 empty ids out of
473 and is marked `complete = false`, so it is offered as a preview. About shows
it as **Deutsch (Vorschau)**. A filled id renders German; an empty id renders a
bracketed id marker and never falls back to English.

The catalog no longer uses English source strings as keys, so editing an
English label cannot orphan a translation. The German text that existed before
this pass is preserved byte for byte, including the 10 merged labels that were
prefilled from an existing translation. The final consolidation renamed the
remaining sentence-slug ids and merged repeated wrappers: 22 duplicate ids are
gone, and every removed id was empty in German, so the filled count is
unchanged.

## Crate layout

`crates/jadeforge-i18n` owns the locale, catalogs and strictness:

```
crates/jadeforge-i18n/
  Cargo.toml
  locales/en.toml   complete = true
  locales/de.toml   complete = false
  src/lib.rs        Locale, catalogs, t/fill/plural, markers, pseudo
  tests/source_scan.rs
```

It depends only on `serde` and `toml`, no Freya and no session types.
`jadeforge-session` re-exports `Locale` and the catalog API for the settings
file and session messages; `jadeforge-gui::i18n` re-exports the same API so the
views keep using `crate::i18n::*`.

## Format

```toml
complete = false

[messages]
"shell.save" = "Save to disk"
```

Both files use `complete` plus a `messages` table. Parsing uses
`deny_unknown_fields`. A malformed catalog degrades to an empty catalog, so
every id becomes a marker; it never falls back to English.

`en.toml` is the source of truth and must be `complete = true`. `de.toml` keeps
`complete = false` until every id has a non-empty value. Key parity between the
two catalogs and placeholder parity for every non-empty German value are
enforced by tests.

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

## Runtime

- `install(locale)` stores the active locale in a thread-local; `current()`
  reads it. Every render pass installs the configured locale.
- `t("id")` looks up the active catalog only. A missing or empty value returns
  a cached `⟦id⟧` marker. English missing ids are markers too.
- `fill("id", arguments)` translates a template and substitutes named
  placeholders. Values are inserted verbatim, so paths, names and error details
  are never translated.
- `plural(count, "count.item.one", "count.item.other")` translates the noun and
  formats `<count> <noun>`.
- `has_id(id)` checks the English catalog.
- `is_complete(locale)` reads the catalog flag. `locale_label(locale)` returns
  `Deutsch (Vorschau)` while German is incomplete and `Deutsch` once it is
  complete.
- The QA pseudolocale is generated from the English values at runtime (accent
  plus brackets); there is no hand-written pseudo catalog.

No GUI or session code passes an English source string to `t`, `fill`,
`plural`, `tr` or `message::fill`. A source scan fails on any literal that is
not an id in `en.toml` or does not match the id grammar.

## Session

`jadeforge-session/src/message.rs` is a thin wrapper over the shared crate:

- `message::tr(id)` calls `jadeforge_i18n::t(id)`.
- `message::fill(id, arguments)` calls `jadeforge_i18n::fill`.
- There is no translator installation hook and no UI callback.

Session tests get the English catalog by default. The GUI installs the locale
each render, exactly as before. Session-owned labels return ids:
`CharacterField::label` and `range`, `SaveSort::label`, `ThemeChoice::label`
and `description`, `format::alignment`, `Section::label`, and the known
`item_category_id`/`style_kind_id` values. Views call `t` on those ids.

## Preview behavior

- German is selectable explicitly in About and the choice persists as
  `locale = "de"`.
- `Locale::from_environment` does not select German while the catalog is
  incomplete; it falls back to English. It can select it once
  `complete = true`.
- Filled German ids render German. Empty ids render `⟦id⟧`. The About page
  itself shows **Deutsch (Vorschau)**.
- The debug/headless tests select German through the isolated test config and
  assert both a translated string and a marker for a known empty id.

## Adding a language

1. Add a `Locale` variant in `crates/jadeforge-i18n/src/lib.rs` and update
   `code`, `ALL`, `from_tag`, `from_environment` and the serde rename.
2. Add `crates/jadeforge-i18n/locales/<code>.toml` with `complete = false` and
   the same ids as `en.toml`.
3. Add the catalog loader and the `is_complete`/`locale_label` branches.
4. Extend the locale tests and the About selector; update the source scan only
   if the new language needs a different call shape.
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
to change, and the id scan still guarantees that no source-string keys are
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

- Catalog parse, key parity, no duplicate ids, English completeness and
  non-empty values, placeholder parity, German completeness only when
  `complete = true`.
- Preview markers for empty and unknown ids, no English fallback, pseudo
  generation, tag parsing and environment selection.
- Source scans for `t`, `fill`, `plural` in `crates/jadeforge-gui/src` and
  `tr`/`fill` in `crates/jadeforge-session/src`.
- Session label and range functions: every returned id exists.
- GUI headless tests: English UI unchanged, About shows **Deutsch (Vorschau)**,
  selecting it renders German and a marker, config persists `de`.
- List the remaining empty German ids with:
  `cargo test -p jadeforge-i18n --lib every_german_value_is_filled -- --ignored`

## Risk

- A value that renames or drops a placeholder shows its literal `{name}`; the
  placeholder test catches it.
- A malformed catalog degrades to markers on purpose. A release must not ship
  `complete = true` until the tests pass.
- Dynamic game-data categories and style kinds with no catalogued id still fall
  back to a title-cased copy of the game id; that is an intentional non-id
  fallback for unknown data.
