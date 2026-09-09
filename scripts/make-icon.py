#!/usr/bin/env python3
"""Draw the application icon: scripts/make-icon.py [out.png] [size]

A pinwheel of translucent petals on a white tile. Someone arriving from macOS
recognises the shape of a photo library at a glance, which is the point: the
icon has to read as "the photos app" in a launcher full of unfamiliar ones. The
petal shape, its proportions and the palette are our own; only the family is
borrowed.

Where petals overlap the colours mix, which is what gives the mark its depth, so
they are drawn with alpha rather than flat. Uses ImageMagick, which this project
already needs; Pillow is not a dependency.
"""
from __future__ import annotations

import shutil
import subprocess
import sys

OUT = sys.argv[1] if len(sys.argv) > 1 else "icon.png"
SIZE = int(sys.argv[2]) if len(sys.argv) > 2 else 512
SS = 4  # supersampling, for edges that survive being scaled down to 16 px

# Around the wheel, warm to cool. Eight petals: enough to read as a flower, few
# enough that each colour is still its own at 32 px.
PETALS = [
    (255, 59, 48), (255, 149, 0), (255, 204, 0), (52, 199, 89),
    (0, 199, 190), (48, 176, 199), (0, 122, 255), (175, 82, 222),
]
PETAL_ALPHA = 0.66
TILE = "white"
TILE_EDGE = "#DEE2E9"


def build(size: int, out: str) -> None:
    magick = shutil.which("magick")
    if magick is None:
        raise SystemExit("ImageMagick 7 (`magick`) is needed to draw the icon")
    s = size * SS
    c = s / 2.0
    radius = s * 0.225
    # the petal: a lens standing on the centre, long enough to reach well out
    rx, ry = s * 0.0925, s * 0.150
    offset = s * 0.038 + ry

    draw = []
    for i, (r, g, b) in enumerate(PETALS):
        angle = i * (360.0 / len(PETALS)) + 22.5      # off the axes, so it turns
        draw += ["-fill", f"rgba({r},{g},{b},{PETAL_ALPHA})",
                 "-draw", f"translate {c:.2f},{c:.2f} rotate {angle:.2f} "
                          f"ellipse 0,{-offset:.2f} {rx:.2f},{ry:.2f} 0,360"]

    args = [magick, "-size", f"{s}x{s}", "xc:none",
            # the white tile, with a hairline edge so it has a shape on white
            "-fill", TILE, "-stroke", TILE_EDGE, "-strokewidth", f"{max(1, s * 0.006):.1f}",
            "-draw", f"roundrectangle 0,0 {s - 1},{s - 1} {radius:.1f},{radius:.1f}",
            "-stroke", "none", *draw,
            # clip anything that reached past the corners
            "(", "-size", f"{s}x{s}", "xc:black", "-fill", "white", "-stroke", "none",
            "-draw", f"roundrectangle 0,0 {s - 1},{s - 1} {radius:.1f},{radius:.1f}", ")",
            "-alpha", "off", "-compose", "CopyOpacity", "-composite",
            "-resize", f"{size}x{size}"]
    if size <= 48:
        args += ["-unsharp", "0x0.6+0.7+0.02"]        # keep the small sizes crisp
    args += [out]
    run = subprocess.run(args, capture_output=True, text=True)
    if run.returncode != 0:
        raise SystemExit(f"could not draw the icon: {(run.stderr or '').strip()[:300]}")
    print(f"{out} {size}x{size}")


if __name__ == "__main__":
    build(SIZE, OUT)
