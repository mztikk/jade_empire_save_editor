#!/usr/bin/env python3
"""Regenerate the JadeForge application icon assets from ``icon.svg``.

The SVG is the source of truth; the PNGs, the Windows ICO and the macOS ICNS
are generated from it and committed so that a checkout never needs an SVG
renderer. Run this after editing the SVG:

    just icons

Requires ``rsvg-convert`` (librsvg), ImageMagick 7 (``magick``) and Python 3.
ImageMagick writes a valid multi-size ICO but not a usable ICNS container, so
the ICNS is packed here: a big-endian container of typed PNG elements.
"""

from __future__ import annotations

import shutil
import struct
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
ASSETS = ROOT / "crates" / "jadeforge-gui" / "assets"
SVG = ASSETS / "icon.svg"

PNG_SIZES = (16, 24, 32, 48, 64, 128, 256, 512, 1024)
ICO_SIZES = (16, 24, 32, 48, 64, 128, 256)
# Base sizes plus the Retina (``@2x``) variants Apple's iconutil also emits.
ICNS_ELEMENTS = (
    ("icp4", 16),
    ("icp5", 32),
    ("icp6", 64),
    ("ic07", 128),
    ("ic08", 256),
    ("ic09", 512),
    ("ic10", 1024),
    ("ic11", 32),
    ("ic12", 64),
    ("ic13", 256),
    ("ic14", 512),
)


def require(tool: str) -> None:
    if shutil.which(tool) is None:
        sys.exit(f"icons: {tool} is required to regenerate the assets")


def render_pngs() -> None:
    for size in PNG_SIZES:
        out = ASSETS / f"icon-{size}.png"
        subprocess.run(
            [
                "rsvg-convert",
                "-w",
                str(size),
                "-h",
                str(size),
                str(SVG),
                "-o",
                str(out),
            ],
            check=True,
        )
        print(f"rendered {out.relative_to(ROOT)}")


def write_ico() -> None:
    out = ASSETS / "jadeforge.ico"
    subprocess.run(
        [
            "magick",
            *[str(ASSETS / f"icon-{size}.png") for size in ICO_SIZES],
            str(out),
        ],
        check=True,
    )
    print(f"wrote {out.relative_to(ROOT)}")


def write_icns() -> None:
    elements = bytearray()
    for kind, size in ICNS_ELEMENTS:
        data = (ASSETS / f"icon-{size}.png").read_bytes()
        elements += kind.encode("ascii")
        elements += struct.pack(">I", len(data) + 8)
        elements += data
    out = ASSETS / "jadeforge.icns"
    out.write_bytes(b"icns" + struct.pack(">I", len(elements) + 8) + elements)
    print(f"wrote {out.relative_to(ROOT)}")


def main() -> None:
    require("rsvg-convert")
    require("magick")
    render_pngs()
    write_ico()
    write_icns()


if __name__ == "__main__":
    main()
