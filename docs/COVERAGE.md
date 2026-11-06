# Code coverage

Line and function coverage measured with `cargo llvm-cov nextest` over the whole
workspace. Two numbers matter:

- **full corpus**: with a real installation, so the tests that need savegames
  and game data run;
- **no game data**: what CI sees, because the real-data tests return early when
  `JADEFORGE_SAVE_DIR`/`JADEFORGE_GAME_DIR` are unset.

`just coverage` runs the suite once, prints the summary, and writes
`coverage/lcov.info` (per-line records, usable for gating) plus
`coverage/html/index.html`. Both are gitignored.

```sh
# Full corpus
JADEFORGE_SAVE_DIR="$HOME/games/Jade Empire/Save" \
JADEFORGE_GAME_DIR="$HOME/games/Jade Empire" \
JADEFORGE_CONFIG_DIR="$(mktemp -d)" \
just coverage

# CI baseline: no game installation
env -u JADEFORGE_SAVE_DIR -u JADEFORGE_GAME_DIR -u JADEFORGE_CONFIG_DIR \
    just coverage
```

## Baseline (2026-11-06, after the P1 UI work)

| metric | full corpus | no game data |
| --- | --- | --- |
| lines | **12 186 / 13 709 = 88.9 %** | **12 233 / 13 709 = 89.2 %** |
| functions | **1 225 / 1 530 = 80.1 %** | **1 230 / 1 530 = 80.4 %** |
| regions | **19 666 / 22 223 = 88.5 %** | **19 723 / 22 223 = 88.8 %** |
| tests run | 382 | 382 |

Several crates keep unit tests inside `src/`, so those test lines are part of
the totals above.

| crate | full corpus | no game data |
| --- | --- | --- |
| `jadeforge-cli` | 90.3 % | 90.3 % |
| `jadeforge-core` | 83.8 % | 83.5 % |
| `jadeforge-gamedata` | 95.6 % | 95.2 % |
| `jadeforge-gff` | 92.5 % | 92.5 % |
| `jadeforge-gui` | 88.5 % | 90.1 % |
| `jadeforge-session` | 88.5 % | 88.4 % |

Notes:

- Branch coverage is not collected; the report is line/function/region based.
- No test is `#[ignore]`d. Real-corpus tests return early without
  `JADEFORGE_SAVE_DIR`, while the GUI suite builds its synthetic fixture and
  still runs all 107 headless tests without game data. `nextest` reports 382
  run; the only skip is the opt-in windowed suite.
- GUI numbers are inflated by declarative view code that runs on every render.
  Treat GUI coverage as a regression signal, not as a quality target.
- The full-corpus GUI number is lower because the real-corpus fixture copies the
  smallest savegame, which has no amulet; the synthetic no-game fixture exercises
  the gem view.
- The P1 work added geometry, window-fit, validation, memoisation, target-size,
  focus-restoration and an 89-save render test. `gui/src/geometry.rs` is covered
  by its own unit tests but its winit monitor path cannot run headlessly;
  `session/src/session/form.rs` is lower than the rest of the session crate
  because the per-field validation branches for every numeric type are not all
  exercised by one fixture.
- CLI coverage is real binary coverage: the tests invoke
  `CARGO_BIN_EXE_jadeforge-cli` and cover `add-item`, every editable field type
  and the no-game-data fallbacks.
- GFF and game-data coverage is now almost entirely synthetic: both crates are
  above 92 % without a game installation.

## Where the gaps are

| file | full corpus | no game data | why it matters |
| --- | ---: | ---: | --- |
| `gui/src/views/gems.rs` | 51.6 % | 79.9 % | the real-corpus fixture has no amulet; the synthetic one does |
| `gui/src/views/styles.rs` | 86.3 % | 84.1 % | all views need an opened savegame |
| `gui/src/views/character.rs` | 100 % | 100 % | fully exercised by the synthetic save |
| `gui/src/geometry.rs` | 79.8 % | 79.8 % | the pure fit tests run; the monitor query needs a real event loop |
| `gui/src/dialog.rs` | 64.5 % | 64.5 % | native folder dialog; not headless-testable |
| `session/src/session/form.rs` | 71.5 % | 71.5 % | per-field validation has one branch per numeric type |
| `core/src/save.rs` | 78.4 % | 78.0 % | canonical-block comparison and write error paths |
| `core/src/henchmen.rs` | 82.0 % | 81.0 % | the remaining lines are defensive/unreachable |
| `gff/src/lib.rs` | 88.8 % | 88.8 % | add/remove branches and duplicate-stack guards |
| `gamedata/src/gamedata.rs` | 98.1 % | 97.1 % | stale test-root/env discovery and a non-UTF-8 filename branch |

## Recently completed

- **Synthetic P0 suites.** Character range checks and typed writes, legacy
  `polyplayer.sav`/campaign merging, inventory branches and `read_items`, gems,
  styles and henchmen are built from synthetic GFF blocks. Core went from 51 %
  to 91 % without game data; inventory is at 97 %.
- **Session actions.** Save/backup/revert, apply/apply-silently, item count and
  removal, gem move/remove, style removal, companion level/availability and
  directory handling have unit tests with a synthetic savegame.
- **CLI.** `add-item`, `--dry-run`, backups, zero count, missing item lists,
  every `set` field type and the no-game-data fallbacks are covered.
- **GFF codec and helpers.** All `Value` variants/accessors, field-data
  decoding of every out-of-line type (including truncated, non-UTF-8 and
  malformed locstring payloads), structural decode errors,
  `selected_fields` failure modes, `find`/`find_mut`/`reachable`, mutation
  errors and `validate`. GFF moved from 83 % to 96 %.
- **Synthetic game-data resolver.** A complete temp installation now exercises
  items, styles, level-up tables, advances, effects, gems, `dialogf.tlk`,
  missing tables, invalid rows, `****` names and an `override`-falls-back-to-RIM
  case. Game data moved from 74 % to 98 % without the game.
- **Two decoder bugs found and fixed.** `Value::as_u64` now accepts `Int64`
  (a `CurrentXP` stored as `Int64` read back as 0), and a 5-byte locstring no
  longer panics on an unchecked `inner[4..8]` slice.
- **Effect label fix.** `EFFECT_DAMAGEARMOR` now produces the documented
  “Damage armor” instead of “Damagearmor”.
- **`just coverage` fixed.** The old recipe passed `--summary-only` together
  with `--lcov`, producing an lcov file with zero `DA:` records. It now runs
  nextest for the summary and exports lcov/html with `cargo llvm-cov report`.
- **Startup scan is asynchronous.** `Session::empty` renders the shell without
  parsing savegames; `use_future` runs `Session::scan_entries` on a worker and
  applies the result. A failed scan is reported instead of hanging on
  “Scanning for savegames…”.
- **Metadata fast path.** `SaveGame::metadata` walks the container and
  `Gff::selected_fields` decodes only `PlayerCreature.FirstName`,
  `PlayerCreature.Level` and `Campaign.TimePlayed`. On the 89-savegame corpus
  this is about 39 ms warm instead of 1.87 s for the full character parse, and
  `metadata_matches_the_full_character_parse` checks every save.
- **Review remediation (2026-11-05).** Strict GFF layout/reference/allocation
  checks and a decoded-data budget; transactional save/backup path with
  conflict detection, exclusive temps, unique backups and permission
  preservation; fixed seven-slot amulet model; checked character reads with
  type-preserving writes; style family API that cannot create gaps; checked
  TGA/JSON/game-data parses; scan tokens and preserved form input; synthetic UI
  fixture so the headless suite runs without game data. The suite is now 280
  tests. Deterministic mutation passes and the `fuzz/` cargo-fuzz targets cover
  the remaining hostile-input surface.

## Remaining plan

### 1. Remaining GUI interaction paths

- The synthetic fixture now covers the gem rows, both move directions, removal,
  the effect text, seven-socket hint, item search/add, pagination, the Revert
  confirmation and item-removal confirmation. Add invalid-form preservation,
  companion buttons and inventory count interactions to close the remaining
  view branches.
- Headless accessibility assertions now cover static text values, headings,
  live status, tab/list roles, selected and expanded states, the single Tab
  stop of the save list, and activation of a tab through a synthetic AccessKit
  `Click` action (`TestingRunner::send_accessibility_action`). Remaining:
  associate labels with `Input`s (still needs a Freya Input label API) and give
  every repeated row action a contextual name.

### 2. Small error paths

- Screenshot: short TGA, zero pixels, colour mapped, unsupported type/depth,
  truncated uncompressed/RLE packets, 8-bit greyscale.
- JSON: duplicate labels, missing struct, depth > 64, remaining scalar arms and
  non-finite floats.
- Config: `ensure_test_path` reuse, unknown legacy keys, no-path `store`.
- `GameData::discover` with a broken test root/stale environment fallback.

## Target

| milestone | full corpus | no game data | core | session | cli | gui | gamedata |
| --- | --- | --- | --- | --- | --- | --- | --- |
| now | 89.1 % | 89.3 % | 83.6 % | 92.3 % | 89.8 % | 90.0 % | 95.8 % |
| phase 1 | **90 %** | **90 %** | 85 % | 93 % | 91 % | 91 % | 96 % |
| phase 2 | **92 %** | **92 %** | 88 % | 94 % | 93 % | 93 % | 97 % |

Functions are tracked alongside but not gated until the line target is stable;
the GUI number is a regression signal, not a target (declarative view code runs
on every render).

## Gating

- Run `just coverage` twice above a milestone before making it blocking.
- The no-game-data number is now the CI baseline: add
  `cargo llvm-cov nextest --workspace --fail-under-lines 89` as a non-blocking
  artifact job first, then make it blocking above the phase-1 milestone. The
  full-corpus number is a local/nightly target, not a hosted-runner gate.
- `coverage/lcov.info` contains per-line `DA:` records and can drive per-crate
  checks from the CI matrix (core/gff/session/cli).
- The HTML report and lcov are artifacts, not committed.

## Guardrails

- Coverage is a signal, not a goal: every new test asserts behaviour or an
  error, not merely execution.
- Synthetic fixtures first; real savegames remain the format guarantee and are
  skipped when the environment is missing.
- Do not add `#[coverage(off)]` to hand-written code. The only acceptable
  exclusions are generated glue with a comment explaining why.
- Update this document in every PR that materially changes coverage.
