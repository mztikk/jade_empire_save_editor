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
cargo build --workspace                       # build everything (MSRV 1.99)
cargo nextest run --workspace                 # all tests (cargo-nextest)
cargo test --workspace --doc                  # doctests
cargo clippy --workspace --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc --locked --workspace --no-deps
cargo fmt --all --check

# Task branches: install the hooks once, then start every task on a branch
just setup
just start fix/<slug>
just finish                                   # clean tree + commits ahead of main

# Install the GUI and CLI locally, tuned for this machine's CPU (not portable)
just install

# Tests against real savegames (skipped when the variable is not set)
JADEFORGE_SAVE_DIR="$HOME/games/Jade Empire/Save" \
JADEFORGE_GAME_DIR="$HOME/games/Jade Empire" \
JADEFORGE_CONFIG_DIR="$(mktemp -d)" cargo nextest run --workspace

# Render the UI into a PNG without a window (screenshots, layout debugging)
cargo run -p jadeforge-gui --example snapshot -- "<SAVE_DIR>" docs/screenshot.png [TAB|all]
# Add `--height 1800 --query viper` for tall layouts and filter results;
# `--query` targets the active tab's own picker filter (Items, Styles or Gems),
# never the sidebar save filter. `--width 900` checks narrow windows.
```

The same commands are available through `just` (see the `justfile`): `just
check` runs formatting, clippy, rustdoc and the tests, `just setup` installs the tracked
git hooks once per clone, `just start <type>/<slug>` creates a task branch from
the latest `origin/main`, `just finish` is the clean-hand-over guard, `just live`
runs the opt-in windowed UI suite against the real Wayland desktop (approval-
gated and only with explicit permission; see rule 13), `just snapshot …` renders the UI, and `just
publish` builds the fat-LTO distribution binaries, stages checksums and
third-party notices, and `just checksums` regenerates `dist/SHA256SUMS`.
`just publish-linux` builds the SteamOS-compatible Linux binaries (glibc 2.35) in
a rootless Podman container. Linux/Windows x86-64 releases ship both the
`x86-64-v3` binaries (`jadeforge`, `jadeforge-cli`) and baseline fallbacks
(`jadeforge-generic-cpu`, `jadeforge-cli-generic-cpu`) in the same folder. `just coverage` runs
`cargo llvm-cov nextest` and writes `coverage/lcov.info` and `coverage/html` (see
[docs/COVERAGE.md](docs/COVERAGE.md) for the current numbers and the plan). The
cargo commands above remain canonical.

## Branch workflow

`main` is integration only: never edit files or make ordinary commits on it.
Every task, including a one-line fix or a documentation change, runs on its own
branch cut from the latest `origin/main`:

```sh
just setup                  # once per clone: enable the tracked .githooks
just start fix/save-load    # fetch origin/main, create and switch the branch
# ... work and commit logical steps with imperative messages ...
just check                  # the quality gate (fmt, clippy, rustdoc, nextest, doctests)
just finish                 # fails on main, a dirty tree or no commits ahead
```

Branch names are `<type>/<slug>` in lower-case kebab-case. `type` is one of
`feat`, `fix`, `docs`, `test`, `refactor`, `perf`, `build`, `ci`, `chore` or
`release`; `slug` describes the change (`fix/zero-length-locstrings`).

`just finish` is the hand-over condition: it passes only when the branch is not
`main`, `git status --porcelain` is empty and the branch has commits that
`origin/main` does not. The branch is left checked out for review; agents must
not push it or open a pull request unless the user asks. `main` only ever
advances by merging task branches (a pull request is preferred). When a branch
is merged, delete it. The `git-workflow` skill carries the step-by-step
procedure; the tracked `pre-commit` hook refuses direct commits on `main`.

## Editing files

- Read a file before changing it, then make a targeted, literal edit rather than
  rewriting it. Agent tooling mirrors this: `read` to inspect, `edit` to change,
  and `write` only for a new file or a deliberate full rewrite. A targeted edit
  fails when the context it expects is gone; a blind rewrite can silently drop
  work.
- Do not hand-rewrite tracked files through shell one-liners (`sed -i`, shell
  redirection, `perl` or Python). They hide the change from review and can
  over-match. Reserve shell-based editing for genuinely bulk or mechanical
  transformations, such as the same substitution across many files. Even then,
  keep the substitution exact, review `git diff`, and run the formatters and
  tests.

## Shell tooling

- When a shell command is the right tool, prefer the modern CLI over its classic
  predecessor: `rg` (not `grep`), `fd` (not `find`), `bat` (not `cat`/`head`),
  `eza` (not `ls`), `sd` (not `sed -i`), `jq`/`yq` for JSON/YAML, `delta` for
  diffs on a terminal, `dust` (not `du`), `hyperfine` for timings and `tokei`
  for line counts. The harness `grep`, `glob` and `read` tools stay the default
  for structured inspection; the shell replacements are for pipelines,
  aggregations and bulk work those tools do not express.
- Availability is per host, not a build requirement. Use the modern tool when
  it is on `PATH` and fall back to the POSIX equivalent when it is not. Never
  install or bootstrap one (rule 12).
- `sd` is a shell editor: the "Editing files" rule still applies, so use it only
  for genuinely bulk, exact substitutions and review `git diff` afterwards.
- Leave the global git configuration alone. Pipe `git diff`/`git show` through
  `delta` instead of setting `core.pager`.

## Scratch files

- Keep anything that must outlive a single command in `scratch/` at the
  repository root, including files you want to inspect or show. It is
  gitignored (`/scratch/`), so it cannot dirty the tree or block `just finish`.
- Do not put that work in `/tmp`. Sandboxed agent runtimes commonly mount a
  private, per-call tmpfs there: the write reports success and the file is gone
  by the next command, which silently breaks any multi-step task. `/tmp` is
  only safe when the whole lifetime of the file fits inside one command, as
  with `JADEFORGE_CONFIG_DIR="$(mktemp -d)" cargo nextest run --workspace`.
- Do not create a `./tmp` directory. It is not ignored, so its contents show up
  as untracked files and rule 15 fails the hand-over.
- Scratch content is never a deliverable: do not commit it, and delete what you
  no longer need rather than leaving it for the next person to wonder about.

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
7. **No game data files or savegames in the repository.** Never commit
   `dialog.tlk`, 2DA/RIM files, game assets or test savegames; tests that need
   them look for `JADEFORGE_SAVE_DIR` and skip otherwise. The single exception
   is the generated English name index at
   `crates/jadeforge-gamedata/data/names-en.toml`: item and style ids, their
   English names and the item category only. It must be produced by
   `jadeforge-cli export-names`, never copied wholesale, and must not grow
   descriptions, table dumps, upgrade data or other extracted text.
8. **Never hardcode a machine path.** No default game or save directory may be
   baked into the source: users are asked once (the native folder dialog), the choice is
   stored in the config file, and `JADEFORGE_SAVE_DIR`/`JADEFORGE_GAME_DIR`
   override it. Tests use the variables only and skip when they are unset.
9. **Treat the game installation as read only.** `jadeforge-gamedata` only
   reads `dialog.tlk` and the 2DA tables; it must never open a file for writing
   and the editor must keep working when those files are absent.
10. **Never take the cheapest path.** Implement the full request, including
    tests, documentation and follow-through, instead of the smallest change
    that merely appears to work. Do not silently drop scope, park a requested
    item, leave TODOs or placeholders, or defer part of the work because of
    assumed time or context limits. If a complete implementation is genuinely
    impractical, say so explicitly and ask before reducing scope. A shortcut is
    only acceptable when the request itself asks for one.
11. **Research before assuming.** For unfamiliar APIs, dependency choices,
    version-sensitive behaviour and external failure modes, consult the
    primary sources before implementing: docs.rs, crates.io, the upstream
    GitHub repository (release notes and issues), and the exact source of the
    pinned version under `~/.cargo/registry`. Local source tells you what
    compiles; docs and issues tell you the intended pattern and known
    pitfalls. Do not rely on memory for a library API, and do not pick a
    dependency merely because it is familiar. Cite the URLs you used in the
    answer. Searching and reading documentation is part of the work, not
    optional overhead.
12. **Use the host toolchain; never install a substitute.** The host is
    expected to already provide every required Rust toolchain, component,
    system package and binary. Do not run `rustup toolchain install`,
    `rustup component add`, `cargo install`, a system package manager, or create
    temporary `CARGO_HOME`/`RUSTUP_HOME` directories, portable toolchains or
    other workarounds to supply something that is missing. If a required tool
    is absent or a command fails because of the environment, stop and ask the
    user to install or provide it. Normal Cargo dependency resolution into the
    host's cache is fine; bootstrapping a tool that should already exist is not.
13. **Never run the windowed UI suite without explicit permission.**
    `JADEFORGE_LIVE_APPROVED=1 just live`,
    `cargo test -p jadeforge-gui --test live` and anything with
    `JADEFORGE_LIVE=1` open real windows, take focus, move the real pointer and
    type on the user's desktop. Plain `just live` and a run without
    `JADEFORGE_LIVE_APPROVED=1` are refused by both the recipe and the test
    binary; that gate is not permission. Ask the user first, explain why the
    headless suite cannot cover the case, and wait for their answer; a general
    request to run the tests is not permission. Only run it when a headless GUI test
    genuinely cannot reproduce or verify the behaviour (window creation,
    compositor input, focus, presentation, HiDPI or window-manager interaction),
    or when the bug is in the actual windowed GUI or a mismatch between the
    headless and real window. Otherwise use the headless suite and snapshots.
    Do not use this permission rule as a reason to skip the question: for a
    major Freya/winit upgrade or another window-path change, asking for
    permission is part of the hand-over. Compile-only coverage is not
    verification. For minor view, layout, text or theme changes covered by the
    headless suite, use the headless suite and do not ask. The same rule applies
    to scripts and CI: the suite is not a CI target.
14. **Never work on `main`.** Every task, including a one-line fix or a
    documentation change, starts on its own branch from `origin/main` with
    `just start <type>/<slug>`; `main` only receives merged task branches. The
    tracked `pre-commit` hook refuses direct commits on `main`.
15. **Finish with a clean worktree.** A task is not finished until its work is
    committed to the task branch, the quality gate passes and `just finish`
    succeeds. Never leave uncommitted or untracked files behind after a task,
    and never discard someone else's changes to get clean.
16. **Never hardcode secrets or sensitive data.** API keys, tokens, passwords,
    private keys and credentials belong in the environment or an untracked local
    file, never in source, tests, fixtures, examples, docs or commit messages.
    CI reads them from repository secrets. A test that needs a credential-shaped
    value uses an obvious placeholder, never a real-looking secret.
17. **Never interpolate prose into a double-quoted shell argument.** Backticks
    and `$()` are command substitution, not text. Pass multi-line prose or
    Markdown through a file (`gh pr create --body-file`, a heredoc) or single
    quotes. Never put backticked commands in a double-quoted argument.

## Code style

- `cargo fmt` is authoritative.
- Clippy runs with `-D warnings` and the workspace enables `clippy::pedantic`.
  Do not use `#[allow(...)]` to suppress a compiler or Clippy diagnostic unless
  the comment on the attribute explains exactly why the lint is a false
  positive for that specific context. If the lint is right, fix the code
  instead; do not weaken the workspace lint configuration.
- The workspace also **denies** `unwrap_used`, `expect_used`, `panic`, `todo`,
  `unimplemented`, `unreachable`, `exit`, `string_slice` and
  `panic_in_result_fn`, and denies `missing_docs`. Production code therefore
  propagates errors; `expect` is not an escape hatch. The `clippy.toml` test
  allowance covers `#[test]` functions and `cfg(test)` modules, and integration
  tests plus `examples/snapshot.rs` carry a file-level `#![allow]` because
  panicking is their assertion mechanism. Do not add a production `#[allow]` for
  these lints without a documented invariant.
- `just strict` reports the lints that are not gated yet
  (`expect_used`, `indexing_slicing`, `arithmetic_side_effects`, `nursery`) as
  an audit that never gates the build. Run it before promoting a lint: only add
  a lint to the workspace config once its count is zero, so the `-D warnings`
  gate stays meaningful.
- Do not add comments that restate the code. When adding or refactoring a
  feature, prefer a short `///` doc comment that explains the behaviour and why
  it exists. Reserve inline `//` comments for invariants, upstream workarounds
  and non-obvious safety constraints.
- Public items have doc comments that explain behaviour, not syntax.
- GUI code uses the shared tokens in `crates/jadeforge-gui/src/theme.rs` for
  font sizes and numeric layout values: `theme::font::BODY`,
  `theme::space::MD`, `theme::radius::CARD`, `theme::stroke::FOCUS`. Do not pass
  a literal font size, gap, padding, corner radius or fixed component dimension
  to a builder. If the scale lacks a value, add a token there. A dimension that
  is an invariant of one module may stay a documented `const` in that module,
  but it must have a name and the module that owns the invariant.
- Errors carry context: use `Error::Corrupt`/`Error::Gff` with the path, block
  or field that failed, not bare strings; the GFF crate uses its own
  `Error::Malformed` for the same reason.
- No panics on user data. `unwrap` and `expect` are denied in production code;
  propagate errors with context instead.
- Keep the core free of UI dependencies; keep the GUI free of format knowledge.
- JSON is reserved for machine-readable output and machine-to-machine
  communication: the CLI's `dump --json` export (`jadeforge-core::json`). Do not
  use JSON for configuration, persisted settings or internal data flow; the
  settings file is TOML and savegames stay binary.
- Do not use an em dash (U+2014) in code, comments, documentation, commit
  messages or user-facing strings. Use a colon, comma, semicolon or parentheses
  instead. This rule is part of the Freya upstream conventions as well.

## Game data (names, item and style tables)

- `crates/jadeforge-gamedata` locates the installation from the save folder
  (`JADEFORGE_GAME_DIR` wins; a broken override falls back to the normal
  `<game>/Save` layout), parses `dialog.tlk` (TLK V4.0) and the text 2DA tables
  in `override/`.
- Every lookup degrades gracefully: when the files are missing the UI falls back
  to the bundled English name index and then to numeric ids; it never fails to
  open a savegame because of missing game data. An installed game always wins
  over the bundled names.
- Names come from the talk table via the 2DA `STRREF_NAME`/`NameRef` columns. The
  generated `data/names-en.toml` is the only fallback copy; do not add another
  hand-written name list.
- `data/names-en.toml` contains only item ids/names/categories and style
  ids/names. Do not extend it with descriptions, upgrade rows, table dumps or
  any other extracted text; regenerate it with
  `jadeforge-cli export-names` instead.
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
  `set_style_family_slot` writes the mask and the level together and validates
  the family slots; never write one without the other. Do not go back to
  "bit = advance row - 1": it contradicts real saves.
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
page. That is a bot check, **not** a missing page: never conclude that a page
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
documentation, not ground truth: verify against the game files where possible).

## Adding an editable field

1. Find the field: `cargo run -p jadeforge-cli -- "<SAVE_DIR>" dump save.sav --path PlayerCreature`
   or search the label list with `jadeforge-cli blocks` / `jadeforge-cli dump`.
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
  (`cargo nextest run -p jadeforge-gui`) and the snapshot example at 900,
  1280 and 1600 px. Any new list row with trailing controls needs a geometry
  test (`assert_compact_action_rows`): the controls must stay inside the card
  and the row pitch must stay compact. Every tab must render at both narrow and
  wide widths.
- Windowed coverage is a risk-based escalation, not a routine step. Compiling
  with `--all-targets` proves the live support code builds; it does not prove
  the window path works. For a major Freya/winit/window-stack upgrade, a window
  creation/configuration/close/focus/input/presentation change, a live-probe
  change, or a bug that only reproduces in a real window, ask for explicit
  permission to run `JADEFORGE_LIVE_APPROVED=1 just live` before `just finish`
  and wait for the answer.
  View, layout, text, theme, form, session and core changes covered by the
  headless suite and snapshots do not need it. If unsure, ask. If a window-path
  change cannot be run (no permission or no display), say so explicitly in the
  hand-over; do not present compile-only coverage as verification. Keep its
  scenarios small and window-level, because exhaustive layout coverage belongs
  in the headless suite. See `docs/LIVE-UI.md` and rule 13.
- Every dynamic UI list item and every conditional tab root must have a stable
  key. Freya 0.5 fixed several diff panics, including the unkeyed replacement
  crash that 0.4 hit, but keyed identity is still what reconciles reordered rows
  without duplicating or dropping element state. Keys are a correctness and
  crash guard, not optional. Use the existing `KeyExt` implementations and call
  `.key(...)` at list sites.
- UI tests isolate the settings file through a thread-local override
  (`jadeforge_session::config::ensure_test_path`), so a developer's remembered
  sort order or save folder can never influence them. Keep it that way; do not
  read the real config from a test.
- The UI fixture uses `JADEFORGE_SAVE_DIR` (the same `<game>/Save` walk-up the
  app uses) when it is set and hands it to the session through a thread-local
  test root. Without the variables it builds a synthetic savegame and game
  installation, so the headless suite runs in CI instead of skipping.
- A list the user cannot browse without typing a search is a bug: every picker
  shows its full contents and *additionally* offers a filter.
- The same value is spelled the same way everywhere. Item rows and style rows
  use `category · #id` / `template · #id` in both the owned list and the picker.
- Upgrade tiers are plain golden circles like in the game: filled when owned, an
  empty ring when buyable, no numerals (left-to-right order is the tier, the
  a11y label carries it). Do not turn them back into rectangular buttons or add
  text to them.
- A change is not done until `just check` (formatting, clippy, rustdoc, nextest
  and doctests) passes and `just finish` reports a clean task branch.

## Commits

- Commit on the task branch, never on `main`; the tracked hook refuses it.
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
| `crates/jadeforge-i18n` | stable-id English/German interface catalogs, locale selection and strictness (no Freya) |
| `crates/jadeforge-session` | UI independent session state, form/model modules and settings (no Freya) |
| `crates/jadeforge-gamedata` | talk table and 2DA readers, names for items and styles |
| `crates/jadeforge-cli` | `jadeforge-cli` command line tool |
| `crates/jadeforge-gui` | Freya application: `app/` (shell, sidebar, navigation), `views` (tabs), `theme`, `widgets` |
| `docs/FORMAT.md` | savegame format reference |
| `docs/ARCHITECTURE.md` | crate layout, data flow, safety model |
| `docs/NAMES.md` | bundled English names: scope, generation and precedence |
| `docs/LIVE-UI.md` | windowed UI suite: backends, process model, desktop/VM requirements and how to add scenarios |
| `.dsh/skills` | task-specific skills for working on this repository |
| `.githooks` | tracked git hooks; `just setup` enables them (`pre-commit` refuses direct commits on `main`) |
| `.github/workflows/ci.yml` | GitHub Actions workflow (see `ci/README.md`) |
| `justfile` | `just` shortcuts for the commands above (`just setup`, `just start`, `just check`, `just finish`, `just live`, `just coverage`, `just publish`) |
| `docs/COVERAGE.md` | current coverage, the gaps and the plan to close them |
| `SaveEditorDoc.md` | the original project brief |

## Skills

The repository ships skills that encode repeatable workflows:

- `savegame-format` – inspecting and validating format changes against real
  savegames,
- `freya` – upstream Freya framework best practices for components, hooks,
  elements, theming and state,
- `freya-ui` – building, theming, headlessly testing and driving the real
  windowed Freya screens,
- `jade-empire-wiki` – reading the community wiki through the Fandom API when the
  normal URLs are blocked,
- `git-workflow` – starting, committing and finishing a task branch with a clean
  worktree,
- `release-check` – the checklist before a release or a push.
