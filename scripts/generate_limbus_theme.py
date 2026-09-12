#!/usr/bin/env python3
"""v2 ORIGINAL assets redrawn from the real palette (sampled, not copied).

Palette (sampled from official art, values only):
  BG near-black cold #0B0A0E / card red-black #1B1114
  gold mustard #D8A800 / deep #B38600 / blood dark #4A1010 / bright #B91C1C
  bone #D8D0BC
All shapes original: riveted tag frames, stencil bands, grunge.
"""
from __future__ import annotations

import math
import random
from pathlib import Path

from PIL import Image, ImageDraw

OUT = Path(__file__).resolve().parents[1] / "gpui-app" / "resources" / "assets" / "themes" / "limbus"

BG = (11, 10, 14)
CARD = (27, 17, 20)
GOLD = (216, 168, 0)
GOLD_D = (179, 134, 0)
BLOOD_D = (74, 16, 16)
BLOOD = (150, 24, 24)
BONE = (216, 208, 188)

# Each generator reseeds itself so adding or removing one asset cannot shift
# another asset's random stream. Regenerating therefore only touches the files
# whose code actually changed.
SEED_BG = 7
SEED_DIVIDER = 11
SEED_SEAL = 13


def grain(base: Image.Image, sigma: int = 10, alpha: int = 30) -> Image.Image:
    n = Image.effect_noise(base.size, sigma).convert("L")
    dark = Image.new("RGB", base.size, (0, 0, 0))
    return Image.blend(base, Image.composite(base, dark, n), alpha / 255)


def vignette(img: Image.Image, strength: float = 0.6) -> Image.Image:
    w, h = img.size
    px = img.load()
    cx, cy = w / 2, h / 2
    maxd = math.hypot(cx, cy)
    for y in range(0, h, 2):
        for x in range(0, w, 2):
            d = math.hypot(x - cx, y - cy) / maxd
            f = 1.0 - strength * d * d
            r, g, b = px[x, y]
            px[x, y] = (int(r * f), int(g * f), int(b * f))
    return img


def erode(d: ImageDraw.ImageDraw, box, n: int = 40, color=BG):
    x0, y0, x1, y1 = box
    for _ in range(n):
        x = random.randint(x0, x1)
        y = random.randint(y0, y1)
        r = random.randint(1, 3)
        d.ellipse([x - r, y - r, x + r, y + r], fill=color)


def make_bg() -> Image.Image:
    random.seed(SEED_BG)
    w, h = 1600, 900
    img = Image.new("RGB", (w, h), BG)
    d = ImageDraw.Draw(img)
    # cold blue nebula blotches (very dim)
    for _ in range(26):
        x, y = random.randint(0, w), random.randint(0, h)
        r = random.randint(80, 260)
        c = random.choice([(16, 16, 34), (26, 14, 20), (14, 22, 26)])
        d.ellipse([x - r, y - r, x + r, y + r], fill=c)
    # faint cracks
    for _ in range(14):
        x, y = random.randint(0, w), random.randint(0, h)
        pts = [(x, y)]
        for _ in range(6):
            x += random.randint(-90, 90)
            y += random.randint(-40, 40)
            pts.append((x, y))
        d.line(pts, fill=(30, 28, 34), width=1)
    img = grain(img)
    img = vignette(img)
    # No page chrome is baked in. This plate is stretched to the window size,
    # so a bottom rule or hazard band would float at ~97% of the window height
    # instead of sitting at the bottom of the content, and would also be
    # stretched vertically. Chrome belongs to the widgets, not the backdrop.
    return img


# NOTE: the card-header band and the riveted frame are no longer generated as
# bitmaps. Stretching an 800x72 band to the card width distorted its rivets and
# smears, and a 256x256 frame tile could not be resized without the same
# problem, so both are now drawn as resolution-independent geometry in
# `gpui-app/src/components/base.rs`. Only the assets below are shipped.


def make_divider() -> Image.Image:
    random.seed(SEED_DIVIDER)
    w, h = 1200, 26
    img = Image.new("RGBA", (w, h), (0, 0, 0, 0))
    d = ImageDraw.Draw(img)
    cy = h // 2
    d.rectangle([0, cy - 1, w, cy + 1], fill=GOLD_D + (255,))
    cx = w // 2
    d.polygon([(cx - 34, cy), (cx, cy - 9), (cx + 34, cy), (cx, cy + 9)],
              outline=GOLD + (255,), width=2)
    d.polygon([(cx - 12, cy), (cx, cy - 4), (cx + 12, cy), (cx, cy + 4)],
              fill=BLOOD + (255,))
    for sx in (-1, 1):
        for i in range(1, 4):
            x = cx + sx * (52 + i * 30)
            ln = 8 - i
            d.line([x, cy - ln, x, cy + ln], fill=GOLD_D + (255,), width=2)
    return img


def make_seal() -> Image.Image:
    random.seed(SEED_SEAL)
    s = 256
    img = Image.new("RGBA", (s, s), (0, 0, 0, 0))
    d = ImageDraw.Draw(img)
    c = s // 2
    d.ellipse([6, 6, s - 6, s - 6], outline=BLOOD + (255,), width=8)
    d.ellipse([24, 24, s - 24, s - 24], outline=(90, 16, 16, 255), width=2)
    for i in range(12):
        a = 2 * math.pi * i / 12
        x1, y1 = c + 100 * math.cos(a), c + 100 * math.sin(a)
        x2, y2 = c + 84 * math.cos(a), c + 84 * math.sin(a)
        d.line([x1, y1, x2, y2], fill=BLOOD + (255,), width=4 if i % 3 == 0 else 2)
    d.polygon([(44, 200), (72, 200), (212, 56), (184, 56)], fill=(90, 16, 16, 235))
    d.polygon([(c, c - 32), (c + 32, c), (c, c + 32), (c - 32, c)],
              outline=BONE + (255,), width=4)
    for _ in range(260):
        x, y = random.randint(0, s - 1), random.randint(0, s - 1)
        if math.hypot(x - c, y - c) < 120 and random.random() < 0.5:
            d.point((x, y), fill=(0, 0, 0, 0))
    return img


def main() -> None:
    OUT.mkdir(parents=True, exist_ok=True)
    for p in OUT.glob("*.png"):
        p.unlink()
    make_bg().save(OUT / "bg.png", optimize=True)
    make_divider().save(OUT / "divider.png", optimize=True)
    make_seal().save(OUT / "seal-red.png", optimize=True)
    for p in sorted(OUT.glob("*.png")):
        print(f"{p.name:16s} {p.stat().st_size/1024:7.1f} KB")  # noqa: T201


if __name__ == "__main__":
    main()
