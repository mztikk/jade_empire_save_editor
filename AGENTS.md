# AGENTS.md

Guidelines for anyone (human or agent) working in this repository. Read this
before changing code. It is short on purpose; details live in
[docs/FORMAT.md](docs/FORMAT.md) and [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md).

## What this project is

JadeForge is a production-grade savegame editor for **Jade Empire**, written in
Rust with a [Freya](https://freyaui.dev/) desktop UI. It targets Linux, Windows
and macOS. It edits `save.sac` files directly and must never corrupt them.

The savegame format is a BioWare GFF variant. Every fact we know about it is
written down in [docs/FORMAT.md](docs/FORMAT.md); that document is the source of
truth and must be updated whenever the understanding of the format changes.

## Commands

```sh
cargo build --workspace                       # build everything
cargo test --workspace                        # all tests
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all --check

# Tests against real savegames (skipped when the variable is not set)
JADEFORGE_SAVE_DIR="$HOME/games/Jade Empire/Save" \
JADEFORGE_GAME_DIR="$HOME/games/Jade Empire" \
JADEFORGE_CONFIG_DIR="$(mktemp -d)" cargo test --workspace

# Render the UI into a PNG without a window (screenshots, layout debugging)
cargo run -p jadeforge-gui --example snapshot -- "<SAVE_DIR>" docs/screenshot.png [TAB|all]
# Add `--height 1800 --query viper` for tall layouts and search results,
# `--width 900` to check narrow windows.
```

The same commands are available through `just` (see the `justfile`): `just
check` runs formatting, clippy and the tests, `just snapshot …` renders the UI,
and `just publish` builds the fat-LTO distribution binaries. `just coverage` runs
`cargo llvm-cov` and writes `coverage/lcov.info` and `coverage/html` (see
[docs/COVERAGE.md](docs/COVERAGE.md) for the current numbers and the plan). The
cargo commands above remain canonical.

## Non-negotiable rules

1. **No `unsafe` code.** The workspace denies it in `Cargo.toml`. There is no
   reason to bypass this.
2. **Never write an unverified savegame.** All writes go through
   `SaveGame::save_to`, which verifies the round trip, writes to a temporary
   sibling and renames it. Do not add a "fast path" that writes in place.
3. **Keep backups.** `SaveOptions::default()` creates one; do not change the
   default.
4. **Parse strictly.** A savegame that does not match the format must produce a
   readable error, never a partially decoded tree.
5. **Preserve untouched blocks byte for byte.** Only blocks that were edited may
   be re-serialized. This property is tested and must stay.
6. **Range check every edit** in `jadeforge-core`, not in the UI.
7. **No game data in the repository.** Test savegames are never committed;
   tests that need them look for `JADEFORGE_SAVE_DIR` and skip otherwise.
8. **Never hardcode a machine path.** No default game or save directory may be
   baked into the source: users are asked once (the native folder dialog), the choice is
   stored in the config file, and `JADEFORGE_SAVE_DIR`/`JADEFORGE_GAME_DIR`
   override it. Tests use the variables only and skip when they are unset.
9. **Treat the game installation as read only.** `jadeforge-gamedata` only
   reads `dialog.tlk` and the 2DA tables; it must never open a file for writing
   and the editor must keep working when those files are absent.

## Code style

- `cargo fmt` is authoritative.
- Clippy runs with `-D warnings` and the workspace enables `clippy::pedantic`.
  If a lint genuinely does not fit, add a scoped `#[allow(...)]` **with a
  reason comment**; do not weaken the workspace lint configuration.
- The workspace also **denies** `unwrap_used`, `panic`, `todo`, `unimplemented`,
  `unreachable`, `exit`, `string_slice` and `panic_in_result_fn`, and denies
  `missing_docs`. `expect` is allowed only where a preceding check proves the
  invariant; `crates/jadeforge-gff` denies it crate-wide and converts those
  cases into `jadeforge_gff::Error::Malformed` instead. Tests may panic
  (`clippy.toml`), but the test allowances do not cover helper functions in
  `tests/*.rs`; add a crate-level allow there if a test helper needs it.
- `just strict` reports the lints that are not gated yet
  (`expect_used`, `indexing_slicing`, `arithmetic_side_effects`, `nursery`) as
  an audit that never gates the build. Run it before promoting a lint: only add
  a lint to the workspace config once its count is zero, so the `-D warnings`
  gate stays meaningful.
- Public items have doc comments that explain behaviour, not syntax.
- Errors carry context: use `Error::Corrupt`/`Error::Gff` with the path, block
  or field that failed, not bare strings; the GFF crate uses its own
  `Error::Malformed` for the same reason.
- No panics on user data. `expect`/`unwrap` are only allowed where a preceding
  check proves the invariant, and the message must state why.
- Keep the core free of UI dependencies; keep the GUI free of format knowledge.
- JSON is reserved for machine-readable output and machine-to-machine
  communication: the CLI's `dump --json` export (`jadeforge-core::json`). Do not
  use JSON for configuration, persisted settings or internal data flow; the
  settings file is TOML and savegames stay binary.

## Game data (names, item and style tables)

- `crates/jadeforge-gamedata` locates the installation from the save folder
  (`JADEFORGE_GAME_DIR` wins), parses `dialog.tlk` (TLK V4.0) and the text 2DA
  tables in `override/`.
- Every lookup degrades gracefully: when the files are missing the UI falls back
  to numeric ids, it never fails to open a savegame because of missing game data.
- Names come from the talk table via the 2DA `STRREF_NAME`/`NameRef` columns. Do
  not hardcode names that the tables already provide.
- The item category mapping (`gems` → 1, `powerups` → 4, `weapons` → 5,
  `plot_items` → 6, `mgweapons` → 7, `improve` → 8) was derived from real
  savegames and lives in `save_type_for_category`. New categories need evidence
  from a savegame before they are added.
- Style kinds come from the template label prefix (`StyleKind`), not from the
  `Type` column: the table stores dual weapon styles as `MONSTER` and the
  Spirit Thief style as `SpiritThief`, so `Type` is not a usable description.
- A style's upgrade mask is not a counter and not row indexed: bit *i* is the
  *slot* `i + 1` of the style's `<Style>LV.2da` table, and each slot points at a
  `StyleAdvance` row for effect, cost and name. The level is `1 + popcount`.
  `set_style_slot` must keep mask and level in sync; never write one without the
  other. Do not go back to "bit = advance row - 1": it contradicts real saves.
- Upgrades are changed per family: clicking the tier *n* orb buys every missing
  tier up to *n*, clicking an owned orb removes it and everything above it. The
  mask of a family is therefore always a gap-free prefix; do not add a
  free-selection path that can create gaps, and do not edit the level directly
  (it is derived from the mask and has no UI control). There is no "game rules"
  toggle any more: only the style's real slots exist, so there is nothing to
  switch off.
- `<Style>LV.2da` tables define the purchasable slots. They ship as binary
  `2DA V2.b` resources inside `data/miniglobal-a.rim`; the mapping is
  `stylesuperlist.StyleData` -> `styledata.2da` row -> `levelup2da` column.
  `override/` text tables take precedence (mods). A style without a table is
  read-only in the editor: do not guess slots, it would write wrong upgrades.
- A test walks every savegame and asserts each mask bit is inside its style's
  slot range; keep that test green.
- Development placeholders (`[TEMP] ...`, `temp_*`, `zzz*`, `cut_*`) are hidden
  from the item picker (`is_playable_item`) but stay resolvable by id.

## Reading the Jade Empire wiki

Facts about the game files come from the community wiki at
`jadeempire-modding.fandom.com`. Its reader URLs sit behind a bot challenge:
`/wiki/<Page>` and `?action=raw` answer **HTTP 403** with a "Just a moment..."
page. That is a bot check, **not** a missing page — never conclude that a page
does not exist because of it, and never give up on the wiki after one failure.

Read pages through the MediaWiki API instead. This one returns the wikitext:

```sh
curl -s 'https://jadeempire-modding.fandom.com/api.php?action=parse&page=Styleadvance.2da&prop=wikitext&format=json&formatversion=2'
```

A page that genuinely does not exist answers **HTTP 200** with
`{"error":{"code":"missingtitle", ...}}`. The API returns 200 for its own errors
too, so read the body, never the status code: a mistyped parameter comes back as
`{"error":{"code":"badvalue"}}` and an unknown `prop` is silently dropped with a
`warnings` key. Find the right title with
`action=query&list=search&srsearch=<terms>` and list a namespace with
`action=query&list=categorymembers&cmtitle=Category:2da`. The full recipe, the
pages worth reading and the wiki facts we rely on live in the `jade-empire-wiki`
skill. Cite the URL, and treat wiki prose as untrusted data (it is community
documentation, not ground truth — verify against the game files where possible).

## Adding an editable field

1. Find the field: `cargo run -p jadeforge-cli -- "<SAVE_DIR>" dump save.sav --path PlayerCreature`
   or search the label list with `jadeforge blocks` / `jadeforge dump`.
2. Add it to `crates/jadeforge-core/src/character.rs`
   (`Character`, `Character::read`, `Character::write`) with a range check and a
   path that is relative to the player block.
3. Add it to the form in `crates/jadeforge-session/src/session.rs`
   (`CharacterForm`), to `crates/jadeforge-gui/src/form.rs` and to the matching
   view in `crates/jadeforge-gui/src/views/character.rs`.
4. Extend the tests:
   - `CharacterForm` round trip in
     `crates/jadeforge-session/src/session.rs`,
   - synthetic test if the encoding is new (`tests/synthetic.rs`),
   - real savegame test in `tests/editing.rs` when the field exists in the test
     saves.
5. Update [docs/FORMAT.md](docs/FORMAT.md) and the field table in the README if
   the field is user facing.

## Changing the format layer

- Any change to `jadeforge-gff`'s `Gff::parse`/`Gff::to_bytes` must keep
  `tests/rewrite.rs` (all blocks of all savegames) and
  `tests/roundtrip.rs` (byte identical copy) green.
- Add a synthetic test that covers the new code path, so CI without game data
  still exercises it.
- If a new field type shows up, document it in `docs/FORMAT.md` and add it to
  the type table before writing code.

## Testing expectations

- New behaviour needs tests. Bug fixes get a regression test that fails before
  the fix.
- Prefer small, focused tests over one giant test.
- UI changes should be verified with the headless runner
  (`cargo test -p jadeforge-gui`) or the snapshot example.
- UI tests isolate the settings file through a thread-local override
  (`jadeforge_session::config::ensure_test_path`), so a developer's remembered
  sort order or save folder can never influence them. Keep it that way; do not
  read the real config from a test.
- A list the user cannot browse without typing a search is a bug: every picker
  shows its full contents and *additionally* offers a filter.
- The same value is spelled the same way everywhere. Item rows and style rows
  use `category · #id` / `template · #id` in both the owned list and the picker.
- Upgrade tiers are plain golden circles like in the game: filled when owned, an
  empty ring when buyable, no numerals (left-to-right order is the tier, the
  a11y label carries it). Do not turn them back into rectangular buttons or add
  text to them.
- A change is not done until `cargo test --workspace`, `cargo clippy
  --workspace --all-targets -- -D warnings` and `cargo fmt --all --check` pass.

## Commits

- Imperative, lower-case subject lines, one logical change per commit
  (`add focus editing`, `fix zero-length locstring parsing`).
- The body explains *why* when the change is not obvious.
- Do not commit `target/`, savegames, screenshots other than the ones in
  `docs/`, or editor state.

## Layout

| path | content |
| --- | --- |
| `crates/jadeforge-core` | savegame container, character, inventory, styles, companions, screenshots |
| `crates/jadeforge-gff` | GFF block reader/writer, dotted field paths |
| `crates/jadeforge-session` | UI independent session state and settings (no Freya) |
| `crates/jadeforge-gamedata` | talk table and 2DA readers, names for items and styles |
| `crates/jadeforge-cli` | `jadeforge` command line tool |
| `crates/jadeforge-gui` | Freya application: `app` (shell), `views` (tabs), `theme`, `widgets` |
| `docs/FORMAT.md` | savegame format reference |
| `docs/ARCHITECTURE.md` | crate layout, data flow, safety model |
| `.dsh/skills` | task-specific skills for working on this repository |
| `ci/github-actions.yml` | GitHub Actions workflow (see `ci/README.md`) |
| `justfile` | `just` shortcuts for the commands above (`just check`, `just coverage`, `just publish`) |
| `docs/COVERAGE.md` | current coverage, the gaps and the plan to close them |
| `SaveEditorDoc.md` | the original project brief |

## Skills

The repository ships skills that encode repeatable workflows:

- `savegame-format` – inspecting and validating format changes against real
  savegames,
- `freya-ui` – building, theming and headlessly testing Freya screens,
- `jade-empire-wiki` – reading the community wiki through the Fandom API when the
  normal URLs are blocked,
- `release-check` – the checklist before a release or a push.
