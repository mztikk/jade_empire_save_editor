#!/usr/bin/env bash
# Test ci/publish-release.sh with a fake gh binary.
set -euo pipefail

root=$(cd "$(dirname "$0")/.." && pwd)
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT

dist="$work/dist"
mkdir -p "$dist/jadeforge-1.2.3-x86_64-unknown-linux-gnu"
printf 'binary\n' > "$dist/jadeforge-1.2.3-x86_64-unknown-linux-gnu/jadeforge"
printf 'notices\n' > "$dist/THIRD_PARTY_NOTICES.txt"
printf '{}\n' > "$dist/cargo-metadata.json"
printf '{}\n' > "$dist/sbom.cdx.json"
printf 'skia licence\n' > "$dist/LICENSE_SKIA"
printf 'project licence\n' > "$dist/LICENSE"
"$root/ci/assemble-release.sh" "$dist" v1.2.3 >/dev/null

log="$work/gh.log"
mkdir -p "$work/bin"
cat > "$work/bin/gh" <<'EOF'
#!/usr/bin/env bash
printf '%s\n' "$*" >> "$GH_LOG"
if [ "$1" = "release" ] && [ "$2" = "view" ] && [ "${GH_RELEASE_EXISTS:-0}" = "1" ]; then
    exit 0
fi
if [ "$1" = "release" ] && [ "$2" = "view" ]; then
    exit 1
fi
EOF
chmod +x "$work/bin/gh"

GH_LOG="$log" GH_RELEASE_EXISTS=0 PATH="$work/bin:$PATH" \
    "$root/ci/publish-release.sh" v1.2.3 "$dist"

grep -q 'release create v1.2.3 --verify-tag --draft --title JadeForge v1.2.3 --notes-file' "$log"
grep -q 'release upload v1.2.3' "$log"
grep -q 'jadeforge-1.2.3-x86_64-unknown-linux-gnu.zip' "$log"
grep -q 'SHA256SUMS' "$log"
grep -q 'release edit v1.2.3 --draft=false' "$log"

upload_line=$(grep 'release upload' "$log" | head -1)
case "$upload_line" in
    *RELEASE_NOTES.md* | *" $dist/LICENSE "*)
        echo "publish-release: uploaded a non-asset file: $upload_line" >&2
        exit 1
        ;;
esac

# An existing release (for example a draft from a failed run) must not be
# created again, but its assets are still replaced and it is published.
GH_LOG="$log" GH_RELEASE_EXISTS=1 PATH="$work/bin:$PATH" \
    "$root/ci/publish-release.sh" v1.2.3 "$dist"
[ "$(grep -c 'release create' "$log")" -eq 1 ]

echo "test-publish-release: ok"
