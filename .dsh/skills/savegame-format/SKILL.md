---
name: savegame-format
description: Use when working on the Jade Empire savegame format in jadeforge-core, adding or changing field types, debugging a savegame that fails to parse or round trip, or validating edits against real savegames.
---

# Working on the savegame format

Read [docs/FORMAT.md](../../../docs/FORMAT.md) first. It documents the container,
the block header, all three tables, the type encodings and the player paths.
Anything you learn that is not in there belongs in there.

## Finding a savegame to test with

Real savegames live in the game folder (`/media/Games/Heroic/Jade Empire/Save`
on this machine, or whatever `JADEFORGE_SAVE_DIR` points at). The repository
never contains them.

```sh
export JADEFORGE_SAVE_DIR="/media/Games/Heroic/Jade Empire/Save"

# every block of every savegame must parse
cargo test -p jadeforge-core --test roundtrip -- --nocapture

# every block must survive a serialize/parse round trip
cargo test -p jadeforge-core --test rewrite -- --nocapture

# editing must survive a save/reload
cargo test -p jadeforge-core --test editing -- --nocapture
```

The round trip tests print how many savegames and blocks they checked. If they
print "skipping", the environment variable is not set.

## Inspecting a savegame by hand

```sh
cargo run -q -p jadeforge-cli -- "<SAVE_DIR>/Auto 56 - 16h 22min - level 15" blocks
cargo run -q -p jadeforge-cli -- "<SAVE_DIR>/Auto 56 - 16h 22min - level 15" info
cargo run -q -p jadeforge-cli -- "<SAVE_DIR>/Auto 56 - 16h 22min - level 15" dump save.sav --path PlayerCreature.Stats
```

`dump` without `--path` prints the whole block as JSON (structs become objects,
lists become arrays, blobs become hex strings).

To look at raw bytes, copy the savegame into the repository scratch folder first
(`scratch/` is ignored by git) and use `xxd`. `/tmp` is not stable in every
sandbox.

## Facts that are easy to get wrong

- Field table entries are `(type, label, data)`, in that order. Swapping type and
  label parses without error but produces nonsense.
- Labels are 16 bytes, fixed size, and may use all 16 bytes without a NUL
  terminator (`VisualEffectNode`).
- A struct with exactly one field stores the field index **inline** in the data
  word. A struct with two or more fields stores a **byte offset** into the
  field-index section.
- Field indices in that section are 32 bit and tile the section exactly; any gap
  or overlap means the interpretation is wrong.
- `totalSize` of a localized string counts the bytes *after* the size word, so a
  talk-table-only string is 8 bytes with `totalSize = 4`.
- Block size is always `listIndicesOffset + listIndicesSize`.
- All integers are little endian.

## Adding a type or a field

1. Write the decoder in `crates/jadeforge-core/src/gff/value.rs` and the encoder
   in the same enum (`Value::encode`, `Value::encode_inline`).
2. Extend `Value::type_id`, `Value::type_name`, `Value::is_in_field_data` and
   `PartialEq`.
3. Add the type to the table in `docs/FORMAT.md`.
4. Add a case to `crates/jadeforge-core/tests/synthetic.rs` so CI covers it
   without game data.
5. Run the three real-savegame tests above; a new type that no savegame uses
   will not be exercised there, so the synthetic test matters.

## Before you finish

```sh
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all --check
```

If a format change alters what the writer produces, explain in the commit
message why the new layout is still valid for the game (the engine reads by
offset, so table order and de-duplication are free, but offsets, counts and
type tags are not).
