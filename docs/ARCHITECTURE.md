# Architecture

JadeForge is a Cargo workspace with three crates. The split keeps all savegame
knowledge in one place so that both front ends stay thin and the interesting
logic can be tested without a window or a terminal.

```
crates/
  jadeforge-core/   format reader/writer, editing model, verification
  jadeforge-cli/    `jadeforge` command line tool
  jadeforge-gui/    `jadeforge-gui` desktop application (Freya)
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

## `jadeforge-cli`

A thin wrapper over the core: `info`, `blocks`, `character`, `items`, `get`,
`set`, `dump`, `verify`, `add-item`. It is intentionally useful on its own and
serves as an end-to-end test target (see `crates/jadeforge-cli/tests`).

## `jadeforge-gui`

Freya 0.4 application. The `session` module owns all behaviour and has no Freya
types, which makes it unit testable. `app` renders the session and forwards
input:

* scan a save folder, list savegames with level, name and play time,
* edit name, level, experience, gold, unspent points, vitals, attributes and
  chapter/kills,
* edit inventory stacks and add or remove items,
* apply changes in memory, save to disk, revert.

The GUI is tested headlessly through `freya-testing`: the real component tree is
rendered off-screen, a savegame is opened by clicking it, and the rendered
frame is checked. `cargo run -p jadeforge-gui --example snapshot -- <dir> <png>`
produces the screenshots used in the documentation.

## Test strategy

| layer | file | needs the game? |
| --- | --- | --- |
| format unit tests | `gff::path`, `gff::value`, `character`, `session` | no |
| synthetic savegames | `crates/jadeforge-core/tests/synthetic.rs` | no |
| real savegames | `tests/roundtrip.rs`, `tests/rewrite.rs`, `tests/editing.rs` | yes, skipped otherwise |
| CLI end to end | `crates/jadeforge-cli/tests/cli.rs` | synthetic |
| UI | `crates/jadeforge-gui/tests/ui.rs` | yes for the open-a-save test, otherwise no |

Tests that need the game look for `JADEFORGE_SAVE_DIR` and skip with a note when
it is not set, so CI stays green on machines without Jade Empire.

## Dependencies

Deliberately small:

* `clap` for the CLI,
* `serde_json` for dumps,
* `thiserror` for errors,
* `freya` for the GUI,
* `freya-testing` (dev only) for headless UI tests.
