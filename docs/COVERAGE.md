# Code coverage

Line and function coverage measured with `cargo llvm-cov nextest` over the whole
workspace. Two numbers matter:

- **full corpus** — with a real installation, so the tests that need savegames
  and game data run;
- **no game data** — what CI sees, because the real-data tests return early when
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

## Baseline (2026-11-05)

| metric | full corpus | no game data |
| --- | --- | --- |
| lines | **15 895 / 17 832 = 89.1 %** | **15 913 / 17 832 = 89.2 %** |
| functions | **944 / 1 215 = 77.7 %** | **947 / 1 215 = 77.9 %** |
| regions | **9 636 / 10 745 = 89.7 %** | **9 645 / 10 745 = 89.8 %** |
| tests run | 285 | 285 |

Several crates keep unit tests inside `src/`, so those test lines are part of
the totals above. The previous non-test-only table was measured with a smaller
suite and is not directly comparable, so it is not repeated here.

| crate | full corpus | no game data |
| --- | --- | --- |
| `jadeforge-cli` | 89.8 % | 89.8 % |
| `jadeforge-core` | 83.6 % | 83.1 % |
| `jadeforge-gamedata` | 95.8 % | 95.6 % |
| `jadeforge-gff` | 89.7 % | 89.7 % |
| `jadeforge-gui` | 88.1 % | 90.0 % |
| `jadeforge-session` | 92.3 % | 91.8 % |

Notes:

- Branch coverage is not collected; the report is line/function/region based.
- No test is `#[ignore]`d. Real-corpus tests return early without
  `JADEFORGE_SAVE_DIR`, while the GUI suite builds its synthetic fixture and
  still runs all 33 headless tests. `nextest` reports 285 run and 0 skipped in
  both modes.
- GUI numbers are inflated by declarative view code that runs on every render.
  Treat GUI coverage as a regression signal, not as a quality target.
- The full-corpus GUI number is lower because the real-corpus fixture copies the
  smallest savegame, which has no amulet; the synthetic no-game fixture exercises
  the gem view.
- CLI coverage is real binary coverage: the tests invoke
  `CARGO_BIN_EXE_jadeforge-cli` and cover `add-item`, every editable field type
  and the no-game-data fallbacks.
- GFF and game-data coverage is now almost entirely synthetic: both crates are
  above 95 % without a game installation.

## Where the gaps are

| file | full corpus | no game data | why it matters |
| --- | ---: | ---: | --- |
| `gui/src/views/gems.rs` | 59.2 % | 81.1 % | the real-corpus fixture has no amulet; the synthetic one does |
| `gui/src/views/styles.rs` | 84.8 % | 82.4 % | all views need an opened savegame |
| `gui/src/views/character.rs` | 100 % | 100 % | fully exercised by the synthetic save |
| `gui/src/dialog.rs` | 63.2 % | 63.2 % | native folder dialog; not headless-testable |
| `core/src/henchmen.rs` | 82.0 % | 82.0 % | the remaining lines are defensive/unreachable |
| `core/src/screenshot.rs` | 79.8 % | 79.8 % | malformed TGA error paths |
| `session/src/session.rs` | 92.3 % | 91.9 % | a few game-data helpers |
| `gff/src/value.rs` | 94.6 % | 94.6 % | the remaining decode/accessor lines are unreachable or defensive |
| `gff/src/lib.rs` | 88.9 % | 88.9 % | add/remove branches and duplicate-stack guards |
| `gamedata/src/gamedata.rs` | 98.2 % | 98.2 % | stale test-root/env discovery and a non-UTF-8 filename branch |

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
  the effect text, seven-socket hint, item search/add and pagination. Add
  footer Save/Revert, invalid-form preservation, companion buttons and
  inventory count/remove interactions to close the remaining view branches.
- Add a headless accessibility assertion for the tab, sort and savegame-row
  controls; the labels and keyboard activation are implemented, but
  `freya-testing` does not expose the accessibility tree yet.

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
| now | 89.1 % | 89.2 % | 83.6 % | 92.3 % | 89.8 % | 90.0 % | 95.8 % |
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
