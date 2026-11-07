#!/usr/bin/env bash
# Test stage-windows-cross.sh with a synthetic cross build.
set -euo pipefail

root=$(cd "$(dirname "$0")/.." && pwd)
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT

dist="$work/dist"
v3="$work/v3"
baseline="$work/baseline"
mkdir -p "$v3" "$baseline"
printf 'v3 gui\n' > "$v3/jadeforge.exe"
printf 'v3 cli\n' > "$v3/jadeforge-cli.exe"
printf 'baseline gui\n' > "$baseline/jadeforge.exe"
printf 'baseline cli\n' > "$baseline/jadeforge-cli.exe"

version=$(awk -F'"' '/^version = / { print $2; exit }' "$root/Cargo.toml")
out="$dist/jadeforge-$version-x86_64-pc-windows-msvc"

"$root/ci/stage-windows-cross.sh" "$dist" "$v3" "$baseline" >/dev/null

for name in jadeforge.exe jadeforge-cli.exe jadeforge-generic-cpu.exe \
    jadeforge-cli-generic-cpu.exe LICENSE NOTICE README.md \
    THIRD_PARTY_NOTICES.txt cargo-metadata.json; do
    if [ ! -f "$out/$name" ]; then
        echo "test-stage-windows-cross: missing $name" >&2
        exit 1
    fi
done
cmp "$v3/jadeforge.exe" "$out/jadeforge.exe"
cmp "$baseline/jadeforge.exe" "$out/jadeforge-generic-cpu.exe"

# A stale file from an older staging run must not survive.
printf 'stale\n' > "$out/jadeforge-gui.exe"
"$root/ci/stage-windows-cross.sh" "$dist" "$v3" "$baseline" >/dev/null
if [ -e "$out/jadeforge-gui.exe" ]; then
    echo "test-stage-windows-cross: stale binary was not removed" >&2
    exit 1
fi

# A missing input must fail before the staged folder is replaced.
rm "$baseline/jadeforge-cli.exe"
if "$root/ci/stage-windows-cross.sh" "$dist" "$v3" "$baseline" >/dev/null 2>&1; then
    echo "test-stage-windows-cross: missing baseline binary was accepted" >&2
    exit 1
fi

echo "test-stage-windows-cross: ok"
