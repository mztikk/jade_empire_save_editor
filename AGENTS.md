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

# Tests against real savegames (skipped when the folder is missing)
JADEFORGE_SAVE_DIR="/media/Games/Heroic/Jade Empire/Save" cargo test --workspace

# Render the UI into a PNG without a window (screenshots, layout debugging)
cargo run -p jadeforge-gui --example snapshot -- "<SAVE_DIR>" docs/screenshot.png
```

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

## Code style

- `cargo fmt` is authoritative.
- Clippy runs with `-D warnings` and the workspace enables `clippy::pedantic`.
  If a lint genuinely does not fit, add a scoped `#[allow(...)]` **with a
  reason comment**; do not weaken the workspace lint configuration.
- Public items have doc comments that explain behaviour, not syntax.
- Errors carry context: use `Error::Corrupt`/`Error::Gff` with the path, block
  or field that failed, not bare strings.
- No panics on user data. `expect`/`unwrap` are only allowed where a preceding
  check proves the invariant, and the message must state why.
- Keep the core free of UI dependencies; keep the GUI free of format knowledge.

## Adding an editable field

1. Find the field: `cargo run -p jadeforge-cli -- "<SAVE_DIR>" dump save.sav --path PlayerCreature`
   or search the label list with `jadeforge blocks` / `jadeforge dump`.
2. Add it to `crates/jadeforge-core/src/character.rs`
   (`Character`, `Character::read`, `Character::write`) with a range check and a
   path that is relative to the player block.
3. Add it to the form in `crates/jadeforge-gui/src/session.rs` (`CharacterForm`)
   and to the UI in `crates/jadeforge-gui/src/app.rs`.
4. Extend the tests:
   - `CharacterForm` round trip in `session.rs`,
   - synthetic test if the encoding is new (`tests/synthetic.rs`),
   - real savegame test in `tests/editing.rs` when the field exists in the test
     saves.
5. Update [docs/FORMAT.md](docs/FORMAT.md) and the field table in the README if
   the field is user facing.

## Changing the format layer

- Any change to `gff::parse`/`gff::to_bytes` must keep
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
- A change is not done until `cargo test --workspace`, `cargo clippy
  --workspace --all-targets -- -D warnings` and `cargo fmt --all --check` pass.

## Commits

- Imperative, lower-case subject lines, one logical change per commit
  (`add focus editing`, `fix zero-length locstring parsing`).
- The body explains *why* when the change is not obvious.
- Do not commit `target/`, savegames, screenshots other than
  `docs/screenshot.png`, or editor state.

## Layout

| path | content |
| --- | --- |
| `crates/jadeforge-core` | format reader/writer, character and inventory model |
| `crates/jadeforge-cli` | `jadeforge` command line tool |
| `crates/jadeforge-gui` | Freya application, `session` (logic) and `app` (view) |
| `docs/FORMAT.md` | savegame format reference |
| `docs/ARCHITECTURE.md` | crate layout, data flow, safety model |
| `.dsh/skills` | task-specific skills for working on this repository |
| `ci/github-actions.yml` | GitHub Actions workflow (see `ci/README.md`) |
| `SaveEditorDoc.md` | the original project brief |

## Skills

The repository ships skills that encode repeatable workflows:

- `savegame-format` – inspecting and validating format changes against real
  savegames,
- `freya-ui` – building and headlessly testing Freya screens,
- `release-check` – the checklist before a release or a push.
