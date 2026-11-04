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
- `gff, core, session and cli`: tests and `clippy -D warnings` for
  `jadeforge-gff`, `jadeforge-core`, `jadeforge-session` and `jadeforge-cli` on
  Linux, Windows and macOS (no GUI system libraries needed).
- `gui`: installs the X11/Wayland/EGL development packages, runs the headless
  `freya-testing` UI tests and clippy on Linux.
- `publish`: runs `just publish` on Linux, Windows and macOS, runs the headless
  UI tests on Windows and macOS, and uploads the fat-LTO binaries from `dist/`
  as one artifact per OS. Pull requests skip it; push to `main`, tag a release
  or start the workflow manually.

The Windows publish job sets `RUSTFLAGS=-C link-arg=/FORCE:MULTIPLE` because
Skia's vendored ICU and the system `icuuc.dll` that `accesskit_windows` pulls in
through the `windows` crate define the same symbols. Remove the flag once that
conflict is fixed upstream; the artifacts still need the release checklist
before they are shipped.

Tests that need real savegames skip themselves unless `JADEFORGE_SAVE_DIR` is
set, so the workflow is green on hosted runners without the game.

## Strict clippy report

`just strict` runs the lints that are not part of the gate yet
(`expect_used`, `indexing_slicing`, `arithmetic_side_effects`, `nursery`) and
prints the diagnostics per lint. It is meant to be run manually (or as a
non-blocking CI job):

```sh
just strict
```

When a lint reaches zero findings, promote it to `deny` in the workspace lints
and it becomes part of the normal gate. The binary parsers intentionally index
slices after validating offsets, so `indexing_slicing` and
`arithmetic_side_effects` will need module-level allows rather than a blanket
fix.
