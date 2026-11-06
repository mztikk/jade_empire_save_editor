#!/usr/bin/env bash
# Assemble the release assets from the per-OS build artifacts.
#
# Release assets must be regular files, so every staged target folder is
# zipped. The script then writes one SHA256SUMS over exactly the assets the
# workflow uploads, and RELEASE_NOTES.md from the tag's CHANGELOG section plus
# the unsigned-build notice.
#
# usage: assemble-release.sh <dist-directory> <tag> [changelog]
set -euo pipefail

dist=${1:?usage: assemble-release.sh <dist-directory> <tag> [changelog]}
tag=${2:?usage: assemble-release.sh <dist-directory> <tag> [changelog]}
changelog=${3:-CHANGELOG.md}

command -v python3 >/dev/null 2>&1 || {
    echo "assemble-release: python3 is required" >&2
    exit 1
}

# Zip one staged target folder with python3 so the script does not depend on a
# zip binary being installed. Paths stay relative to the dist directory, which
# puts the target folder at the archive root. Artifacts downloaded from
# actions/upload-artifact lose their Unix modes, so native executables are
# marked executable again before they are stored.
zip_folder() {
    python3 - "$1" "$2" <<'PY'
import os
import sys
import zipfile

EXECUTABLE_MAGICS = (
    b"\x7fELF",          # ELF
    b"\xcf\xfa\xed\xfe",  # Mach-O 64-bit, little endian
    b"\xce\xfa\xed\xfe",  # Mach-O 32-bit, little endian
    b"\xfe\xed\xfa\xcf",  # Mach-O 64-bit, big endian
    b"\xfe\xed\xfa\xce",  # Mach-O 32-bit, big endian
    b"\xca\xfe\xba\xbe",  # universal Mach-O
)

dist, name = sys.argv[1], sys.argv[2]
base = os.path.join(dist, name)
with zipfile.ZipFile(os.path.join(dist, name + ".zip"), "w", zipfile.ZIP_DEFLATED) as archive:
    for root, dirs, files in os.walk(base):
        dirs.sort()
        for filename in sorted(files):
            path = os.path.join(root, filename)
            with open(path, "rb") as handle:
                if handle.read(4) in EXECUTABLE_MAGICS:
                    os.chmod(path, 0o755)
            archive.write(path, os.path.relpath(path, dist))
PY
}
if [ ! -d "$dist" ]; then
    echo "assemble-release: $dist is not a directory" >&2
    exit 1
fi

shopt -s nullglob
folders=("$dist"/jadeforge-*/)
if [ ${#folders[@]} -eq 0 ]; then
    echo "assemble-release: no staged jadeforge-*/ folders in $dist" >&2
    exit 1
fi

for folder in "${folders[@]}"; do
    name=$(basename "$folder")
    rm -f "$dist/$name.zip"
    zip_folder "$dist" "$name"
    echo "assemble-release: zipped $name"
done

# Extract the changelog section for this tag, e.g. v0.2.0 -> [0.2.0].
version=${tag#v}
body=$(mktemp)
trimmed=$(mktemp)
trap 'rm -f "$body" "$trimmed"' EXIT
if [ -f "$changelog" ]; then
    awk -v heading="## [$version]" '
        index($0, heading) == 1 { found = 1; next }
        found && substr($0, 1, 3) == "## " { exit }
        found { print }
    ' "$changelog" > "$body"
fi
awk 'NF { found = 1 } found' "$body" > "$trimmed"
if [ ! -s "$trimmed" ]; then
    printf 'Release %s of JadeForge.\n' "$tag" > "$trimmed"
fi

notes="$dist/RELEASE_NOTES.md"
{
    cat "$trimmed"
    printf '\n---\n\n'
    printf '**These builds are unsigned.** Windows SmartScreen warns about an unknown\n'
    printf 'publisher until code signing is added; verify the downloads against\n'
    printf '`SHA256SUMS`.\n'
} > "$notes"

# The checksum file must describe exactly the assets that are uploaded: the
# archives, the top-level notices and metadata, and the Skia licence.
assets=()
for file in "$dist"/*.zip "$dist/THIRD_PARTY_NOTICES.txt" \
    "$dist/cargo-metadata.json" "$dist/sbom.cdx.json" "$dist/LICENSE_SKIA"; do
    if [ -f "$file" ]; then
        assets+=("$(basename "$file")")
    fi
done
if [ ${#assets[@]} -eq 0 ]; then
    echo "assemble-release: nothing to checksum in $dist" >&2
    exit 1
fi
( cd "$dist" && printf '%s\0' "${assets[@]}" | sort -z | xargs -0 sha256sum > SHA256SUMS )

echo "assemble-release: wrote $notes and $dist/SHA256SUMS"
