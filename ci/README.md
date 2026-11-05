# CI and release engineering

The GitHub Actions workflow lives in
[`.github/workflows/ci.yml`](../.github/workflows/ci.yml). It is active for
pushes to `main`, pull requests, tags and manual dispatches.

## Jobs

- `formatting`: `cargo fmt --all --check` and `ci/test-git-workflow.sh`, which
  proves the task-branch guards in throwaway repositories: the pre-commit hook
  refuses a direct commit on `main`, and `start`/`finish` reject a dirty or empty
  branch.
- `test`: tests, doctests, clippy and rustdoc for `jadeforge-gff`,
  `jadeforge-core`, `jadeforge-session`, `jadeforge-gamedata` and
  `jadeforge-cli` on Linux, Windows and macOS. This includes the game-data
  tests that the previous workflow missed. On Windows it also checks the GUI so
  the `winresource` build script really embeds the `.exe` icon, and on macOS it
  checks that the GUI still compiles even though macOS is not released yet.
- `msrv`: `cargo check` with the declared MSRV (currently 1.88) on Linux.
- `gui`: installs the X11/Wayland/EGL development packages, runs the headless
  `freya-testing` UI suite, clippy and the release icon staging smoke test. The
  UI suite builds a synthetic savegame and game installation when
  `JADEFORGE_SAVE_DIR`/`JADEFORGE_GAME_DIR` are not set, so it is not a no-op on
  CI. The windowed UI suite is not part of CI: it opens real windows and needs a
  real Wayland desktop, so it is only run locally or in a desktop VM with
  explicit permission (see [docs/LIVE-UI.md](../docs/LIVE-UI.md)).
- `audit`: `cargo audit` against the lockfile.
- `publish` (non-PR only): builds the fat-LTO binaries with `just publish` on
  Linux and Windows, stages the Windows icon and the Linux `share/` tree,
  checks the glibc baseline on Linux, writes `THIRD_PARTY_NOTICES.txt`, the
  dependency inventory, a CycloneDX `sbom.cdx.json` and, for public
  repositories, a GitHub build-provenance attestation, and uploads the staged
  folders as per-OS artifacts. macOS is not in the release matrix yet and the published binaries
  are unsigned; see [Release signing](#release-signing). Windows also runs the
  headless UI tests.
- `release` (tags only): downloads the per-OS artifacts, zips every staged
  folder (the Linux `share/` tree travels inside the Linux archive), writes one
  `SHA256SUMS` over the uploaded assets and `RELEASE_NOTES.md` from the
  matching changelog section, creates a draft GitHub release and publishes it
  once the assets are attached. Re-running a failed tag run re-uploads with
  `--clobber`.

All actions are pinned to commit SHAs. The workflow has a read-only default
token; only the release job may write release contents.

## Local reproduction

```sh
just setup           # enable the tracked git hooks (once per clone)
just start fix/x     # create a task branch from origin/main
just finish          # fail unless the branch is clean and ahead of origin/main
just check           # fmt, clippy, nextest, doctests
just strict         # non-gating audit of the strict lints
just coverage       # cargo llvm-cov summary, lcov and html
just publish        # fat-LTO binaries, macOS .app/Linux share, notices under dist/
ci/assemble-release.sh dist vX.Y.Z  # zip the folders, write SHA256SUMS and notes
just checksums      # dist/SHA256SUMS for the staged (unzipped) files
just sbom           # dist/sbom.cdx.json
just icons          # regenerate the icon PNG/ICO/ICNS assets from icon.svg
just install-desktop # Linux GUI + launcher entry + hicolor icons
ci/test-icon-staging.sh # smoke-test the macOS bundle and Linux share staging
cargo bench -p jadeforge-core --bench save_corpus
cargo +nightly fuzz run gff   # long local run; see fuzz/README.md
```

`just check` is the canonical gate. With a real installation, add
`JADEFORGE_SAVE_DIR`/`JADEFORGE_GAME_DIR` to run the corpus tests; without them
the UI tests use the synthetic fixture. Work happens on task branches; a branch
is ready for review when `just finish` passes, and pull requests run CI before a
merge.

## Release signing

The publish job does not sign or notarise anything yet: releases are unsigned
and must say so in their notes. Windows shows an "unknown publisher" SmartScreen
warning until the binaries are signed and their reputation builds up.

The current scripts are kept for a later cloud-signing integration but are not
called by the workflow:

- Windows: [ci/sign-windows.ps1](sign-windows.ps1) assumes a base64 PKCS#12
  certificate and `signtool /f`, which only works with a private CA or a
  grandfathered certificate. Public CAs have required hardware or cloud-HSM key
  storage since 1 June 2023 (CA/Browser Forum), so enabling signing again means
  swapping the `.pfx` secret for a cloud signing service (for example Azure
  Artifact Signing, SignPath or a CA cloud HSM) and verifying the result with
  `signtool verify /pa /v`.
- macOS: [ci/sign-macos.sh](sign-macos.sh) imports a Developer ID certificate,
  signs the Mach-O binaries and the `JadeForge.app` bundle and notarises the
  archive with `xcrun notarytool`. It runs again once macOS returns to the
  release matrix; notarisation needs an Apple Developer Program membership.
- Linux: the glibc check covers the runtime baseline; an optional AppImage or
  package signature is outside this repository.

The workflow also creates a GitHub build-provenance attestation for the staged
files. Keep the manual checklist below until a signed release has been
exercised against the real credentials.

## Release checklist

1. `just check`, `just finish` and, with game data, `cargo nextest run
   --workspace`.
2. Bump the workspace version in `Cargo.toml` on the release branch.
3. Merge the release branch through a pull request, then tag the merge commit on
   `origin/main` without checking it out. The tag run builds the binaries and
   the `release` job creates and publishes the GitHub release:
   `git fetch origin && git tag -a vMAJOR.MINOR.PATCH origin/main -m "..." &&
   git push origin vMAJOR.MINOR.PATCH`.
4. Verify the four Linux binaries with `ci/check-glibc.sh` against 2.35.
5. Verify the release contains one `.zip` per platform (the Linux `share/`
   tree is inside the Linux archive), one `SHA256SUMS`, `THIRD_PARTY_NOTICES.txt`,
   `cargo-metadata.json`, `sbom.cdx.json`, the provenance attestation when the
   repository is public and the Skia licence when present;
   `sha256sum -c SHA256SUMS` must pass.
6. Verify the icons on the real artifacts: the Windows `.exe` in Explorer and
   the Linux hicolor files resolving `Icon=jadeforge`.
7. This release is unsigned: state that in the notes. Signing is a follow-up
   (see [Release signing](#release-signing)).
8. Never ship a build whose real-savegame tests were skipped.

## Strict clippy report

`just strict` runs the lints that are not part of the gate yet
(`expect_used`, `indexing_slicing`, `arithmetic_side_effects`, `nursery`) and
prints the diagnostics per lint. When a lint reaches zero findings, promote it
to `deny` in the workspace lints and it becomes part of the normal gate. The
binary parsers intentionally index slices after validating offsets, so
`indexing_slicing` and `arithmetic_side_effects` need module-level allows rather
than a blanket fix.
