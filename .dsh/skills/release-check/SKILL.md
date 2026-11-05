---
name: release-check
description: Use before committing, pushing, tagging or releasing JadeForge; runs the quality gate plus the manual checks that CI cannot perform.
---

# Before a commit, push or release

## Quality gate

Run all three, in this order, from the repository root. They must be free of
warnings and failures.

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo nextest run --workspace
```

With real savegames available, run the full suite as well. It is the only test
that proves the format handling still works against retail data:

```sh
JADEFORGE_SAVE_DIR="/path/to/Jade Empire/Save" \
JADEFORGE_GAME_DIR="/path/to/Jade Empire" \
JADEFORGE_CONFIG_DIR="$(mktemp -d)" cargo nextest run --workspace
```

Expect nextest's `Summary ... 122 tests run: 122 passed, 0 skipped` (the number
grows over time; skips are fine when the game or savegames are absent). Nextest
captures stdout, so the `round tripped 89 savegames` / `rewrote 962 blocks` lines
are only printed with `--no-capture` or on failure. The UI suite is the 24
headless tests of the `jadeforge-gui` crate. A different count is fine on another machine, "skipping" is
not (it means the variables did not reach the tests).

## Manual checks

1. `cargo build --release --workspace` succeeds.
2. `cargo run -p jadeforge-gui --example snapshot -- "<SAVE_DIR>" /tmp/ui.png all`
   renders every tab, not the empty state, and the images look sane. Regenerate
   `docs/screenshot*.png` when the layout changed.
3. The CLI round trip on a *copy* of a savegame leaves the file byte identical
   when nothing is changed:

   ```sh
   cp "<SAVE_DIR>/<save>/save.sac" /tmp/before.sac
   cp /tmp/before.sac /tmp/after.sac
   cargo run -q -p jadeforge-cli -- /tmp/after.sac verify
   cmp /tmp/before.sac /tmp/after.sac
   ```

4. `git status` shows no `target/`, `scratch/`, savegames or
   stray screenshots. `docs/screenshot.png` may be committed when it changed.

## Review the diff

- Read `git diff --stat` and then the diff itself. No debug prints, no
  commented-out code, no TODOs without an issue.
- Public API changes have doc comments.
- New behaviour has tests; bug fixes have a regression test.
- `docs/FORMAT.md` is updated when format knowledge changed, `README.md` when
  user visible behaviour changed, `AGENTS.md` when the workflow changed.

## Release

- Bump `version` in the workspace `Cargo.toml` only; the crates inherit it.
- Update the changelog section in the release notes (or `CHANGELOG.md` when one
  exists).
- Tag with `vMAJOR.MINOR.PATCH` and push tags: `git push origin main --tags`.
- Build artifacts are produced by `.github/workflows/ci.yml`; attach the
  binaries of all three platforms to the release.
- Never ship a build whose real-savegame tests were skipped.
