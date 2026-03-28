#!/usr/bin/env python3
"""Build assets/tiles/dustfall_terrain.png: farwest_ground wang grid + custom cobble & dirt tiles at indices 16–17."""

from __future__ import annotations

from pathlib import Path

try:
    from PIL import Image
except ImportError:
    raise SystemExit("requires Pillow: pip install pillow")

ROOT = Path(__file__).resolve().parents[1]
SRC = ROOT / "assets/tiles/farwest_ground.png"
OUT = ROOT / "assets/tiles/dustfall_terrain.png"
TS = 32


def gen_cobble() -> Image.Image:
    """32×32 top-down cobble / mortar — warm gray, reads as paved town street."""
    im = Image.new("RGBA", (TS, TS), (201, 184, 150, 255))
    px = im.load()
    # irregular stone patches
    stones = [
        (4, 4, 12, 10, (160, 152, 138)),
        (18, 2, 12, 11, (176, 168, 154)),
        (2, 16, 14, 12, (168, 160, 146)),
        (20, 14, 10, 14, (152, 146, 132)),
        (8, 26, 18, 4, (180, 172, 158)),
        (14, 8, 10, 8, (156, 148, 134)),
    ]
    for x, y, w, h, c in stones:
        for yy in range(y, min(y + h, TS)):
            for xx in range(x, min(x + w, TS)):
                px[xx, yy] = c + (255,)
    # mortar lines
    mortar = (139, 115, 85, 255)
    for x in range(TS):
        if x % 9 in (0, 1):
            for y in range(TS):
                px[x, y] = mortar
        if x % 9 == 2:
            for y in range(TS):
                r, g, b, a = px[x, y]
                px[x, y] = (max(r - 15, 0), max(g - 12, 0), max(b - 10, 0), a)
    for y in range(TS):
        if y % 8 in (0, 1):
            for x in range(TS):
                px[x, y] = mortar
    return im


def gen_dirt() -> Image.Image:
    """32×32 dirt trail — dust tracks, warm brown (scrub / mine road)."""
    import random

    rnd = random.Random(42)
    im = Image.new("RGBA", (TS, TS), (0, 0, 0, 0))
    px = im.load()
    base = (139, 115, 85, 255)
    light = (166, 139, 94, 255)
    dark = (110, 90, 65, 255)
    for y in range(TS):
        for x in range(TS):
            n = rnd.random()
            if n < 0.08:
                c = dark
            elif n < 0.35:
                c = light
            else:
                c = base
            # subtle track down center
            cx = abs(x - TS // 2)
            if cx < 4 and rnd.random() < 0.25:
                c = tuple(min(255, c[i] + 12) if i < 3 else 255 for i in range(4))
            px[x, y] = c
    return im


def main() -> None:
    base = Image.open(SRC).convert("RGBA")
    if base.size != (128, 128):
        raise SystemExit(f"expected 128×128 farwest_ground, got {base.size}")
    cobble_img = gen_cobble()
    dirt_img = gen_dirt()
    # 128 wide × 160 tall: 4×5 grid of 32px tiles; indices 0–15 unchanged, 16–17 custom
    out = Image.new("RGBA", (128, 160), (0, 0, 0, 0))
    out.paste(base, (0, 0))
    out.paste(dirt_img, (0, 128))  # index 16 = row 4 col 0
    out.paste(cobble_img, (32, 128))  # index 17 = row 4 col 1
    # pad remaining two cells in row 4 with copies of floor (index 6) from farwest
    tile6 = base.crop((64, 32, 96, 64))
    for col in (2, 3):
        out.paste(tile6, (col * 32, 128))
    out.save(OUT, "PNG")
    print(f"Wrote {OUT} (indices 16=dirt path, 17=cobblestone)")


if __name__ == "__main__":
    main()
