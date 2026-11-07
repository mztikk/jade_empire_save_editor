#!/usr/bin/env bash
# Build the Windows x86-64 release binaries on Linux with cargo-xwin.
#
# cargo-xwin supplies the Microsoft CRT, SDK and linker paths, so the GUI can
# link the prebuilt Windows Skia libraries without a Windows host. It is a local
# prerequisite, not a workspace dependency.
#
# usage: publish-windows-cross.sh [dist-directory]
set -euo pipefail

root=$(cd "$(dirname "$0")/.." && pwd)
cd "$root"

dist=${1:-dist}
target=x86_64-pc-windows-msvc

case "$(uname -s)" in
    Linux) ;;
    *)
        echo "publish-windows-cross: run this on Linux" >&2
        exit 1
        ;;
esac

if ! command -v cargo-xwin >/dev/null 2>&1; then
    echo "publish-windows-cross: cargo-xwin is required" >&2
    echo "install it with: cargo install --locked cargo-xwin@0.23.1" >&2
    exit 1
fi

RUSTFLAGS="-C target-cpu=x86-64-v3" \
    cargo xwin build --locked --profile dist --target "$target" \
    -p jadeforge-gui -p jadeforge-cli
CARGO_TARGET_DIR="target/baseline" \
    cargo xwin build --locked --profile dist --target "$target" \
    -p jadeforge-gui -p jadeforge-cli

ci/stage-windows-cross.sh "$dist"
