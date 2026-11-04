# Architecture

JadeForge is a Cargo workspace with three crates. The split keeps all savegame
knowledge in one place so that both front ends stay thin and the interesting
logic can be tested without a window or a terminal.

```
crates/
  jadeforge-core/     format reader/writer, editing model, verification
  jadeforge-gamedata/ talk table and 2DA readers for names
  jadeforge-cli/      `jadeforge` command line tool
  jadeforge-gui/      `jadeforge-gui` desktop application (Freya)
```

## `jadeforge-core`

| module | responsibility |
| --- | --- |
| `gff::mod` | block header, tables, parsing into `Gff`/`Struct`/`Field` |
| `gff::value` | the `Value` enum, encoding and decoding of every field type |
| `gff::write` | deterministic serializer for a block |
| `gff::path` | dotted paths such as `Stats.Focus.STAT_CUR` or `ItemList[3].Count` |
| `save` | the container: parsing, block access, locking in byte ranges, atomic save |
| `character` | typed view of the player: read, validate, write, report changes |
| `inventory` | item list: list, count changes, add, remove |
| `styles` | fighting styles: names, levels, upgrade masks, add/remove |
| `henchmen` | companions: level and availability |
| `screenshot` | `SaveImage.tga` decoder (TGA 24/32 bit, RLE and raw) |
| `json` | JSON export used by `jadeforge dump` and debugging |
| `error` | one error enum with context for every failure |

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
 save_to(): verify() ─▶ to_bytes() ─▶ temp file ─▶ fsync ─▶ rename
```

Blocks keep the byte range they occupy in the original file. `SaveGame::to_bytes`
copies untouched blocks verbatim and only serializes blocks that were actually
edited, which is what makes "open and save" byte identical and keeps diffs
minimal.

### Writer invariants

The writer emits a canonical layout:

* structs keep their indices and order,
* every struct's fields stay in order,
* the field table is the concatenation of all structs' fields,
* single-field structs use the inline field-index encoding,
* multi-field structs get contiguous ranges in the field-index section,
* field data and list data are de-duplicated by payload,
* labels keep their original order, new labels are appended.

Before anything is written, `SaveGame::verify` re-parses the produced bytes and
compares every struct and label. A mismatch aborts the save.

### Safety model

1. Parsing is strict: bad magic, out-of-range offsets, unknown field types and
   inconsistent block sizes are errors, never "best effort".
2. Edits are range checked (`level` 1..=255, health 0..=1_000_000, names at most
   64 printable characters, ...).
3. Only fields that actually change are touched; `apply_character` returns the
   list of changes so callers can show a diff.
4. Saving verifies the full round trip first.
5. The new file is written to a temporary sibling and moved into place, so an
   interrupted save can never truncate the original.
6. A timestamped backup is created next to the savegame by default.

## `jadeforge-gamedata`

Names live in the installation, not in the savegame. The crate reads

* `dialog.tlk` (format `TLK V4.0`, fixed 10 byte entries with UTF-8 offsets),
* text 2DA tables from `override/` (`items.2da`, `stylesuperlist.2da`).

`GameData::discover` walks up from the save folder (`JADEFORGE_GAME_DIR`
overrides) and resolves items and styles once, caching the decoded names.
`search_items` powers the item picker. Every table is optional: without the game
files the UI falls back to numeric ids.

## `jadeforge-cli`

A thin wrapper over the core: `info`, `blocks`, `character`, `items`, `get`,
`set`, `dump`, `verify`, `add-item`. It is intentionally useful on its own and
serves as an end-to-end test target (see `crates/jadeforge-cli/tests`).

## `jadeforge-gui`

Freya 0.4 application with a small, explicit structure:

| module | responsibility |
| --- | --- |
| `session` | all behaviour (no Freya types): loading, editing, search, save/revert |
| `app` | the shell: header, save list with screenshot, tabs, action bar |
| `theme` | palette and the Freya component theme (dark surfaces, gold accents) |
| `widgets` | cards, fields, badges, dividers |
| `views` | one module per tab: overview, character, styles, inventory, companions |
| `form` | the text input states of the character sheet |

The shell renders the session and forwards input:

* scan a save folder, list savegames with level, name and play time,
* edit name, level, experience, gold, unspent points, vitals, attributes and
  chapter/kills,
* edit inventory stacks and add or remove items,
* apply changes in memory, save to disk, revert.

The GUI is tested headlessly through `freya-testing`: the real component tree is
rendered off-screen, a savegame is opened by clicking it, tabs are switched, an
item is searched and added, and the rendered frames are checked. The tests build
a miniature installation (talk table + two 2DA files + one savegame) in a
temporary folder. `cargo run -p jadeforge-gui --example snapshot -- <dir> <png>
[TAB|all] [--height N] [--query TEXT]` produces the screenshots used in the
documentation.

## Test strategy

| layer | file | needs the game? |
| --- | --- | --- |
| format unit tests | `gff::path`, `gff::value`, `character`, `session` | no |
| synthetic savegames | `crates/jadeforge-core/tests/synthetic.rs` | no |
| real savegames | `tests/roundtrip.rs`, `tests/rewrite.rs`, `tests/editing.rs` | yes, skipped otherwise |
| CLI end to end | `crates/jadeforge-cli/tests/cli.rs` | synthetic |
| game data | `crates/jadeforge-gamedata/tests/real_data.rs` | yes, skipped otherwise |
| UI | `crates/jadeforge-gui/tests/ui.rs` | yes (saves + game files), skipped otherwise |

Tests that need the game look for `JADEFORGE_SAVE_DIR` and skip with a note when
it is not set, so CI stays green on machines without Jade Empire.

## Dependencies

Deliberately small:

* `clap` for the CLI,
* `serde_json` for dumps,
* `thiserror` for errors,
* `freya` for the GUI,
* `freya-testing` (dev only) for headless UI tests.
