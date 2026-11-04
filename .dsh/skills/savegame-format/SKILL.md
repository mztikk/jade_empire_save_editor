---
name: savegame-format
description: Use when working on the Jade Empire savegame format in jadeforge-core, adding or changing field types, resolving names from the game data, debugging a savegame that fails to parse or round trip, or validating edits against real savegames.
---

# Working on the savegame format

Read [docs/FORMAT.md](../../../docs/FORMAT.md) first. It documents the container,
the block header, the tables, the type encodings, the player paths and where
names come from. Anything you learn belongs in there.

## Finding data to test with

Real savegames live in the game installation's `Save` folder. The repository
never contains them, and no path is hardcoded anywhere: point
`JADEFORGE_SAVE_DIR` at your installation and `JADEFORGE_GAME_DIR` at the game
root (the folder that holds `dialog.tlk`).

```sh
export JADEFORGE_SAVE_DIR="/path/to/Jade Empire/Save"
export JADEFORGE_GAME_DIR="/path/to/Jade Empire"

cargo test -p jadeforge-core --test roundtrip -- --nocapture   # parse + byte identical
cargo test -p jadeforge-core --test rewrite   -- --nocapture   # serialize/parse every block
cargo test -p jadeforge-core --test editing   -- --nocapture   # character edits
cargo test -p jadeforge-core --test extras    -- --nocapture   # styles, companions, screenshots
cargo test -p jadeforge-gamedata --test real_data              # item/style names
```

If a test prints "skipping", the environment variables did not reach it.

## Inspecting by hand

```sh
cargo run -q -p jadeforge-cli -- "<SAVE_DIR>/<save>" blocks
cargo run -q -p jadeforge-cli -- "<SAVE_DIR>/<save>" styles
cargo run -q -p jadeforge-cli -- "<SAVE_DIR>/<save>" items
cargo run -q -p jadeforge-cli -- "<SAVE_DIR>/<save>" dump save.sav --path PlayerCreature.Stats
```

Copy savegames into `scratch/` (git ignored) before touching them; `/tmp` is not
stable in every sandbox.

## Facts that are easy to get wrong

- Field table entries are `(type, label, data)`, in that order.
- Labels are 16 bytes, fixed size, and may use all 16 without a NUL.
- A struct with exactly one field stores the field index **inline**; two or more
  fields store a **byte offset** into the field-index section. That section is
  tiled exactly by the multi-field structs.
- `totalSize` of a localized string counts the bytes after the size word.
- Block size is always `listIndicesOffset + listIndicesSize`.
- Style levels: `StyleLevel == 1 + population count of the StyleLevelInfo mask`.
  Never set one without the other.
- Alignment: the field is absent until the first decision; in the 76 savegames
  that have it the values run 131..255 with a peak at exactly 255, so the scale
  is 0..255 with 128 neutral (`DEFAULT_ALIGNMENT`). Below 128 is the Closed
  Fist, above is the Open Palm. There is no second alignment field.
- Item `Type` values in savegames map to 2DA categories: 1 gems, 4 powerups,
  5 weapons, 6 plot_items, 7 mgweapons, 8 improve. Measure before adding more.

## Game data

`crates/jadeforge-gamedata` reads `dialog.tlk` (TLK V4.0: header of 24 bytes,
10 byte entries `(u32 flags, u32 offset, u16 length)`, UTF-8 text) and text 2DA
tables. It is read only and optional; the editor must keep working without it.

## Before you finish

```sh
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all --check
```

Update `docs/FORMAT.md` whenever format knowledge changed, and add a synthetic
test in `crates/jadeforge-core/tests/synthetic.rs` for code paths that the real
savegames do not exercise.
