#!/usr/bin/env bash
# Stage the Windows cross-build binaries for assemble-release.sh.
#
# usage: stage-windows-cross.sh <dist-directory> [v3-source] [baseline-source]
set -euo pipefail

root=$(cd "$(dirname "$0")/.." && pwd)
cd "$root"

dist=${1:?usage: stage-windows-cross.sh <dist-directory> [v3-source] [baseline-source]}
v3=${2:-target/x86_64-pc-windows-msvc/dist}
baseline=${3:-target/baseline/x86_64-pc-windows-msvc/dist}
version=$(awk -F'"' '/^version = / { print $2; exit }' Cargo.toml)
out="$dist/jadeforge-$version-x86_64-pc-windows-msvc"

for source in "$v3/jadeforge.exe" "$v3/jadeforge-cli.exe" \
    "$baseline/jadeforge.exe" "$baseline/jadeforge-cli.exe"; do
    if [ ! -f "$source" ]; then
        echo "stage-windows-cross: missing $source" >&2
        exit 1
    fi
done

mkdir -p "$out"
rm -f "$out/jadeforge.exe" "$out/jadeforge-cli.exe" \
    "$out/jadeforge-generic-cpu.exe" "$out/jadeforge-cli-generic-cpu.exe" \
    "$out/jadeforge-baseline.exe" "$out/jadeforge-cli-baseline.exe" \
    "$out/jadeforge-gui.exe"
cp "$v3/jadeforge.exe" "$out/"
cp "$v3/jadeforge-cli.exe" "$out/"
cp "$baseline/jadeforge.exe" "$out/jadeforge-generic-cpu.exe"
cp "$baseline/jadeforge-cli.exe" "$out/jadeforge-cli-generic-cpu.exe"
cp LICENSE NOTICE README.md "$out/"
ci/notices.sh "$out" >/dev/null
echo "stage-windows-cross: staged $out/"
