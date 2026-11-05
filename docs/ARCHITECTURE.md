# Architecture

JadeForge is a Cargo workspace with six crates. The split keeps the savegame
knowledge in the lower crates so that both front ends stay thin and the
interesting logic can be tested without a window or a terminal.

```
crates/
  jadeforge-gff/      GFF block reader/writer
  jadeforge-core/     savegame container, editing model, verification
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
| `save` | the container: parsing, block access, locking in byte ranges, atomic save |
| `character` | typed view of the player: read, validate, write, report changes |
| `inventory` | item list: list, count changes, add, remove |
| `styles` | fighting styles: names, levels, upgrade masks, add/remove |
| `henchmen` | companions: level and availability |
| `screenshot` | `SaveImage.tga` decoder (TGA 24/32 bit, RLE and raw) |
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
2. Edits are range checked (`level` 1..=255, health 0..=1_000_000, names at most
   64 characters, ...) and typed editors run on a working copy, so a rejected
   edit leaves the savegame and its dirty flag untouched.
3. Only fields that actually change are touched; `apply_character` returns the
   list of changes so callers can show a diff.
4. Saving verifies the full round trip and the domain invariants first.
5. The new file is written to an exclusive temporary sibling with the original
   permissions, fsynced and moved into place; the parent directory is fsynced
   too, so an interrupted save cannot truncate the original.
6. Backups use unique nanosecond/counter names, are written to a temporary file
   and renamed, and a pre-existing file or symlink is never followed or
   overwritten.
7. The file identity (size, mtime, inode on Unix) is recorded at open time; a
   savegame that changed underneath the editor is reported as a conflict
   instead of overwritten. Saving through a symlink keeps the link and writes
   through to its target.

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
is optional: without the game files the UI falls back to numeric ids. A file
that exists but cannot be parsed is collected in `GameData::warnings` and shown
when the savegame is opened instead of being reported as "missing".

## `jadeforge-cli`

A thin wrapper over the core: `info`, `blocks`, `character`, `items`, `get`,
`set`, `dump`, `verify`, `add-item`. It is intentionally useful on its own and
serves as an end-to-end test target (see `crates/jadeforge-cli/tests`).

## `jadeforge-session`

UI independent state and behaviour of the desktop application. The `Session`
fields are private; views use read-only accessors such as `items()`, `gems()`
and `message()`. UI actions return the typed `SessionError`; form parsing returns `FormError`. `session` scans
the save folder, holds the opened savegame, applies form edits, searches the
item database and saves or reverts; the row/message model lives in
`session/model.rs`, the editable form in `session/form.rs`; `config` is the small TOML settings file
(save folder and list sorting; old `key=value` files are still read). The crate
depends on `jadeforge-core` and `jadeforge-gamedata` but on no windowing
toolkit, so its tests run on every platform without the GUI system libraries.
`jadeforge-gui` re-exports the modules, which keeps the views' `crate::session`
paths unchanged.

## `jadeforge-gui`

Freya 0.4 application with a small, explicit structure:

| module | responsibility |
| --- | --- |
| `app/` | root component (`mod.rs`), header/preview/footer (`shell.rs`), save list and sort controls (`sidebar.rs`), tab bar (`navigation.rs`) |
| `dialog` | the native folder dialog (`rfd`) with a test seam for the headless suite |
| `theme` | palette and the Freya component theme (dark surfaces, gold accents) |
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
checked. The fixture uses `JADEFORGE_SAVE_DIR` when it is set and otherwise
builds a complete synthetic savegame and installation in a temporary folder, so
the suite runs in CI without the retail game. `cargo run -p jadeforge-gui --example snapshot -- <dir> <png>
[TAB|all] [--height N] [--query TEXT]` produces the screenshots used in the
documentation.

## Startup path

Opening the window must not wait for the savegame corpus. The sequence is:

1. `Session::empty(directory)` builds the shell without opening any savegame and
   loads the game data (a few milliseconds on the retail install).
2. The first frame renders with the "Scanning for savegames…" message.
3. `app_with_directory` starts a `use_future` task. It spawns a worker thread
   running `Session::scan_entries` and awaits the result through a futures
   channel, so the Freya executor is never blocked by file I/O.
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

Tests that need the game look for `JADEFORGE_SAVE_DIR` and skip with a note when
it is not set, so CI stays green on machines without Jade Empire.

## Dependencies

Deliberately small:

* `clap` for the CLI,
* `serde_json`, only for the `jadeforge-cli dump --json` export (never for settings or
  internal data flow),
* `thiserror` for errors,
* `serde` and `toml` for the settings file,
* `freya` for the GUI,
* `freya-testing` (dev only) for headless UI tests,
* `futures-channel` for the one-shot hand-off from the scan worker to the GUI
  future.
