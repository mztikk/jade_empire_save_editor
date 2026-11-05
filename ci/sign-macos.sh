#!/usr/bin/env bash
# Sign and notarise the staged macOS binaries and app bundles.
#
# Requires a Developer ID Application certificate exported as a base64 PKCS#12
# and the Apple notarisation credentials. The workflow only calls this when the
# certificate secret is configured; see ci/README.md.
set -euo pipefail

dist=${1:?usage: sign-macos.sh <dist-directory>}
: "${MACOS_CERTIFICATE:?MACOS_CERTIFICATE is not set}"
: "${MACOS_CERTIFICATE_PASSWORD:?MACOS_CERTIFICATE_PASSWORD is not set}"
: "${MACOS_IDENTITY:?MACOS_IDENTITY is not set}"
: "${APPLE_ID:?APPLE_ID is not set}"
: "${APPLE_TEAM_ID:?APPLE_TEAM_ID is not set}"
: "${APPLE_APP_PASSWORD:?APPLE_APP_PASSWORD is not set}"

work=$(mktemp -d)
keychain="$work/jadeforge-signing.keychain-db"
keychain_password=$(uuidgen)
cleanup() {
    security delete-keychain "$keychain" >/dev/null 2>&1 || true
    rm -rf "$work"
}
trap cleanup EXIT

printf '%s' "$MACOS_CERTIFICATE" | base64 --decode > "$work/certificate.p12"
security create-keychain -p "$keychain_password" "$keychain"
security default-keychain -s "$keychain"
security unlock-keychain -p "$keychain_password" "$keychain"
security import "$work/certificate.p12" -k "$keychain" \
    -P "$MACOS_CERTIFICATE_PASSWORD" -T /usr/bin/codesign
security set-key-partition-list -S apple-tool:,apple:,codesign: \
    -s -k "$keychain_password" "$keychain" >/dev/null

# Sign every Mach-O in the staged folders, including the copy inside the .app
# bundle; the bare binaries stay usable from a terminal.
while IFS= read -r -d '' binary; do
    [ -f "$binary" ] && [ -x "$binary" ] || continue
    case "$binary" in
        *.exe) continue ;;
    esac
    echo "signing $binary"
    codesign --force --options runtime --timestamp \
        --sign "$MACOS_IDENTITY" "$binary"
    codesign --verify --strict --verbose=2 "$binary"
done < <(find "$dist" -type f -print0)

# A bundle has to be signed after its nested code. Signing the .app is what
# seals Info.plist and the icon resource for Gatekeeper.
shopt -s nullglob
apps=("$dist"/jadeforge-*/*.app)
for app in "${apps[@]}"; do
    echo "signing bundle $app"
    codesign --force --options runtime --timestamp \
        --sign "$MACOS_IDENTITY" "$app"
    codesign --verify --strict --verbose=2 "$app"
done

for folder in "$dist"/jadeforge-*/; do
    archive="$work/$(basename "$folder").zip"
    ditto -c -k --keepParent "$folder" "$archive"
    echo "notarising $(basename "$folder")"
    xcrun notarytool submit "$archive" \
        --apple-id "$APPLE_ID" \
        --team-id "$APPLE_TEAM_ID" \
        --password "$APPLE_APP_PASSWORD" \
        --wait
    cp "$archive" "$dist/"
done
