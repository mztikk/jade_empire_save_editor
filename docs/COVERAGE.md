# Code coverage

Line and function coverage measured with `cargo llvm-cov nextest` over the whole
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
| lines | **6 709 / 8 100 = 82.8 %** |
| functions | **656 / 939 = 69.9 %** |

| crate | lines | functions |
| --- | --- | --- |
| `jadeforge-gamedata` | 93.6 % | 90.1 % |
| `jadeforge-gui` | 87.8 % | 65.5 % |
| `jadeforge-core` | 78.2 % | 63.2 % |
| `jadeforge-session` | 77.2 % | 78.1 % |
| `jadeforge-gff` | 80.2 % | 58.6 % |
| `jadeforge-cli` | 68.3 % | 55.3 % |

Notes:

- Branch coverage is not collected; the report is line/function based.
- GUI numbers are inflated by declarative view code that runs on every render.
  Treat GUI coverage as a regression signal, not as a quality target.
- The CLI numbers are low because the integration tests call the libraries
  directly, not the `jadeforge` binary.

## Where the gaps are

| file | lines | why it matters |
| --- | --- | --- |
| `gui/src/views/gems.rs` | 61.2 % | gem actions and empty/error states are untested |
| `core/src/inventory.rs` | 64.6 % | remaining type/count error branches |
| `gamedata/src/rim.rs` | 95.0 % | done |
| `cli/src/main.rs` | 68.3 % | `add_item`, `verify` failure and argument errors |
| `core/src/henchmen.rs` | 69.7 % | list/field error paths |
| `core/src/save.rs` | 70.7 % | verification and write-failure paths |
| `gff/src/value.rs` | 72.8 % | uncommon decode branches |
| `gamedata/src/talktable.rs` | 92.1 % | done |
| `session/src/session.rs` | 75.3 % | failure paths (config, form, save/revert) |

## Target

| milestone | overall lines | core | session | gff | cli | gui | gamedata |
| --- | --- | --- | --- | --- | --- | --- | --- |
| now | 82.8 % | 78.2 % | 77.2 % | 80.2 % | 68.3 % | 87.8 % | 93.6 % |
| phase 1 | **85 %** | 82 % | 82 % | 82 % | 78 % | 89 % | 92 % |
| phase 2 | **88 %** | 86 % | 86 % | 86 % | 84 % | 90 % | 94 % |

Functions are tracked alongside but not gated until the line target is stable;
the GUI number is a regression signal, not a target (declarative view code runs
on every render).

## Plan

### 1. GUI gem flows (`views/gems.rs`, 61.2 %)

- headless interactions: add a predefined gem, move both directions, remove,
  empty amulet, no game data, filter with no match, Show more;
- assert the visible row/effect text, not just the rendered frame.

Expected: +60–80 covered lines, gui ≥ 89 %. This is the cheapest visible win
and covers user-facing behaviour the other tests skip.

### 2. Core inventory branches (`inventory.rs`, 64.6 %)

- `Count` stored as word vs byte, non-numeric count field, missing `Count`
  field, missing `ItemList`, `ItemList` that is not a list;
- add/remove with a missing player block and with an item id of 0;
- count boundaries around the error paths added in `errors.rs`.

Expected: +40–60 lines, core ≥ 80 %.

### 3. Core save and henchmen (`save.rs` 70.7 %, `henchmen.rs` 69.7 %)

- save: verification failure on a block that cannot be rewritten, temporary
  file failure (read-only directory), target without an existing file and
  backup naming;
- henchmen: missing `HenchList`, list that is not a list, missing fields
  (defaults), level 0/61 rejected, availability flags.

Expected: +40–60 lines, core ≥ 82 % together with item 2.

### 4. Session failure paths (`session.rs`, 75.3 %)

- config: invalid TOML, legacy fallback, failed store, missing directory;
- form: every invalid field, applying an unchanged form, applying after an
  error;
- staging: scan/switch while dirty, revert with and without changes, save with
  nothing to save;
- gems/styles/items without game data and open failures.

Expected: +50–70 lines, session ≥ 82 %.

### 5. gff decode leftovers (partially done; `value.rs` 72.8 %, `lib.rs` 79.6 %)

- type-tag/value mismatches (byte field holding a struct, etc.);
- label table edge cases (16-byte label, duplicate labels, missing label);
- offset/length overflow and the `fieldCount == 1` inline encoding;
- list section with zero-count and repeated indices.

Done: empty/repeated/zero lists and the 16-byte label are covered, plus the
single-field inline struct. `gff` stays at 80.2 % and `value.rs` at 72.8 %; the
remaining branches are exotic decode combinations that may not be reachable
from real data. Finish these only with synthetic payloads and a comment saying
which format rule they exercise.

### 6. CLI leftovers (`main.rs`, 68.3 %)

- `add_item` with `--dry-run`, real write + backup, zero count and a missing
  item list;
- `verify` on a corrupt savegame, unknown subcommand/help exit codes;
- `set --no-backup`.

Expected: +30–50 lines, cli ≥ 78 %.

### 7. Game data error paths (done)

- truncated RIM header/entry table, entry pointing beyond the archive, 2DA
  payload with a bad signature;
- truncated TLK header, entry offset beyond the data, zero-length strings;
- these are synthetic files in `jadeforge-gamedata/tests` and need no game.

`crates/jadeforge-gamedata/tests/errors.rs` builds RIM archives, TLK tables and
2DA tables in memory and covers bad signatures/versions, truncated tables,
counts that exceed the file, payload offsets beyond the archive, out-of-range
strings and malformed text/binary 2DA input. `gamedata` moved from 89.6 % to
**93.9 %**; `rim.rs` is at 95.0 % and `talktable.rs` at 92.1 %.

## Gating

- Run `just coverage` twice above a milestone before making it blocking.
- At 82.4 % now, add `--fail-under-lines 82` first; when phase 1 is reached,
  replace it with `--fail-under-lines 85` and add per-crate checks in the CI
  matrix (core/gff/session/cli) using the lcov file.
- The HTML report and lcov are artifacts, not committed.

## Guardrails

- Coverage is a signal, not a goal: every new test asserts behaviour or an
  error, not merely execution.
- Synthetic fixtures first; real savegames remain the format guarantee and are
  skipped when the environment is missing.
- Do not add `#[coverage(off)]` to hand-written code. The only acceptable
  exclusions are generated glue with a comment explaining why.
- Update this document in every PR that materially changes coverage.
