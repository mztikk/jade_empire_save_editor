---
name: release-check
description: Use before committing, pushing, tagging or releasing JadeForge; runs the quality gate plus the manual checks that CI cannot perform.
---

# Before a commit, push or release

Run this on the task branch that holds the change, never on `main`. The
worktree must be clean and `just finish` must pass before a branch is pushed or
tagged; see the `git-workflow` skill for the branch lifecycle.

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

Expect nextest's `Summary ... 286 tests run: 286 passed, 0 skipped` with a real
corpus (the number grows over time; skips are fine when the game or savegames
are absent). Nextest captures stdout, so the `round tripped 89 savegames` /
`rewrote 962 blocks` lines are only printed with `--no-capture` or on failure.
The UI suite is the 29 headless tests of the `jadeforge-gui` crate and builds a
synthetic fixture when no installation is set, so it must not report "skipping"
in CI. A different count is fine on another machine, "skipping" is not (it means
the variables did not reach the tests).

## Manual checks

1. `cargo build --release --workspace` succeeds; `cargo check --locked -p ...`
   also succeeds with the declared MSRV (1.99 in `Cargo.toml` and CI).
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

4. Classify the window-path risk. A major Freya/winit/window-stack upgrade, a
   window configuration/close/focus/input/presentation change, a live-probe
   change, or a bug that only reproduces in a real window needs explicit
   permission to run `JADEFORGE_LIVE_APPROVED=1 just live`. Ask before `just finish`, wait for the answer,
   and record the result. View, layout, text, theme, form, session and core
   changes covered by the headless suite and snapshots do not. Compiling the
   live target is not a substitute for running it; a window-path change that
   was not run is not verified.
5. `git status` shows no `target/`, `scratch/`, savegames or
   stray screenshots. `docs/screenshot.png` may be committed when it changed.
   `just finish` must report a clean task branch with commits ahead of
   `origin/main`.

## Review the diff

- Read `git diff --stat` and then the diff itself. No debug prints, no
  commented-out code, no TODOs without an issue.
- Public API changes have doc comments.
- New behaviour has tests; bug fixes have a regression test.
- `docs/FORMAT.md` is updated when format knowledge changed, `README.md` when
  user visible behaviour changed, `AGENTS.md` when the workflow changed.

## Release

- Do the release work on a `release/vX.Y.Z` branch, not on `main`.
- Bump `version` in the workspace `Cargo.toml` only; the crates inherit it.
- Update the changelog section in the release notes (or `CHANGELOG.md` when one
  exists).
- Merge the release branch through a pull request, then tag the merge commit on
  `origin/main` without checking it out:
  `git fetch origin && git tag -a vX.Y.Z origin/main -m "JadeForge vX.Y.Z" &&
  git push origin vX.Y.Z`. Tags do not require a checkout of `main`.
- `.github/workflows/ci.yml` produces the binaries, `SHA256SUMS`,
  `THIRD_PARTY_NOTICES.txt`, the dependency inventory, `sbom.cdx.json` and
  the Skia licence;
  attach all of them to the release.
- When GitHub Actions is unavailable, follow
  [docs/RELEASING.md](../../../docs/RELEASING.md) for the local Linux and
  Windows build, assembly, publishing and verification path.
- In a workspace-only sandbox, only `just publish-linux` (podman runtime
  directory and rootless user namespace) and `just publish-windows-cross`
  (cargo-xwin cache) need a one-shot full-access approval. Run both recipes
  verbatim, one approval each, after the version bump; keep everything else
  sandboxed. Do not probe or work around the runtime. If approval is refused,
  stop and ask. Keep persistent logs in `scratch/`, not `/tmp`.
- Sign/notarise the macOS and Windows binaries when the platform credentials
  are available (see `ci/README.md`); otherwise mark the release unsigned.
- Never ship a build whose real-savegame tests were skipped.
