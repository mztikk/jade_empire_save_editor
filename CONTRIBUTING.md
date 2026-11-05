# Contributing

Read [AGENTS.md](AGENTS.md) first: it contains the non-negotiable safety rules
and the code style for this repository.

## Workflow

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

The UI suite builds a synthetic fixture when no installation is configured, so
it must run rather than skip. New format knowledge belongs in
[docs/FORMAT.md](docs/FORMAT.md); user-visible changes belong in `README.md` and
`CHANGELOG.md`.

`just live` is not part of this workflow: it opens real windows and needs a real
Wayland desktop. **Never run it without explicit permission** from the machine's
user, and only when the headless UI suite cannot cover the change or the bug is
in the real window. See [AGENTS.md](AGENTS.md) rule 13 and
[docs/LIVE-UI.md](docs/LIVE-UI.md).

## Pull requests

- One logical change per pull request; keep the diff reviewable.
- Add a regression test for every bug fix and a synthetic test for every new
  parser or editing path.
- Do not commit savegames, game data, `target/`, `scratch/`, or generated
  coverage reports.
- Do not weaken the workspace lints. A scoped `#[allow]` needs a reason comment.
- Preserve untouched blocks byte for byte and never bypass
  `SaveGame::save_to`.
