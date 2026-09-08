#!/usr/bin/env python3
"""Draw the application icon: scripts/make-icon.py [out.png] [size]

A rounded tile with two photo cards, the front one showing a horizon: readable
at 32 px in a launcher and still clean at 512. Needs Pillow.
"""
from __future__ import annotations

import sys
from PIL import Image, ImageDraw, ImageFilter

OUT = sys.argv[1] if len(sys.argv) > 1 else "icon.png"
SIZE = int(sys.argv[2]) if len(sys.argv) > 2 else 512
SS = 4  # supersampling for smooth edges

BG_TOP = (44, 92, 214)
BG_BOTTOM = (128, 66, 214)
CARD_BACK = (226, 232, 245)
CARD_FRONT = (255, 255, 255)
SKY = (150, 202, 255)
SKY_LOW = (206, 232, 255)
HILL_FAR = (96, 176, 138)
HILL_NEAR = (52, 140, 106)
SUN = (255, 198, 71)


def rounded(size: int, radius: int) -> Image.Image:
    m = Image.new("L", (size, size), 0)
    ImageDraw.Draw(m).rounded_rectangle((0, 0, size - 1, size - 1), radius, fill=255)
    return m


def gradient(size: int, top: tuple[int, int, int], bottom: tuple[int, int, int]) -> Image.Image:
    g = Image.new("RGB", (1, size))
    px = g.load()
    for y in range(size):
        t = y / max(size - 1, 1)
        px[0, y] = tuple(round(a + (b - a) * t) for a, b in zip(top, bottom))
    return g.resize((size, size))


def card(w: int, h: int, radius: int, fill: tuple[int, int, int]) -> Image.Image:
    c = Image.new("RGBA", (w, h), (0, 0, 0, 0))
    ImageDraw.Draw(c).rounded_rectangle((0, 0, w - 1, h - 1), radius, fill=fill + (255,))
    return c


def photo(w: int, h: int, radius: int) -> Image.Image:
    """The front card: a framed picture of a horizon."""
    c = card(w, h, radius, CARD_FRONT)
    pad = round(w * 0.075)
    iw, ih = w - 2 * pad, h - 2 * pad
    inner = Image.new("RGBA", (iw, ih), (0, 0, 0, 0))
    inner.paste(gradient(max(iw, ih), SKY, SKY_LOW).resize((iw, ih)), (0, 0))
    d = ImageDraw.Draw(inner)
    d.ellipse((round(iw * 0.62), round(ih * 0.12), round(iw * 0.62) + round(iw * 0.19),
               round(ih * 0.12) + round(iw * 0.19)), fill=SUN)
    d.polygon([(-iw, ih), (round(iw * 0.42), round(ih * 0.34)), (round(iw * 1.05), ih)], fill=HILL_FAR)
    d.polygon([(round(iw * -0.15), ih), (round(iw * 0.72), round(ih * 0.52)), (iw + iw, ih)], fill=HILL_NEAR)
    inner.putalpha(rounded(max(iw, ih), round(radius * 0.55)).resize((iw, ih)))
    c.alpha_composite(inner, (pad, pad))
    return c


def build(size: int) -> Image.Image:
    s = size * SS
    img = Image.new("RGBA", (s, s), (0, 0, 0, 0))
    tile = gradient(s, BG_TOP, BG_BOTTOM).convert("RGBA")
    tile.putalpha(rounded(s, round(s * 0.22)))
    img.alpha_composite(tile)

    cw, ch, radius = round(s * 0.50), round(s * 0.40), round(s * 0.055)
    back = card(cw, ch, radius, CARD_BACK).rotate(11, resample=Image.BICUBIC, expand=True)
    shadow = Image.new("RGBA", (s, s), (0, 0, 0, 0))
    shadow.alpha_composite(back, (round(s * 0.25), round(s * 0.20)))
    shadow = shadow.filter(ImageFilter.GaussianBlur(s * 0.012))
    img.alpha_composite(Image.new("RGBA", (s, s), (0, 0, 0, 60)), (0, 0)) if False else None
    img.alpha_composite(shadow)
    img.alpha_composite(back, (round(s * 0.25), round(s * 0.20)))

    front = photo(round(s * 0.56), round(s * 0.44), round(s * 0.06))
    fx, fy = round((s - front.width) / 2), round(s * 0.31)
    fshadow = Image.new("RGBA", (s, s), (0, 0, 0, 0))
    fshadow.alpha_composite(Image.new("RGBA", front.size, (0, 0, 0, 90)), (fx, fy + round(s * 0.012)))
    fshadow.putalpha(fshadow.getchannel("A").filter(ImageFilter.GaussianBlur(s * 0.02)))
    img.alpha_composite(fshadow)
    img.alpha_composite(front, (fx, fy))

    out = img.resize((size, size), Image.LANCZOS)
    out.putalpha(rounded(s, round(s * 0.22)).resize((size, size), Image.LANCZOS))
    return out


if __name__ == "__main__":
    build(SIZE).save(OUT)
    print(f"wrote {OUT} ({SIZE}x{SIZE})")
