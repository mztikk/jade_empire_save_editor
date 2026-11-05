#!/usr/bin/env bash
# Stage the Linux launcher entry and hicolor icons in a release folder.
#
# The resulting share/ tree can be copied into ~/.local/share (or /usr/local)
# so the desktop environment can resolve Icon=jadeforge for the app id.
set -euo pipefail

out=${1:?usage: stage-linux-desktop.sh <dist-directory>}
root=$(cd "$(dirname "$0")/.." && pwd)
assets="$root/crates/jadeforge-gui/assets"

mkdir -p "$out/share/applications" "$out/share/icons/hicolor"
cp "$assets/linux/jadeforge.desktop" "$out/share/applications/jadeforge.desktop"

for size in 16 24 32 48 64 128 256 512; do
    directory="$out/share/icons/hicolor/${size}x${size}/apps"
    mkdir -p "$directory"
    cp "$assets/icon-$size.png" "$directory/jadeforge.png"
done

echo "stage-linux-desktop: staged $out/share/"
