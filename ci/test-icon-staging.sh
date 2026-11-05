#!/usr/bin/env bash
# Smoke-test the release icon staging on a non-macOS host.
#
# macOS bundles are only built on macOS and the Linux share tree is only staged
# in the Linux release job. This script exercises both staging scripts with a
# dummy executable and validates the plist and the copied files, so a broken
# path or plist key is caught by the Ubuntu GUI job.
set -euo pipefail

root=$(cd "$(dirname "$0")/.." && pwd)
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT

printf '#!/bin/sh\nexit 0\n' > "$work/jadeforge"
chmod +x "$work/jadeforge"

"$root/ci/macos-bundle.sh" "$work/JadeForge.app" "$work/jadeforge" "1.2.3"
python3 - "$work/JadeForge.app/Contents/Info.plist" <<'PY'
import plistlib
import sys

with open(sys.argv[1], "rb") as handle:
    info = plistlib.load(handle)
expected = {
    "CFBundleDisplayName": "JadeForge",
    "CFBundleExecutable": "jadeforge",
    "CFBundleIconFile": "jadeforge.icns",
    "CFBundleIdentifier": "io.github.mztikk.jadeforge",
    "CFBundleShortVersionString": "1.2.3",
    "CFBundleVersion": "1.2.3",
}
for key, value in expected.items():
    actual = info.get(key)
    if actual != value:
        raise SystemExit(f"Info.plist {key}: expected {value!r}, got {actual!r}")
PY
test -x "$work/JadeForge.app/Contents/MacOS/jadeforge"
cmp "$work/JadeForge.app/Contents/Resources/jadeforge.icns" \
    "$root/crates/jadeforge-gui/assets/jadeforge.icns"

# CFBundleVersion must stay numeric even if the workspace version carries a
# pre-release suffix.
"$root/ci/macos-bundle.sh" "$work/Prerelease.app" "$work/jadeforge" "1.2.3-beta.1"
python3 - "$work/Prerelease.app/Contents/Info.plist" <<'PY'
import plistlib
import sys

with open(sys.argv[1], "rb") as handle:
    info = plistlib.load(handle)
for key in ("CFBundleShortVersionString", "CFBundleVersion"):
    if info.get(key) != "1.2.3":
        raise SystemExit(f"Info.plist {key}: got {info.get(key)!r}")
PY

"$root/ci/stage-linux-desktop.sh" "$work/linux"
for size in 16 24 32 48 64 128 256 512; do
    test -f "$work/linux/share/icons/hicolor/${size}x${size}/apps/jadeforge.png"
done
cmp "$work/linux/share/applications/jadeforge.desktop" \
    "$root/crates/jadeforge-gui/assets/linux/jadeforge.desktop"
cmp "$work/linux/share/icons/hicolor/256x256/apps/jadeforge.png" \
    "$root/crates/jadeforge-gui/assets/icon-256.png"

echo "icon staging: OK"
