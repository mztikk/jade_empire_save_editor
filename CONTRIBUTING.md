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

## Pull requests

- One logical change per pull request; keep the diff reviewable.
- Add a regression test for every bug fix and a synthetic test for every new
  parser or editing path.
- Do not commit savegames, game data, `target/`, `scratch/`, or generated
  coverage reports.
- Do not weaken the workspace lints. A scoped `#[allow]` needs a reason comment.
- Preserve untouched blocks byte for byte and never bypass
  `SaveGame::save_to`.
