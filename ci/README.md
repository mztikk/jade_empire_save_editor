# CI and release engineering

The GitHub Actions workflow lives in
[`.github/workflows/ci.yml`](../.github/workflows/ci.yml). It is active for
pushes to `main`, pull requests, tags and manual dispatches.

## Jobs

- `formatting`: `cargo fmt --all --check`.
- `test`: tests, doctests, clippy and rustdoc for `jadeforge-gff`,
  `jadeforge-core`, `jadeforge-session`, `jadeforge-gamedata` and
  `jadeforge-cli` on Linux, Windows and macOS. This includes the game-data
  tests that the previous workflow missed. On Windows it also checks the GUI so
  the `winresource` build script really embeds the `.exe` icon.
- `msrv`: `cargo check` with the declared MSRV (currently 1.88) on Linux.
- `gui`: installs the X11/Wayland/EGL development packages, runs the headless
  `freya-testing` UI suite, clippy and the release icon staging smoke test. The
  UI suite builds a synthetic savegame and game installation when
  `JADEFORGE_SAVE_DIR`/`JADEFORGE_GAME_DIR` are not set, so it is not a no-op on
  CI.
- `audit`: `cargo audit` against the lockfile.
- `publish` (non-PR only): builds the fat-LTO binaries with `just publish`,
  stages the Windows icon, the macOS `JadeForge.app` bundles and the Linux
  `share/` tree, checks the glibc baseline on Linux, writes
  `THIRD_PARTY_NOTICES.txt`, the dependency inventory, a CycloneDX
  `sbom.cdx.json`, `SHA256SUMS` and a GitHub build-provenance attestation, and
  attaches the files to a release when the ref is a tag. Windows and macOS also
  run the headless UI tests.

All actions are pinned to commit SHAs. The workflow has a read-only default
token; only the publish job may write release contents.

## Local reproduction

```sh
just check          # fmt, clippy, nextest, doctests
just strict         # non-gating audit of the strict lints
just coverage       # cargo llvm-cov summary, lcov and html
just publish        # fat-LTO binaries, macOS .app/Linux share, notices under dist/
just checksums      # dist/SHA256SUMS
just sbom           # dist/sbom.cdx.json
just icons          # regenerate the icon PNG/ICO/ICNS assets from icon.svg
just install-desktop # Linux GUI + launcher entry + hicolor icons
ci/test-icon-staging.sh # smoke-test the macOS bundle and Linux share staging
cargo bench -p jadeforge-core --bench save_corpus
cargo +nightly fuzz run gff   # long local run; see fuzz/README.md
```

`just check` is the canonical gate. With a real installation, add
`JADEFORGE_SAVE_DIR`/`JADEFORGE_GAME_DIR` to run the corpus tests; without them
the UI tests use the synthetic fixture.

## Release signing

Signing is wired into the publish job and runs only when the platform
credentials are configured; unsigned builds are still produced and marked as
unsigned in the release checklist.

- macOS (secrets `MACOS_CERTIFICATE`, `MACOS_CERTIFICATE_PASSWORD`,
  `MACOS_IDENTITY`, `APPLE_ID`, `APPLE_TEAM_ID`, `APPLE_APP_PASSWORD`):
  [ci/sign-macos.sh](sign-macos.sh) imports the Developer ID certificate,
  signs every Mach-O binary with the hardened runtime and timestamp, signs the
  `JadeForge.app` bundle after its nested binary, and submits a notarisation
  archive with `xcrun notarytool`. The archives are attached to the release.
  `spctl --assess` on the `.app` is the final manual check.
- Windows (secrets `WINDOWS_CERTIFICATE`, `WINDOWS_CERTIFICATE_PASSWORD`):
  [ci/sign-windows.ps1](sign-windows.ps1) finds `signtool.exe` in the Windows
  SDK and signs every `.exe` with SHA-256 and a timestamp; `just publish`
  embeds `assets/jadeforge.ico` before signing.
- Linux: the glibc check covers the runtime baseline; an optional AppImage or
  package signature is outside this repository.

The workflow also creates a GitHub build-provenance attestation for the staged
files. Keep the manual checklist below until the signed path has been exercised
on a real release.

## Release checklist

1. `just check` and, with game data, `cargo nextest run --workspace`.
2. Bump the workspace version in `Cargo.toml`.
3. Tag `vMAJOR.MINOR.PATCH` and push the tag.
4. Verify the four Linux binaries with `ci/check-glibc.sh` against 2.35.
5. Verify the release includes `SHA256SUMS`, `THIRD_PARTY_NOTICES.txt`,
   `cargo-metadata.json`, `sbom.cdx.json`, the provenance attestation, the
   Skia licence, the macOS `JadeForge.app` bundles and the Linux `share/` tree.
6. Verify the icons on the real artifacts: the Windows `.exe` in Explorer, the
   macOS `JadeForge.app` in Finder/Dock, and the Linux hicolor files resolving
   `Icon=jadeforge`.
7. Verify the macOS/Windows signing steps ran (the conditional steps are skipped
   when the secrets are absent); otherwise mark the release unsigned in the
   notes and run the manual signing commands above on a controlled host.
8. Never ship a build whose real-savegame tests were skipped.

## Strict clippy report

`just strict` runs the lints that are not part of the gate yet
(`expect_used`, `indexing_slicing`, `arithmetic_side_effects`, `nursery`) and
prints the diagnostics per lint. When a lint reaches zero findings, promote it
to `deny` in the workspace lints and it becomes part of the normal gate. The
binary parsers intentionally index slices after validating offsets, so
`indexing_slicing` and `arithmetic_side_effects` need module-level allows rather
than a blanket fix.
