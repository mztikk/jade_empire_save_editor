#!/usr/bin/env bash
# Test ci/assemble-release.sh against a synthetic distribution directory.
set -euo pipefail

root=$(cd "$(dirname "$0")/.." && pwd)
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT

dist="$work/dist"
linux="$dist/jadeforge-1.2.3-x86_64-unknown-linux-gnu"
windows="$dist/jadeforge-1.2.3-x86_64-pc-windows-msvc"
mkdir -p "$linux/share/applications" "$windows"
printf '\177ELFlinux binary\n' > "$linux/jadeforge"
chmod 644 "$linux/jadeforge"
printf 'windows binary\n' > "$windows/jadeforge.exe"
printf 'desktop entry\n' > "$linux/share/applications/jadeforge.desktop"
printf 'notices\n' > "$dist/THIRD_PARTY_NOTICES.txt"
printf '{}\n' > "$dist/cargo-metadata.json"
printf '{}\n' > "$dist/sbom.cdx.json"
printf 'skia licence\n' > "$dist/LICENSE_SKIA"
printf 'project licence\n' > "$dist/LICENSE"

cat > "$work/CHANGELOG.md" <<'EOF'
# Changelog

## [Unreleased]

- Not released yet.

## [1.2.3] - 2026-01-02

### Added

- A thing worth releasing.
- Another thing.

## [1.2.2] - 2025-12-01

- Older change.
EOF

"$root/ci/assemble-release.sh" "$dist" v1.2.3 "$work/CHANGELOG.md" >/dev/null

[ -f "$dist/jadeforge-1.2.3-x86_64-unknown-linux-gnu.zip" ]
[ -f "$dist/jadeforge-1.2.3-x86_64-pc-windows-msvc.zip" ]

python3 - "$dist/jadeforge-1.2.3-x86_64-unknown-linux-gnu.zip" <<'PY'
import sys
import zipfile

with zipfile.ZipFile(sys.argv[1]) as archive:
    names = archive.namelist()
    binary = archive.getinfo("jadeforge-1.2.3-x86_64-unknown-linux-gnu/jadeforge")
    desktop = archive.getinfo(
        "jadeforge-1.2.3-x86_64-unknown-linux-gnu/share/applications/jadeforge.desktop"
    )
assert "jadeforge-1.2.3-x86_64-unknown-linux-gnu/jadeforge" in names, names
assert any(name.endswith("share/applications/jadeforge.desktop") for name in names), names
# Artifact downloads lose their modes; the ELF binary must be executable in
# the archive while ordinary files stay non-executable.
assert (binary.external_attr >> 16) & 0o111, oct(binary.external_attr >> 16)
assert not (desktop.external_attr >> 16) & 0o111, oct(desktop.external_attr >> 16)
PY

( cd "$dist" && sha256sum -c SHA256SUMS >/dev/null )

[ -s "$dist/RELEASE_NOTES.md" ]
grep -q 'A thing worth releasing' "$dist/RELEASE_NOTES.md"
grep -q 'Another thing' "$dist/RELEASE_NOTES.md"
! grep -q 'Older change' "$dist/RELEASE_NOTES.md"
! grep -q 'Not released yet' "$dist/RELEASE_NOTES.md"
grep -q 'unsigned' "$dist/RELEASE_NOTES.md"

# The checksum file lists assets only: no local licence, notes or directories.
! grep -q 'LICENSE$' "$dist/SHA256SUMS"
! grep -q 'RELEASE_NOTES' "$dist/SHA256SUMS"
grep -q 'jadeforge-1.2.3-x86_64-unknown-linux-gnu.zip' "$dist/SHA256SUMS"
grep -q 'jadeforge-1.2.3-x86_64-pc-windows-msvc.zip' "$dist/SHA256SUMS"
grep -q 'THIRD_PARTY_NOTICES.txt' "$dist/SHA256SUMS"

# Re-running on an existing directory must stay idempotent, and a tag without
# a changelog section falls back to a generic body.
"$root/ci/assemble-release.sh" "$dist" v9.9.9 "$work/CHANGELOG.md" >/dev/null
grep -q 'Release v9.9.9 of JadeForge.' "$dist/RELEASE_NOTES.md"
( cd "$dist" && sha256sum -c SHA256SUMS >/dev/null )

echo "test-assemble-release: ok"
