# Coverage audit — post-P1

> **Historical.** The P0/P1/P2 review remediation superseded these numbers.
> [docs/COVERAGE.md](COVERAGE.md) records the current baseline (286 tests,
> 89.1 % lines with the full corpus and 89.3 % without game data).

Measured on 2026-11-05 on the working tree (`c121cb8` plus the uncommitted GUI
changes), Rust 1.99.1, `cargo-llvm-cov 0.9.1`, `cargo-nextest 0.9.151`.

The P0 and P1 suites from the first audit are implemented. This document records
the new baseline and what is left.

## TL;DR

| run | lines | functions |
| --- | --- | --- |
| full corpus (real savegames + game data, 89 saves) | **8 435 / 9 062 = 93.1 %** | 824 / 1 008 = 81.7 % |
| no game data (`env -u JADEFORGE_SAVE_DIR -u JADEFORGE_GAME_DIR -u JADEFORGE_CONFIG_DIR`) | **6 774 / 9 062 = 74.8 %** | 696 / 1 008 = 69.0 % |
| non-test source lines | 7 036 / 7 488 = 94.0 % | — |
| non-test source lines, no data | 5 431 / 7 488 = 72.5 % | — |

Non-test source excludes every `#[cfg(test)]` module, including the inline
session, GFF and 2DA unit tests added during this work.

The suite now has **237 tests**. 51 still return early without the game; one GUI
test still returns early with the full corpus. `just coverage` was fixed: the
lcov export now contains per-line `DA:` records.

## Improvement over the first audit

| crate | before (full / no data) | after (full / no data) |
| --- | --- | --- |
| `jadeforge-cli` | 68.3 % / 67.7 % | **96.4 % / 94.9 %** |
| `jadeforge-core` | 78.3 % / 51.0 % | **91.9 % / 91.1 %** |
| `jadeforge-session` | 78.5 % / 43.3 % | **93.3 % / 81.9 %** |
| `jadeforge-gff` | 81.3 % / 80.7 % | **95.8 % / 95.8 %** |
| `jadeforge-gamedata` | 93.6 % / 74.3 % | **98.5 % / 97.5 %** |
| `jadeforge-gui` | 89.4 % / 31.0 % | 89.4 % / 31.0 % |
| overall | 83.6 % / 52.2 % | **93.1 % / 74.8 %** |

## P0 implemented

### Core

- `core/tests/character.rs` — every range-check error, float/name validation,
  type-preserving writes for `Word`/`Short`/`Int`/`DWord64`/`Int64`, wrong
  field types, missing optional fields, `polyplayer.sav` + campaign merging and
  the unchanged-edit `mark_clean` path. `character.rs`: 90.8 % full corpus /
  89.1 % without data. The typed-write test exposed an `Int64` gap:
  `Value::as_u64` previously returned `None`, so `CurrentXP` stored as `Int64`
  read back as 0; that is fixed and covered by the codec test.
- `core/tests/container.rs` — block helpers, metadata fast path, polyplayer
  defaults and wrong-type fallback, container parse/metadata errors, backup
  naming. `save.rs`: 92.0 % in both runs.
- `core/tests/inventory.rs` — missing/malformed lists, dangling structs,
  defaults, `Byte`/`Word` count handling, missing/non-numeric `Count`,
  failure-not-dirtying and the public `read_items`. `inventory.rs`: 97.3 %.
- `core/tests/gems_synthetic.rs` — amulet presence/tier clamp, both gem lists,
  malformed lists, nested/missing `Gem`, effect parsing, adding, moving both
  directions and removing socketed/inventory gems. `gems.rs`: 95.2 %.
- `core/tests/styles_synthetic.rs` — list errors, level-from-mask, missing
  fields, bit preservation, slot overflow, add/remove. `styles.rs`: 91.5 %.
- `core/tests/henchmen_synthetic.rs` — list errors, defaults, level clamping,
  type preservation, availability and combat mode. `henchmen.rs`: 87.0 %.

### Session

The inline `session::tests` module covers, with a synthetic `save.sac`:
save/backup/revert, no-op save, apply and silent apply early returns, item count
bounds/removal, gem move/remove, style removal, companion level and
availability, `set_error`, and the dirty-directory guard. Session production
lines: 90.9 % full corpus / 72.6 % without data.

### CLI

`cli/tests/cli.rs` invokes the real binary for `add-item` (dry run, write,
backup, `--no-backup`, zero count, missing item list), `set` on every field
type, complex-value rendering, non-editable types, path resolution, missing
blocks and the no-game-data name fallbacks. CLI: 96.4 % / 94.9 %.

## P1 implemented

### GFF codec and helpers

- `gff/src/value.rs` unit tests call `decode_field_data` directly for every
  out-of-line type, valid and malformed: truncation, bad offsets, non-UTF-8
  string/resref/text, short and oversized locstrings, blob/quaternion/vector
  truncation and unsupported types.
- `gff/tests/value.rs` covers `type_name`, `type_id`, `encode`, `encode_inline`,
  `is_in_field_data` and every `as_*` arm, including all `None` arms.
- `gff/tests/helpers.rs` covers `find`, `find_mut`, `reachable` (including
  duplicate and out-of-bounds stack entries), root-level `add`/`remove`,
  indexed leaves, missing parents, `add_field`/`remove_field` errors,
  duplicate labels, long labels, dangling struct/list targets, and the
  `selected_fields` malformed variants.
- The malformed-locstring test exposed a panic: a 5-byte locstring indexed
  `inner[4..8]` without checking the length. The decoder now returns
  `Error::Malformed`; the regression test is in `value.rs`.
- GFF moved from 83.4 % to **95.8 %** in both runs; `value.rs` is 94.5 % and
  `lib.rs` 96.1 %.

### Synthetic game-data resolver

- `gamedata/tests/synthetic.rs` builds a complete temp installation:
  `dialog.tlk`, all text 2DA tables in `override/`, binary `2DA` inside a RIM
  archive, and a `dialogf.tlk` fallback.
- It covers item/style/advance/gem/effect resolution, `****` and missing names,
  invalid numeric cells, category mapping, playable filtering, search ranking
  and limits, level-up slot flattening (including the 64-slot cap, zero and
  oversized advances, a missing `Choice_*` column and an empty table name),
  advance families/effects, gem effects/duration and sorting.
- `gamedata/tests/common/mod.rs` builds the text talk table, binary 2DA and RIM
  fixtures; `GameData::open` is exercised with missing tables, `dialogf.tlk`,
  a broken root, invalid overrides and override-to-RIM fallback.
- `Row::u32`/`int`/`is_empty`, blank rows, non-numeric row indices, empty
  headers, RIM/TLK open errors and bad magics are covered.
- `pretty_effect_name` now matches its documented `EFFECT_DAMAGEARMOR` →
  “Damage armor” behaviour; once that mapping was added the resolver tests
  passed.
- Game data moved from 74.3 % to **97.5 %** without the game (98.5 % full).

### Tooling

- `just coverage` runs nextest for the summary and exports the lcov/html with
  `cargo llvm-cov report`; the old `--summary-only` combination produced an
  lcov file without `DA:` records.
- `docs/COVERAGE.md` records both the full-corpus and no-data baselines.

## Remaining gaps

### Feature-complete GUI fixture

`gui/src/views/gems.rs` is 61.2 % with the corpus and 0 % without it. The test
fixture always copies the smallest savegame, which has no unlocked amulet, so
the gem row/action branches never render. `gems_can_be_moved_and_removed` is the
one test that still prints “skipping” with the full corpus.

Build the fixture save from GFF bytes (player, amulet tier/open list, gems,
styles, henchmen, items) and a miniature `dialog.tlk`/2DA installation, as the
new `gamedata/tests/synthetic.rs` does. Then add:

- gem rows: title, both move labels, effect text, seven-socket hint; click
  Remove and Move and assert the visible result;
- footer “Save to disk”/“Revert” interactions;
- invalid-form guards for Scan, Browse and switching savegames;
- companion `+`/`−` and Hide/Unlock buttons;
- inventory `+`/`−` and Remove buttons;
- Learn/Remove style actions.

This removes the remaining skip and lifts the no-data GUI number from 31 %.

### Small error paths

- Screenshot: short TGA, zero pixels, colour mapped, unsupported type/depth,
  truncated uncompressed/RLE packets, 8-bit greyscale.
- JSON: duplicate labels, missing struct, depth > 64, the remaining scalar
  arms and non-finite floats.
- Config: `ensure_test_path` reuse, unknown legacy keys, no-path `store`.
- `GameData::discover` with a broken test root/stale environment fallback.
- `gui/src/dialog.rs` remains hard to test headlessly.

## Do not chase these

- `config.rs:202/204/285/287`: Windows/macOS branches; only a platform matrix
  can execute them.
- `gui/src/main.rs` and `app::app`: process entry points.
- `talktable.rs:68-69`: unreachable after the preceding length checks.
- `value.rs` unsupported-type arm, size-overflow checks, `try_into` guards for
  exact-length slices: defensive and unreachable through the parser.
- `save.rs` `verify` shape mismatches and `save_to` write/sync/rename failures.
- `Gff`’s “just pushed field vanished” / second lookup guards, and the
  `usize::try_from(u32)` inline-field check on 64-bit targets.
- `henchmen::out_of_range`: all callers pass values ≤ 60.

## Suggested follow-up order

1. Build the feature-complete GUI fixture and close the interactive row/footer
   tests.
2. Add the small screenshot/JSON/config error paths.
3. Gate the **no-data** number in CI first (`--fail-under-lines 74`, then 78
   after the GUI fixture). Keep the full-corpus number as a local/nightly
   target.

A 100 % line number is not a goal: the remaining cold lines are mostly
malformed-input defences that must stay readable and strict, not merely
executed.
