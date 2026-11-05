#!/usr/bin/env bash
# Collect the licence metadata that must ship with a binary distribution.
#
# This is intentionally a build-time, best-effort inventory: crates.io metadata
# does not contain the full licence texts, but it records every package and the
# licence it declares, and it copies the Skia licence that is extracted into the
# build directory. Run it from the repository root after a release build.
set -euo pipefail

out=${1:?usage: ci/notices.sh <dist-directory>}
mkdir -p "$out"

cp LICENSE "$out/LICENSE"
cargo metadata --locked --format-version 1 > "$out/cargo-metadata.json"

if command -v jq >/dev/null 2>&1; then
    {
        echo "JadeForge third-party dependency inventory"
        echo
        echo "This file lists the packages linked into the distribution and the"
        echo "licence each package declares in its crates.io metadata. The full"
        echo "licence texts are available from the package sources and are"
        echo "reproduced where the build system stages them (see LICENSE_SKIA)."
        echo
        echo "name<TAB>version<TAB>licence<TAB>repository"
        jq -r '.packages[]
            | [.name, .version, (.license // "UNKNOWN"), (.repository // "")]
            | @tsv' "$out/cargo-metadata.json" | sort -u
    } > "$out/THIRD_PARTY_NOTICES.txt"
else
    {
        echo "JadeForge third-party dependency inventory"
        echo
        echo "jq was not available, so this build only ships the cargo metadata."
        echo "See cargo-metadata.json for the complete package and licence list."
    } > "$out/THIRD_PARTY_NOTICES.txt"
fi

# Skia is downloaded as a prebuilt archive and stages its own licence in the
# build output. Ship it when the file exists.
skia_license=$(find target -path '*freya-skia-bindings-*/out/skia/LICENSE_SKIA' -print -quit 2>/dev/null || true)
if [ -n "$skia_license" ]; then
    cp "$skia_license" "$out/LICENSE_SKIA"
fi

echo "wrote $out/THIRD_PARTY_NOTICES.txt and $out/cargo-metadata.json"
