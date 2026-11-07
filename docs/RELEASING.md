# Creating and publishing a release locally

This is the manual path for a JadeForge release when the tag workflow in
[`.github/workflows/ci.yml`](../.github/workflows/ci.yml) is unavailable, for
example while the GitHub Actions budget is exhausted or the workflow is
manually disabled. It uses the same scripts as the hosted workflow, so the
resulting assets and release notes are identical.

Run every command from the repository root. Work on a task branch, never on
`main`; see [`git-workflow`](../.dsh/skills/git-workflow/SKILL.md).

## Prerequisites

Install these once:

- `just`, `cargo-nextest`, `gh`, `jq` and `python3`.
- A Rust toolchain that satisfies the workspace MSRV.
- `podman` for the glibc 2.35 Linux build. The first
  `just publish-linux` builds the image from [`ci/Containerfile`](../ci/Containerfile).
- `cargo-xwin` for the Windows cross build:
  `cargo install --locked cargo-xwin@0.23.1`.
- `clang`, `lld-link`, `llvm-rc` and the `x86_64-pc-windows-msvc` Rust target.
  Wine is optional and only needed for the smoke test.
- An authenticated `gh` with repository write access:
  `gh auth status` should show the `repo` scope.
- Real savegames for the corpus tests:
  `JADEFORGE_SAVE_DIR` and `JADEFORGE_GAME_DIR`.

cargo-xwin downloads the Microsoft CRT and SDK on first use and stores it in
`~/.cache/cargo-xwin`. Using it means accepting the Microsoft license linked
from the [cargo-xwin README](https://github.com/rust-cross/cargo-xwin).

Check that the hosted workflow will not compete with the local build:

```sh
gh workflow list --all
```

If `CI` is active, pushing the tag starts a hosted release. Either use the
hosted path and skip the build and publish steps below, or disable the
workflow first with `gh workflow disable CI`. Do not run both paths for the
same tag.

## Running inside a sandbox

An automated session that restricts writes to the workspace needs two one-shot
escalations, one per build recipe:

| command | why the sandbox blocks it |
| --- | --- |
| `just publish-linux` | podman needs a writable `/run/user/<uid>/libpod` and a working rootless user namespace |
| `just publish-windows-cross` | cargo-xwin refreshes `~/.cache/cargo-xwin/clang-cl` |

Run each recipe verbatim and request the escalation for that exact command,
after the version bump so the artifacts carry the right version. Everything
else in this runbook (the gate, git, `gh`, notices, SBOM, assembly and
verification) runs inside a workspace-only sandbox. Do not probe podman with
separate commands, move its storage into the workspace, use `unshare`, or
substitute another runtime: those are workarounds, not the release path. If the
escalation is refused, stop and ask instead of working around it.

Keep persistent logs in `scratch/`, not `/tmp`, which is often a per-command
tmpfs in a sandboxed session.

## 1. Prepare the version

```sh
just start release/vX.Y.Z
```

1. Set `version = "X.Y.Z"` in the workspace [`Cargo.toml`](../Cargo.toml). The
   crates inherit it.
2. Add a `## [X.Y.Z] - YYYY-MM-DD` section to
   [`CHANGELOG.md`](../CHANGELOG.md). Move the `[Unreleased]` entries into it
   and leave a fresh `[Unreleased]` section for the next cycle.
   `ci/assemble-release.sh` uses that heading to build the release notes. If it
   is missing, the notes fall back to a generic sentence.
3. Run the local gate:

   ```sh
   just check
   ```

4. Run the real-savegame tests as well:

   ```sh
   JADEFORGE_SAVE_DIR="/path/to/Jade Empire/Save" \
   JADEFORGE_GAME_DIR="/path/to/Jade Empire" \
   JADEFORGE_CONFIG_DIR="$(mktemp -d)" cargo nextest run --workspace
   ```

   The UI tests must not report "skipping" when the real data is available.
   Never ship a build whose real-savegame tests were skipped.

5. Commit the version bump and changelog, then run the hand-over guard:

   ```sh
   git add Cargo.toml Cargo.lock CHANGELOG.md
   git commit -m "release vX.Y.Z"
   just finish
   ```

6. Push the release branch, open the pull request and merge it. The repository
   uses merge commits:

   ```sh
   git push -u origin release/vX.Y.Z
   gh pr create --fill --base main --head release/vX.Y.Z
   gh pr merge --merge --delete-branch
   ```

   With CI disabled there are no hosted checks on the pull request. The local
   `just check` and the real-savegame run are the gate.

## 2. Tag the merged commit

Fetch the merged `main`, then tag the merge commit without checking it out:

```sh
git fetch origin
git tag -a vX.Y.Z origin/main -m "JadeForge vX.Y.Z"
git push origin vX.Y.Z
```

The tag must exist on the remote before the publish script runs, because
`ci/publish-release.sh` creates the release with `--verify-tag`.

If the workflow is enabled, stop here and let the tag run build and publish.
Otherwise continue with the local build.

## 3. Clear stale artifacts

```sh
rm -rf dist
```

`ci/assemble-release.sh` zips every `dist/jadeforge-*` folder it finds, so a
stale folder from an older version would silently end up in the release.

## 4. Build the Linux release

```sh
just publish-linux
```

This builds inside the Ubuntu 22.04 container with glibc 2.35 and stages:

```text
dist/jadeforge-X.Y.Z-x86_64-unknown-linux-gnu/
```

It contains `jadeforge`, `jadeforge-cli`, the two generic-CPU fallbacks, the
Linux `share/` tree, `LICENSE`, `NOTICE`, `README.md` and the licence notices.

Verify all four binaries against the glibc baseline. `publish-linux` checks two
of them itself; the release checklist checks all four:

```sh
linux=dist/jadeforge-X.Y.Z-x86_64-unknown-linux-gnu
for binary in jadeforge jadeforge-cli jadeforge-generic-cpu jadeforge-cli-generic-cpu; do
    ci/check-glibc.sh "$linux/$binary" 2.35
done
```

## 5. Build the Windows release

```sh
just publish-windows-cross
```

This uses cargo-xwin, the prebuilt Windows Skia libraries and the Microsoft
CRT/SDK. It builds the x86-64-v3 binaries and the baseline fallbacks and stages:

```text
dist/jadeforge-X.Y.Z-x86_64-pc-windows-msvc/
```

The folder contains the four `.exe` files, `LICENSE`, `NOTICE`, `README.md` and
the licence notices. Smoke-test the CLI under Wine:

```sh
wine dist/jadeforge-X.Y.Z-x86_64-pc-windows-msvc/jadeforge-cli.exe --help
```

The GUI can also be started under Wine if the desktop supports it, but the
headless test suite is the normal verification. The Windows executables embed
the application icon through the build script. The GUI executables also link as
GUI-subsystem binaries, so Explorer opens no console window next to the editor;
`crates/jadeforge-gui/tests/windows_subsystem.rs` checks the PE subsystem.

## 6. Write the notices and SBOM

The per-platform folders already contain their own notices. The release also
needs the top-level metadata files:

```sh
just notices
just sbom
```

This writes `dist/THIRD_PARTY_NOTICES.txt`, `dist/cargo-metadata.json`,
`dist/LICENSE_SKIA` when Skia's licence is available, and
`dist/sbom.cdx.json`.

Do not run `just checksums` for a release. It checksums the unzipped staging
tree; `ci/assemble-release.sh` writes the correct `SHA256SUMS` over the final
assets.

## 7. Assemble the release

```sh
ci/assemble-release.sh dist vX.Y.Z
```

The script:

1. zips every `dist/jadeforge-*` folder,
2. extracts the `## [X.Y.Z]` section from `CHANGELOG.md` into
   `dist/RELEASE_NOTES.md`,
3. appends the unsigned-build notice,
4. writes `dist/SHA256SUMS` over the archives, notices, metadata, SBOM and the
   Skia licence.

Verify the checksums before publishing:

```sh
(cd dist && sha256sum -c SHA256SUMS)
```

Inspect the notes and the archive contents:

```sh
sed -n '1,40p' dist/RELEASE_NOTES.md
python3 -m zipfile -l dist/jadeforge-X.Y.Z-x86_64-unknown-linux-gnu.zip
python3 -m zipfile -l dist/jadeforge-X.Y.Z-x86_64-pc-windows-msvc.zip
```

The expected assets are:

```text
jadeforge-X.Y.Z-x86_64-unknown-linux-gnu.zip
jadeforge-X.Y.Z-x86_64-pc-windows-msvc.zip
SHA256SUMS
THIRD_PARTY_NOTICES.txt
cargo-metadata.json
sbom.cdx.json
LICENSE_SKIA (when the Skia licence was staged)
```

`RELEASE_NOTES.md` is the release body and is not uploaded as an asset.

## 8. Publish the release

```sh
ci/publish-release.sh vX.Y.Z dist
```

The script creates a draft release, uploads every archive, notice, metadata
file, the Skia licence and `SHA256SUMS` with `--clobber`, then publishes the
draft. Re-running it is safe: it reuses an existing release and replaces the
assets.

The release is unsigned. The generated notes say so, and Windows SmartScreen
will warn about an unknown publisher until signing is added. The local path
does not create a GitHub build-provenance attestation; that step is limited to
the hosted workflow.

## 9. Verify the published release

```sh
gh release view vX.Y.Z
gh release view vX.Y.Z --json assets --jq '.assets[].name'
```

Download the published assets into a temporary directory and verify the
checksums from the release itself:

```sh
verify=$(mktemp -d)
gh release download vX.Y.Z --dir "$verify"
(cd "$verify" && sha256sum -c SHA256SUMS)
```

Check that:

- `RELEASE_NOTES.md` (shown as the release body) matches the version and says
  the builds are unsigned.
- The Linux archive contains the four binaries and the `share/` tree.
- The Windows archive contains the four `.exe` files.
- The Windows GUI `.exe` opens no console window.
- The Windows icon resolves in Explorer on a Windows machine.
- `github.com` shows the same assets and checksum file.

## 10. Clean up

```sh
rm -rf dist
```

`dist/` is gitignored. Removing it avoids stale archives being picked up by the
next `ci/assemble-release.sh` run. Keep the tag and the published release.

## Troubleshooting

- **`cargo-xwin` is missing**: install it with
  `cargo install --locked cargo-xwin@0.23.1`. `just publish-windows-cross`
  prints this hint.
- **`__std_*` undefined while linking Windows**: use the newer clang backend
  and sysroot:

  ```sh
  XWIN_CROSS_COMPILER=clang just publish-windows-cross
  ```

- **`advapi32.lib` or `kernel32.lib` missing**: the build was run through plain
  `cargo build` instead of `cargo xwin build`. Use the recipe.
- **`gh release create` says the tag is unknown**: push the tag first, see
  step 2.
- **Release notes fall back to a generic sentence**: the changelog has no
  `## [X.Y.Z]` heading.
- **`assemble-release` includes an old version**: `dist/` was not cleared
  before building. Remove it and rebuild.
- **The release was published with the wrong assets**: fix the local files and
  rerun `ci/publish-release.sh`; it uploads with `--clobber`.
- **A hosted tag run started as well**: do not publish both. Let one path
  finish, then use the other only to verify.

## Hosted workflow

When the GitHub Actions budget is restored, re-enable the workflow:

```sh
gh workflow enable CI
```

The normal release path is then the tag workflow described in
[`ci/README.md`](../ci/README.md): push the `vX.Y.Z` tag and let the `publish` and
`release` jobs build, assemble and publish. The local commands above remain
useful for reproducing a failed release or preparing assets offline.
