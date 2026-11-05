#!/usr/bin/env bash
# Write a CycloneDX 1.5 inventory of the crates linked into the release.
#
# This is generated from `cargo metadata`; it lists every registry package and
# its declared licence. It is a software bill of materials, not a vulnerability
# scan (CI runs `cargo audit` separately).
set -euo pipefail

out=${1:?usage: ci/sbom.sh <dist-directory>}
mkdir -p "$out"
metadata=$(mktemp)
trap 'rm -f "$metadata"' EXIT
cargo metadata --locked --format-version 1 > "$metadata"
version=$(awk -F'"' '/^version = / { print $2; exit }' Cargo.toml)

jq --arg version "$version" '
  {
    bomFormat: "CycloneDX",
    specVersion: "1.5",
    version: 1,
    metadata: {
      component: {
        type: "application",
        name: "JadeForge",
        version: $version
      }
    },
    components: [
      .packages[]
      | select(.source != null)
      | {
          type: "library",
          name: .name,
          version: .version,
          purl: ("pkg:cargo/" + .name + "@" + .version)
        }
        + (if .license then { licenses: [{ license: { name: .license } }] } else {} end)
    ]
  }
' "$metadata" > "$out/sbom.cdx.json"
echo "wrote $out/sbom.cdx.json"
