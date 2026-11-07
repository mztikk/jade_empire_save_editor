# Contributing

Read [AGENTS.md](AGENTS.md) first: it contains the non-negotiable safety rules
and the code style for this repository.

## Workflow

Work happens on task branches, never on `main`. Install the tracked hooks once
per clone and start every task from the latest `origin/main`:

```sh
just setup                  # once per clone; enables .githooks/pre-commit
just start fix/save-load    # <type>/<slug>; fetches origin/main
```

Branch names are `<type>/<slug>` in lower-case kebab-case, where `type` is
`feat`, `fix`, `docs`, `test`, `refactor`, `perf`, `build`, `ci`, `chore` or
`release`. `just start` refuses a dirty worktree and the `pre-commit` hook
refuses direct commits on `main`.

Before handing a branch over, run the quality gate:

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo nextest run --workspace
cargo test --workspace --doc
```

With the retail game available, also run the corpus suite:

```sh
JADEFORGE_SAVE_DIR="$HOME/games/Jade Empire/Save" \
JADEFORGE_GAME_DIR="$HOME/games/Jade Empire" \
JADEFORGE_CONFIG_DIR="$(mktemp -d)" cargo nextest run --workspace
```

Commit the work to the task branch and finish with a clean worktree:

```sh
git status --short          # must print nothing
just finish                 # fails on main, a dirty worktree or no commits
```

`just finish` must report that the branch is clean and ahead of `main`. Push
the branch and open a pull request when the change is ready; do not commit the
merge on `main` yourself. The UI suite builds a synthetic fixture when no
installation is configured, so it must run rather than skip. New format
knowledge belongs in [docs/FORMAT.md](docs/FORMAT.md); user-visible changes
belong in `README.md` and `CHANGELOG.md`.

`just live` is not part of this workflow: it opens real windows and needs a real
Wayland desktop. **Never run it without explicit permission** from the machine's
user, and only when the headless UI suite cannot cover the change or the bug is
in the real window. See [AGENTS.md](AGENTS.md) rule 13 and
[docs/LIVE-UI.md](docs/LIVE-UI.md).

## UI review work

The desktop UI review lives in [docs/UI-REVIEW.md](docs/UI-REVIEW.md); read it
before changing the shell, sidebar or a tab, and keep its status and screenshots
current. P0 ([#5](https://github.com/mztikk/jade_empire_save_editor/pull/5)),
P1 ([#7](https://github.com/mztikk/jade_empire_save_editor/issues/7)) and
P2 ([#8](https://github.com/mztikk/jade_empire_save_editor/issues/8)) are
complete, and the P3 work shipped in v0.3.0
([PR #32](https://github.com/mztikk/jade_empire_save_editor/pull/32)) except for
the blocked item-icon item
([#9](https://github.com/mztikk/jade_empire_save_editor/issues/9)). The
remaining open follow-ups are platform contrast
([#33](https://github.com/mztikk/jade_empire_save_editor/issues/33)) and the
upstream `Input` label association
([#3](https://github.com/mztikk/jade_empire_save_editor/issues/3)).

UI changes are verified with the headless suite and the snapshot example at
900, 1280 and 1600 px; the windowed suite is never part of the routine
workflow (see [AGENTS.md](AGENTS.md) rule 13).

## Pull requests

- Open the pull request from the task branch; `main` receives only merges.
- One logical change per branch and pull request; keep the diff reviewable and
  leave the worktree clean (`just finish`).
- Add a regression test for every bug fix and a synthetic test for every new
  parser or editing path.
- Do not commit savegames, game data, `target/`, `scratch/`, or generated
  coverage reports.
- Never commit secrets or sensitive data. Keep credentials in the environment
  or an untracked local file; use obvious placeholders in tests and examples.
- Do not use an em dash (U+2014) in code, comments, documentation, commit
  messages or UI strings.
- Do not weaken the workspace lints. A scoped `#[allow]` must explain in a
  comment why the lint is a false positive for that exact context; otherwise
  fix the code.
- Use the named GUI tokens in `crates/jadeforge-gui/src/theme.rs` instead of
  literal font sizes, spacing, padding, corner radii or fixed dimensions.
- Do not add comments that restate the code. Prefer a short `///` doc comment
  for new or refactored behaviour, and reserve inline comments for invariants
  and non-obvious constraints.
- Preserve untouched blocks byte for byte and never bypass
  `SaveGame::save_to`.
