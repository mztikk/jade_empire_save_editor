#!/usr/bin/env bash
# Build the macOS .app bundle around a staged GUI binary.
#
# The bundle gives the app its Dock/Finder icon and a stable bundle identifier.
# The bare binary stays next to it in the release folder for terminal use.
set -euo pipefail

app=${1:?usage: macos-bundle.sh <JadeForge.app> <binary> <version>}
binary=${2:?usage: macos-bundle.sh <JadeForge.app> <binary> <version>}
version=${3:?usage: macos-bundle.sh <JadeForge.app> <binary> <version>}

case "$app" in
    *.app) ;;
    *)
        echo "macos-bundle: refusing to replace a path without .app: $app" >&2
        exit 1
        ;;
esac
if [ ! -x "$binary" ]; then
    echo "macos-bundle: $binary is not executable" >&2
    exit 1
fi

root=$(cd "$(dirname "$0")/.." && pwd)
icon="$root/crates/jadeforge-gui/assets/jadeforge.icns"
if [ ! -f "$icon" ]; then
    echo "macos-bundle: missing $icon" >&2
    exit 1
fi

# CFBundleVersion only accepts period-separated integers, so strip a semver
# pre-release/build suffix. Release versions (the normal case) are unchanged.
bundle_version=${version%%[-+]*}

contents="$app/Contents"
rm -rf "$app"
mkdir -p "$contents/MacOS" "$contents/Resources"
cp "$binary" "$contents/MacOS/jadeforge"
chmod +x "$contents/MacOS/jadeforge"
cp "$icon" "$contents/Resources/jadeforge.icns"

cat > "$contents/Info.plist" <<EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleDevelopmentRegion</key>
    <string>en</string>
    <key>CFBundleDisplayName</key>
    <string>JadeForge</string>
    <key>CFBundleExecutable</key>
    <string>jadeforge</string>
    <key>CFBundleIconFile</key>
    <string>jadeforge.icns</string>
    <key>CFBundleIdentifier</key>
    <string>io.github.mztikk.jadeforge</string>
    <key>CFBundleInfoDictionaryVersion</key>
    <string>6.0</string>
    <key>CFBundleName</key>
    <string>JadeForge</string>
    <key>CFBundlePackageType</key>
    <string>APPL</string>
    <key>CFBundleShortVersionString</key>
    <string>${bundle_version}</string>
    <key>CFBundleVersion</key>
    <string>${bundle_version}</string>
    <key>LSApplicationCategoryType</key>
    <string>public.app-category.utilities</string>
    <key>NSHighResolutionCapable</key>
    <true/>
</dict>
</plist>
EOF

if command -v plutil >/dev/null 2>&1; then
    plutil -lint "$contents/Info.plist" >/dev/null
fi

echo "macos-bundle: staged $app"
