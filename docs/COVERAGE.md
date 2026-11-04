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

## Baseline (2026-11-04)

| metric | covered |
| --- | --- |
| lines | **6 387 / 7 990 = 79.9 %** |
| functions | **610 / 914 = 66.7 %** |

| crate | lines | functions |
| --- | --- | --- |
| `jadeforge-gamedata` | 88.5 % | 82.9 % |
| `jadeforge-gui` | 87.8 % | 65.5 % |
| `jadeforge-core` | 77.4 % | 63.9 % |
| `jadeforge-session` | 76.7 % | 77.7 % |
| `jadeforge-gff` | 72.1 % | 54.7 % |
| `jadeforge-cli` | 48.6 % | 29.8 % |

Notes:

- Branch coverage is not collected; the report is line/function based.
- GUI numbers are inflated by declarative view code that runs on every render.
  Treat GUI coverage as a regression signal, not as a quality target.
- The CLI numbers are low because the integration tests call the libraries
  directly, not the `jadeforge` binary.

## Target

| phase | overall lines | gff | cli | core | session | gui | gamedata |
| --- | --- | --- | --- | --- | --- | --- | --- |
| baseline | 79.9 % | 72 % | 49 % | 77 % | 77 % | 88 % | 88.5 % |
| phase 1 | ≥ 84 % | ≥ 80 % | ≥ 60 % | ≥ 82 % | ≥ 79 % | ≥ 88 % | ≥ 90 % |
| phase 2 | ≥ 88 % | ≥ 85 % | ≥ 80 % | ≥ 86 % | ≥ 85 % | ≥ 89 % | ≥ 92 % |

The overall numbers matter less than the shape: the parser (`gff`), the save
writer (`core`) and the CLI are where a missed line means a corrupt or
misreported savegame. View code is worth covering only where it contains logic.

## Plan

### 1. `jadeforge-gff`: field codec matrix (largest single win)

`value.rs` sits at 62.4 %, mostly because field types that never appear in the
test savegames are never decoded.

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

### 2. `jadeforge-cli`: make the commands testable

`main.rs` is 48.6 % / 29.8 % functions.

- Move the bodies of `blocks`, `dump`, `get`, `set`, `info` and `players` into
  `commands.rs` functions that take a `&Path`, a writer and plain arguments.
- Keep `main.rs` to clap parsing, dispatch and exit codes.
- Test every command against a temporary savegame copy (the fixture helper that
  the other tests already use) and against broken inputs, asserting output and
  exit codes.
- Add a `--json` test that pins a small JSON shape (the only JSON surface).

Expected: +100–130 covered lines, `cli` to ≥ 60 % in phase 1 and ≥ 80 % later.

### 3. `jadeforge-core`: error and boundary paths

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
