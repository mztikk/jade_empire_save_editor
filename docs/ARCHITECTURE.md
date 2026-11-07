# Architecture

JadeForge is a Cargo workspace with seven crates. The split keeps the savegame
knowledge in the lower crates so that both front ends stay thin and the
interesting logic can be tested without a window or a terminal.

```
crates/
  jadeforge-gff/      GFF block reader/writer
  jadeforge-core/     savegame container, editing model, verification
  jadeforge-i18n/     stable-id English/German catalogs and locale selection
  jadeforge-session/  UI independent session state and settings
  jadeforge-gamedata/ talk table and 2DA readers for names
  jadeforge-cli/      `jadeforge` command line tool
  jadeforge-gui/      `jadeforge-gui` desktop application (Freya)
```

## `jadeforge-gff`

| module | responsibility |
| --- | --- |
| `lib` | block header, tables, parsing into `Gff`/`Struct`/`Field` |
| `value` | the `Value` enum, encoding and decoding of every field type |
| `write` | deterministic serializer for a block |
| `path` | dotted paths such as `Stats.Focus.STAT_CUR` or `ItemList[3].Count` |

The crate is deliberately dependency free apart from `thiserror`: it knows
nothing about characters, items or the savegame container, and it is the only
place that touches the byte layout of a block.

### Writer invariants

The writer emits a canonical layout:

* structs keep their indices and order,
* every struct's fields stay in order,
* the field table is the concatenation of all structs' fields,
* single-field structs use the inline field-index encoding,
* multi-field structs get contiguous ranges in the field-index section,
* field data and list data are de-duplicated by payload,
* labels keep their original order, new labels are appended.

## `jadeforge-core`

The GFF blocks themselves are handled by `jadeforge-gff`; this crate adds the
savegame container and the typed editing model.

| module | responsibility |
| --- | --- |
| `save` | the container: parsing, block access, locking in byte ranges, atomic save, block payload restore for undo |
| `character` | typed view of the player: read, validate, write, report changes |
| `inventory` | item list: list, count changes, add, remove |
| `styles` | fighting styles: names, levels, upgrade masks, add/remove |
| `henchmen` | companions: level and availability |
| `screenshot` | `SaveImage.tga` decoder (TGA 24/32 bit, RLE and raw) |
| `backup` | listing and verified atomic restore of the `*.jadeforge-backup-*` files |
| `json` | JSON export used by `jadeforge-cli dump --json`; the only JSON surface |
| `error` | one error enum with context, wrapping the GFF layer's error |

### Data flow

```
save.sac ──parse──▶ SaveGame { blocks: [Block { gff, span, modified }] }
                          │
          character()/items() view
                          │
             user edits (typed, range checked)
                          │
        apply_character() mutates the GFF tree of one block
                          │
 save_to(): verify() ─▶ conflict check ─▶ exclusive temp ─▶ fsync ─▶ rename ─▶ directory fsync
```

Blocks keep the byte range they occupy in the original file. `SaveGame::to_bytes`
copies untouched blocks verbatim and only serializes blocks that were actually
edited, which is what makes "open and save" byte identical and keeps diffs
minimal.

Before anything is written, `SaveGame::verify` re-parses the produced bytes and
compares every block name, version, root, label and struct. A mismatch aborts
the save. The cheap domain invariants are checked as well: the amulet list keeps
its seven slots, socketed gems fit the tier, and a style level matches its mask.

### Safety model

1. Parsing is strict: bad magic, out-of-range offsets, unknown field types and
   inconsistent block sizes are errors, never "best effort".
2. Edits are range checked (`level` 1..=32, experience 0..=1_000_000, unspent
   points 0..=97, appearance 0..=236, chapter 0..=8, health 0..=1_000_000, gold
   and kills 0..=2_147_483_647, alignment 0..=255, names at most 64 characters,
   ...) and typed editors run on a working copy, so a rejected edit leaves the
   savegame and its dirty flag untouched.
3. Only fields that actually change are touched; `apply_character` returns the
   list of changes so callers can show a diff.
4. Saving verifies the full round trip and the domain invariants first.
5. The new file is written to an exclusive temporary sibling with the original
   permissions applied to the open handle before data is written, fsynced and
   moved into place; the parent directory is fsynced too, so an interrupted
   save cannot truncate the original.
6. Backups use unique nanosecond/counter names, are written completely to a
   temporary file, and are published without replacing an existing name.
7. The file identity (size, mtime, inode on Unix, creation time on Windows) is
   sampled from the same open handle that supplies the bytes; a savegame that
   changed underneath the editor is reported as a conflict instead of
   overwritten. Saving through a symlink keeps the link and writes through to
   its target.

## `jadeforge-gamedata`

Names live in the installation, not in the savegame. The crate reads

* `dialog.tlk` (`TLK V4.0`, 10 byte entries, UTF-8 string data),
* text 2DA tables from `override/` and binary `2DA V2.b` tables from the RIM
  archives (`items.2da`, `gems.2da`, `effects.2da`, `stylesuperlist.2da`, ...).

`GameData::discover_result` walks up from the save folder (relative paths are
anchored at the current directory first; `JADEFORGE_GAME_DIR` overrides) and
resolves items, styles and style upgrades once, caching the decoded names. A
found-but-unreadable installation is reported instead of being hidden as
"missing"; a broken `dialog.tlk` falls back to `dialogf.tlk`. `StyleAdvance.2da` supplies the effect, cost and description of a slot, while
the style's own `<Style>LV.2da` table defines the purchasable slots: bit *i* of
`StyleLevelInfo` is slot `i+1`, and the slot points at an advance row.
`stylesuperlist.StyleData` selects a row of `styledata.2da`, whose `levelup2da`
column names the table. Those tables (and `gems.2da`, `effects.2da`) live in
`data/miniglobal-a.rim` and `data/globalunload-a.rim` as binary `2DA V2.b`
resources; both archives are searched. `override/` text tables take precedence
(case-insensitive lookup, lossy decoding), which is what makes mods work. RIM
and text/binary 2DA versions are validated, row numbers and allocations are
bounded, and every cell lookup is confined to the declared string pool. Styles
without a level-up table are read-only in the editor.
`tests/modded.rs` builds a small modded installation and asserts the resolution.
`search_items` powers the item picker, ranking prefix matches first. Every table
is optional: without the game files the compiled-in English name index fills item
and style names and the item categories, while the savegame supplies the rest
(item category, style level, weapon count, icon and weapon resources, time
spent). An installed game always wins over the bundled names. A file that exists
but cannot be parsed is collected in `GameData::warnings` and shown when the
savegame is opened instead of being reported as "missing". The names index,
its generation command and its precedence are documented in
[docs/NAMES.md](NAMES.md); it is the only game-derived file in the repository.

## `jadeforge-cli`

A thin wrapper over the core: `info`, `blocks`, `character`, `items`, `get`,
`set`, `dump`, `verify`, `add-item`, plus the maintainer-only `export-names`
command. It is intentionally useful on its own and serves as an end-to-end test
target (see `crates/jadeforge-cli/tests`).

## `jadeforge-i18n`

The interface catalog is keyed by stable semantic ids, not English sentences.
After the final consolidation the ids use `area.subject.action` names and
repeated wrappers share one template with a placeholder (`error.section_unreadable`,
`status.cannot_add`, `status.added_label`). The crate owns the two peer
catalogs, the locale enum and the strictness rules:

| module | responsibility |
| --- | --- |
| `lib` | `Key`, generated `keys`, `Locale`, `t`/`fill`/`plural`, `install`/`current`, `is_complete`, `has_id`, `locale_label`, catalog loading, generated pseudo locale |
| `build.rs` | generates the typed `keys` module from `en.toml` and fails the build through a generated `compile_error!` on a malformed or colliding catalog |
| `locales/en.toml` | English source of truth, `complete = true` |
| `locales/de.toml` | German peer catalog, `complete = true` |

A missing or empty value renders a cached `⟦id⟧` marker; there is no English
fallback. A malformed catalog degrades to markers, never to English. The typed
`Key` newtype and the build-generated `keys` constants make a raw catalog-id
string a compile error, and a test keeps `keys::ALL` in lockstep with
`en.toml`. Placeholder parity and English completeness are enforced by tests.
German is complete and may be selected from the environment. An incomplete
catalog stays preview-only (`Deutsch (Vorschau)`, `⟦id⟧` markers) until it is
marked complete.

## `jadeforge-session`

UI independent state and behaviour of the desktop application. The `Session`
fields are private; views use read-only accessors such as `items()`, `gems()`
and `message()`. UI actions return the typed `SessionError`; form parsing returns `FormError`. `session` scans
the save folder, holds the opened savegame, applies form edits, searches the
item database and saves or reverts; the row/message model lives in
`session/model.rs`, the editable form in `session/form.rs`; `config` is the small TOML settings file
(save folder, list sorting, theme, text scale, locale and window remembering;
old `key=value` files are still read), while `preferences` and `locale` hold the
typed values it stores. The crate
depends on `jadeforge-core` and `jadeforge-gamedata` but on no windowing
toolkit, so its tests run on every platform without the GUI system libraries.
`jadeforge-gui` re-exports the modules, which keeps the views' `crate::session`
paths unchanged.

The session also owns the P3 editing-safety state:

- a bounded undo/redo history. Each action stores the canonical payloads of
  the blocks it changed plus its field-level before/after rows; restoring a
  payload is exact at block level and an undone structural edit removes the
  appended struct again. History is capped at 100 entries / 32 MiB and cleared
  on save and reload.
- `pending_changes` groups those rows for the review page before a save.
- a disk stamp (size + mtime) that detects an external change and explains the
  conflict; `reload_from_disk` refuses while dirty.
- `backups` and `restore_backup`, which delegate to the core's verified backup
  module and keep the safety copy in `last_backup`.

`is_dirty` is a content comparison: an edit that is undone clears the flag. The
load-time baseline stores a 128 bit fingerprint (two independently salted 64 bit
hashes) of each block's canonical bytes instead of the canonical bytes
themselves. A savegame is a few megabytes, so the copy was a noticeable part of
the editor's idle memory; 16 bytes per block answer the same equality question.
The fingerprints are process-local and never written anywhere.

## `jadeforge-gui`

Freya 0.5 application with a small, explicit structure:

| module | responsibility |
| --- | --- |
| `app/` | root component (`mod.rs`), header/preview/footer (`shell.rs`), save list and sort controls (`sidebar.rs`), tab bar (`navigation.rs`) |
| `dialog` | the native folder dialog (`rfd`) with a test seam for the headless suite |
| `theme` | three semantic palettes, the active thread-local `UiStyle`, scaled type/spacing/radius/layout tokens and the Freya component theme |
| `i18n` | active locale, typed `Key`/`keys` re-exports, English/pseudolocale catalog and the `t(...)` lookup |
| `views/review` | pending-changes page before a save |
| `views/backups` | backup browser with timestamps, original path and verified restore |
| `renderer` | `JADEFORGE_RENDERER` parsing and the Skia GPU resource cache limit |
| `app::HoverKey` | one shared hover state for save, item, style rows and upgrade orbs |
| `widgets` | cards, fields, badges, dividers |
| `views` | one module per tab (overview, character, styles, inventory, companions); item and style rows expand to show the game's description |
| `form` | the text input states of the character sheet |
| `assets/` | icon master (`icon.svg`) and the generated PNG/ICO/ICNS set; `ICON_PNG` embeds the 256 px PNG |

The shell renders the session and forwards input. The savegame list has its own
filter and sort (`session::visible_entries`, natural ordering so `Auto 2` comes
before `Auto 10`). The folder dialog is the operating system's own, wrapped in
`dialog` so the headless tests can answer it. The layout is fluid: the sidebar uses a
percentage width with a minimum, tab labels never wrap, and wide rows wrap
instead of clipping.

The window icon is embedded with `ICON_PNG` and passed to
`WindowConfig::with_icon`; `with_app_id("jadeforge")` matches the Linux desktop
entry so Wayland compositors can resolve the hicolor icon. `build.rs` embeds the
same mark in Windows executables through `winresource`, and
`ci/macos-bundle.sh` wraps the macOS binary in `JadeForge.app` so the Dock can
show the ICNS. `assets/README.md` documents the generated files and
`just icons`.

`renderer` owns the two memory knobs that are not view code. The default stays
Freya's automatic GPU choice; `JADEFORGE_RENDERER` can force `software` (Skia on
the CPU through `softbuffer`), `opengl` or `vulkan`, which is how a machine
where the graphics driver is too expensive trades GPU acceleration for roughly
half the resident memory. Every window also gets a 32 MiB Skia GPU resource cache
limit instead of Skia's 256 MiB default ([Freya's cache option](https://github.com/marc2332/freya/issues/1207)):
the editor's largest texture is one 512 px screenshot, and an uncapped cache
keeps GPU resources around after resizes. `RendererLogPlugin` listens for
`WindowCreated`/`GraphicsDriverChanged` and prints the driver Freya actually
selected, which is the only way to notice that a requested Vulkan backend fell
back to OpenGL. Freya 0.5 ships OpenGL, Vulkan, Metal and software drivers; it
has no Direct3D driver, so there is no Windows Direct3D preference to expose.
The savegame sidebar is a `VirtualScrollView`, so only the rows inside it are
built; each row carries two tooltips and two text paragraphs, and building all
of them for a few hundred saves dominated the idle footprint. The P3 item, style
and gem pickers use the same mechanism through `widgets::picker_list`: the full
filtered corpus is available, the list shows seven rows at a time, and an
expanded item or style row grows to its measured description height while the
other rows stay collapsed.

There are no hardcoded installation paths anywhere. The folder comes from
`JADEFORGE_SAVE_DIR`, the settings file or the native folder dialog, in that order.

* scan a save folder, list savegames with level, name and play time,
* edit name, level, experience, gold, unspent points, vitals, attributes and
  chapter/kills,
* edit inventory stacks and add or remove items,
* apply every edit in memory immediately, save to disk, revert.

The GUI is tested headlessly through `freya-testing`: the real component tree is
rendered off-screen, a savegame is opened by clicking it, tabs are switched, a
style level is raised, an item is searched and added, the pagination of the item
database is exercised, a broken savegame is reported and the rendered frames are
checked. Accessibility assertions read the effective AccessKit nodes (including
the values Freya 0.5 fills while building the tree) and activate a tab through a
synthetic AccessKit `Click` action. The fixture uses `JADEFORGE_SAVE_DIR` when it
is set and otherwise builds a complete synthetic savegame and installation in a
temporary folder, so the suite runs in CI without the retail game.
`cargo run -p jadeforge-gui --example snapshot -- <dir> <png> [TAB|all]
[--height N] [--query TEXT]` produces the screenshots used in the documentation.

A second, opt-in suite (`crates/jadeforge-gui/tests/live.rs`) covers the window
path: it launches the real application through `freya-winit`, opens a real
Wayland window with the production `app::window_config`, drives it with KWin's
fake-input protocol on KDE (or with windowed plugin events on other
compositors), and captures the presented window with Spectacle. A `Probe` plugin
publishes the live label/paragraph tree so assertions use text and layout
rectangles instead of image diffs. The suite is gated by `JADEFORGE_LIVE=1` plus
`JADEFORGE_LIVE_APPROVED=1` and runs one scenario per child process because winit
requires one event loop on the
main thread. It opens real windows, so it is **never run without explicit
permission** and only when the headless suite cannot cover the case
([AGENTS.md](AGENTS.md) rule 13). See [LIVE-UI.md](LIVE-UI.md) for the process
model, backends and the desktop/VM requirements.

## Startup path

Opening the window must not wait for the savegame corpus. The sequence is:

1. `Session::empty(directory)` builds the shell without opening any savegame and
   loads the game data (a few milliseconds on the retail install).
2. The first frame renders with the "Scanning for savegames…" message.
3. `app_with_directory` starts a `use_future` task. It spawns a worker thread
   running `Session::scan_entries` and awaits the result through a futures
   channel, so the Freya executor is never blocked by file I/O. Freya 0.5's
   `use_future` is reactive, but this callback reads no state before its async
   block, so it starts once and cannot restart itself through the revision it
   bumps when the scan lands.
4. The future calls `Session::apply_entries_for` with the directory and scan
   token it started from. Stale results for a folder the user has already left
   are discarded, then the revision is bumped and the list appears.
5. If the worker dies, `Session::report_scan_failure` replaces the pending
   message instead of leaving the UI waiting forever.

`SaveGame::metadata` and `Gff::selected_fields` keep the scan cheap: only
`PlayerCreature.FirstName`, `PlayerCreature.Level` and `Campaign.TimePlayed`
are decoded, not the tens of thousands of fields in the PlayerCreature block.
On the 89-savegame corpus this was measured at about 39 ms warm (the earlier
140 ms figure was a cold-cache run) instead of 1.87 s for the full character
parse, and `tests/metadata.rs` checks every save against `Character::read`.

Directory changes still scan synchronously (about 27 ms warm); they happen on an
explicit user action and use the same `scan_entries`/`apply_entries` path as the
background scan, so the messages cannot diverge.

## Test strategy

| layer | file | needs the game? |
| --- | --- | --- |
| GFF parser/codec/helpers | `crates/jadeforge-gff/src/*`, `tests/*` | no |
| session unit tests | `crates/jadeforge-session/src/session.rs`, `config.rs` | no |
| synthetic savegames | `crates/jadeforge-core/tests/*` | no |
| real savegames | `tests/roundtrip.rs`, `tests/rewrite.rs`, `tests/editing.rs`, `tests/gems.rs` | yes, skipped otherwise |
| CLI end to end | `crates/jadeforge-cli/tests/cli.rs` | synthetic |
| game data | `crates/jadeforge-gamedata/tests/real_data.rs` | yes, skipped otherwise |
| game data synthetic | `crates/jadeforge-gamedata/tests/synthetic.rs` | no |
| mutation pass | `crates/jadeforge-gff/tests/mutations.rs`, `crates/jadeforge-gamedata/tests/mutations.rs` | no |
| fuzz targets | `fuzz/` (`cargo-fuzz`, not a workspace member) | no |
| benchmark | `crates/jadeforge-core/benches/save_corpus.rs` | optional corpus |
| UI | `crates/jadeforge-gui/tests/ui.rs` | synthetic by default, real corpus when set |
| windowed UI | `crates/jadeforge-gui/tests/live.rs` | synthetic, opt-in, Wayland only |

Tests that need the game look for `JADEFORGE_SAVE_DIR` and skip with a note when
it is not set, so CI stays green on machines without Jade Empire.

## Dependencies

Deliberately small:

* `clap` for the CLI,
* `serde_json`, only for the `jadeforge-cli dump --json` export (never for settings or
  internal data flow),
* `thiserror` for errors,
* `serde` and `toml` for the settings file (theme, text scale, locale and
  window remembering are typed values),
* `freya` for the GUI,
* `accesskit` for the roles, states, actions and live regions the views set
  directly; it must stay on the same 0.25 release line as Freya,
* `freya-testing` (dev only) for headless UI tests,
* `freya-winit`, `wayland-client` and `wayland-protocols-plasma` (dev only,
  Linux) for the windowed UI suite and its KWin fake-input driver; `image` and
  `serde_json` are dev-only companions for compositor screenshots and KWin's
  JSON D-Bus replies,
* `futures-channel` for the one-shot hand-off from the scan worker to the GUI
  future.
