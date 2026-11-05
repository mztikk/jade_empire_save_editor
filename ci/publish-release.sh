#!/usr/bin/env bash
# Create (as a draft), fill and publish the GitHub release for a tag.
#
# Re-running is safe: an existing release is reused, the assets are replaced
# with --clobber, and the release is published again. The draft step keeps a
# half-uploaded release out of sight if the upload fails.
#
# usage: publish-release.sh <tag> [dist-directory]
set -euo pipefail

tag=${1:?usage: publish-release.sh <tag> [dist-directory]}
dist=${2:-dist}

command -v gh >/dev/null 2>&1 || {
    echo "publish-release: gh is required" >&2
    exit 1
}
for required in "$dist/RELEASE_NOTES.md" "$dist/SHA256SUMS"; do
    if [ ! -f "$required" ]; then
        echo "publish-release: $required is missing; run assemble-release.sh first" >&2
        exit 1
    fi
done

shopt -s nullglob
assets=()
for file in "$dist"/*.zip "$dist"/*.txt "$dist"/*.json "$dist/LICENSE_SKIA" "$dist/SHA256SUMS"; do
    if [ -f "$file" ]; then
        assets+=("$file")
    fi
done
if [ ${#assets[@]} -eq 0 ]; then
    echo "publish-release: no assets to attach in $dist" >&2
    exit 1
fi

if ! gh release view "$tag" >/dev/null 2>&1; then
    gh release create "$tag" --verify-tag --draft \
        --title "JadeForge $tag" --notes-file "$dist/RELEASE_NOTES.md"
fi
gh release upload "$tag" "${assets[@]}" --clobber
gh release edit "$tag" --draft=false
