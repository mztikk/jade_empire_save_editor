# Code coverage

Line and function coverage measured with `cargo llvm-cov` over the whole
workspace, including the tests that need real savegames and game data.

```sh
# Real-savegame tests are skipped without these variables; the numbers below
# were measured with a real installation.
JADEFORGE_SAVE_DIR="$HOME/games/Jade Empire/Save" \
JADEFORGE_GAME_DIR="$HOME/games/Jade Empire" \
JADEFORGE_CONFIG_DIR="$(mktemp -d)" \
just coverage
```

`just coverage` writes:

- a line/function summary to the terminal,
- `coverage/lcov.info` for tooling,
- `coverage/html/index.html` for browsing per file (gitignored).

## Baseline (2026-11-05)

| metric | covered |
| --- | --- |
| lines | **6 626 / 8 076 = 82.0 %** |
| functions | **639 / 926 = 69.0 %** |

| crate | lines | functions |
| --- | --- | --- |
| `jadeforge-gamedata` | 89.6 % | 83.8 % |
| `jadeforge-gui` | 87.8 % | 65.5 % |
| `jadeforge-core` | 77.4 % | 63.9 % |
| `jadeforge-session` | 77.2 % | 78.1 % |
| `jadeforge-gff` | 79.8 % | 58.6 % |
| `jadeforge-cli` | 68.3 % | 55.3 % |

Notes:

- Branch coverage is not collected; the report is line/function based.
- GUI numbers are inflated by declarative view code that runs on every render.
  Treat GUI coverage as a regression signal, not as a quality target.
- The CLI numbers are low because the integration tests call the libraries
  directly, not the `jadeforge` binary.

## Target

| phase | overall lines | gff | cli | core | session | gui | gamedata |
| --- | --- | --- | --- | --- | --- | --- | --- |
| baseline | 82.0 % | 80 % | 68 % | 77 % | 77 % | 88 % | 90 % |
| phase 1 | ≥ 84 % | ≥ 80 % | ≥ 60 % | ≥ 82 % | ≥ 79 % | ≥ 88 % | ≥ 90 % |
| phase 2 | ≥ 88 % | ≥ 85 % | ≥ 80 % | ≥ 86 % | ≥ 85 % | ≥ 89 % | ≥ 92 % |

The overall numbers matter less than the shape: the parser (`gff`), the save
writer (`core`) and the CLI are where a missed line means a corrupt or
misreported savegame. View code is worth covering only where it contains logic.

## Plan

### 1. `jadeforge-gff`: field codec matrix (done)

`crates/jadeforge-gff/tests/codec.rs` now builds a block with every field type,
round trips it, mutates a locstring back to the legacy type 12, and exercises
the parser/writer error paths (short header, bad magic/version, bad label index,
unknown type, list index out of range, oversized string length, truncated
locstring, duplicate labels, type mismatches, dangling structs). `gff` moved
from 72.1 % to **79.8 %** and the overall project from 79.9 % to **81.3 %**.

The remaining decode branches are covered too: empty string, empty and maximal
(255 byte) resrefs, zero-length void, and locstrings without strref/text.
`gff` is now at **79.8 %**, just below the phase-1 target of 80 %.

- Add `crates/jadeforge-gff/tests/codec.rs` with a synthetic GFF block that
  contains **every** field type: inline byte/char/word/short/dword/int/float,
  offset dword64/int64/double/string/resref/void/struct/list, both locstring
  types (talk-table-only and inline text), quaternion and vector3.
- Round trip each value through `Gff::to_bytes` → `Gff::parse` and assert both
  the decoded value and the re-encoded bytes.
- Add error cases: truncated field data, string length beyond the section,
  locstring with `totalSize <= 4`, list index out of range, struct index out of
  range, unknown type id, field count that overflows the field-index section.
- Cover `Value` accessors per variant (`as_u64`, `as_i64`, `as_f32`, …) so the
  function coverage stops being dominated by never-called helpers.

Expected: +120–180 covered lines, `gff` to ≥ 80 %.

### 2. `jadeforge-cli`: end-to-end tests (done)

`crates/jadeforge-cli/tests/cli.rs` builds a synthetic savegame and runs the
real binary, so clap parsing, dispatch and every read handler are exercised:
`blocks`, `info`, `character`, `get`, `dump` (whole block and `--path`),
`verify`, `items`, `styles` and `companions`. It also covers `set` with
`--dry-run` (file untouched), the real write plus backup creation, bad field
paths and values, a missing save, and the folder-argument resolution.

`cli` moved from 48.6 % to **68.3 %** lines (55.3 % functions); the project is
at **82.0 %**. Remaining: error formatting branches inside `main.rs` and a few
`add_item` options.

### 3. `jadeforge-core`: error and boundary paths (next)

Targets: `inventory.rs` 57.5 %, `save.rs` 68.8 %, `henchmen.rs` 69.7 %.

- Inventory: count 0, count 255, adding/removing a missing item, removing the
  last stack, `Count` overflow, unknown item id.
- Save: missing backup directory, backup naming, verification failure (inject a
  serializer that corrupts a block), read-only destination, round-trip
  preservation of untouched blocks.
- Henchmen: level/availability bounds, missing list, duplicate entries.
- Character: absent optional fields (`Alignment`, `UnassignedPts`, `Gold`) and
  their creation on write.

Expected: +80–120 covered lines, `core` to ≥ 82 %.

### 4. `jadeforge-session`: failure paths

`session.rs` is 75.3 %.

- Config: invalid TOML, legacy fallback, failed store, missing config directory.
- Form: each invalid field and the resulting message, applying an unchanged
  form, applying after a validation error.
- Staging: scanning/opening another save while dirty, revert with and without
  changes, save with nothing to save.
- Gems/items/styles: empty savegame, no game data, missing level table, style
  without a level.

Expected: +50–70 covered lines, `session` to ≥ 79 % (phase 2 ≥ 85 %).

### 5. `jadeforge-gui`: behaviour, not layout

`views/gems.rs` is 61.2 %; most other view files are already high simply because
rendering executes them.

- Gem flows: add via the picker, move both ways, remove, empty amulet, no game
  data, filter with no match, Show more.
- Page-size consistency (already tested) and per-tab scroll (already tested).
- Error banners: invalid character input, save failure, broken savegame.

Expected: +40–60 covered lines, but the goal is tested behaviour, not the
percentage.

### 6. Malformed-save corpus

Build a small set of synthetic broken saves in `crates/jadeforge-gff/tests` and
`crates/jadeforge-core/tests`:

- truncated container, missing `SAV ` magic, wrong version,
- struct/field/list offsets beyond the block,
- label index out of range,
- unknown field type,
- a block that parses but fails to round trip.

This corpus feeds the parser and save-writer error paths from items 1–3 and
keeps them covered without game data.

## Guardrails

- Coverage is a signal, not a goal: do not add tests that execute code without
  asserting behaviour just to move the number.
- Prefer synthetic fixtures for the gate so coverage works without the game
  installation; keep the real-savegame tests for the format guarantees.
- Do not use `#[coverage(off)]` for hand-written code. The only acceptable
  exclusions are generated glue with a comment explaining why.
- Add `--fail-under-lines` only after two consecutive runs above the target:
  phase 1 `80`, phase 2 `85` for the overall project (see the table above for
  per-crate goals). Raise it in the same PR that documents the new baseline.
- Update this document in every PR that materially changes coverage.
