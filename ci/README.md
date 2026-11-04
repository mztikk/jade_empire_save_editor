# CI definition

The GitHub Actions workflow lives in [`github-actions.yml`](github-actions.yml)
instead of `.github/workflows/ci.yml` because the token that pushed this
repository was not allowed to write workflow files.

To activate it:

```sh
mkdir -p .github/workflows
git mv ci/github-actions.yml .github/workflows/ci.yml
git commit -m "ci: activate the GitHub Actions workflow"
git push
```

Pushing a file into `.github/workflows/` requires a GitHub token with the
`workflow` scope (`gh auth refresh -s workflow`) or the "Workflows" permission
of a fine-grained token.

What the workflow does:

- `formatting`: `cargo fmt --all --check` on Linux.
- `core and cli`: tests and `clippy -D warnings` for `jadeforge-core` and
  `jadeforge-cli` on Linux, Windows and macOS (no GUI system libraries needed).
- `gui`: installs the X11/Wayland/EGL development packages, runs the headless
  `freya-testing` UI tests, runs clippy and builds/uploads the release binaries.

Tests that need real savegames skip themselves unless `JADEFORGE_SAVE_DIR` is
set, so the workflow is green on hosted runners without the game.

## Strict clippy report

`ci/clippy-strict.sh` runs the lints that are not part of the gate yet
(`expect_used`, `indexing_slicing`, `arithmetic_side_effects`, `nursery`) and
prints the diagnostics per lint. It always exits 0 and is meant to be run
manually (or as a non-blocking CI job):

```sh
ci/clippy-strict.sh
```

When a lint reaches zero findings, promote it to `deny` in the workspace lints
and it becomes part of the normal gate. The binary parsers intentionally index
slices after validating offsets, so `indexing_slicing` and
`arithmetic_side_effects` will need module-level allows rather than a blanket
fix.
