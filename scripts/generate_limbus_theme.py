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
RIVET = (120, 100, 60)

random.seed(7)


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
    d = ImageDraw.Draw(img)
    d.rectangle([0, h - 30, w, h - 28], fill=GOLD_D)
    # hazard: dark red/black instead of gold/black
    x, flip = 0, False
    while x < w:
        if flip:
            d.polygon([(x, h), (x + 22, h), (x + 22 + 30, h - 28), (x + 30, h - 28)],
                      fill=(60, 14, 14))
        x += 22
        flip = not flip
    return img


def rivet_row(d: ImageDraw.ImageDraw, y: int, x0: int, x1: int, step: int = 90,
              color=RIVET):
    x = x0 + step // 2
    while x < x1:
        d.ellipse([x - 5, y - 5, x + 5, y + 5], fill=(20, 14, 10), outline=color, width=2)
        d.point((x - 1, y - 1), fill=color)
        x += step


def make_tagband() -> Image.Image:
    """Dark-red metal band with tag holes + rivets (card header / button base)."""
    w, h = 800, 72
    img = Image.new("RGB", (w, h), BLOOD_D)
    d = ImageDraw.Draw(img)
    d.rectangle([0, 0, w, 5], fill=(30, 8, 8))
    d.rectangle([0, h - 5, w, h], fill=GOLD_D)
    d.rectangle([0, 5, w, 9], fill=(120, 30, 30))
    # tag holes
    for x in (60, w // 2, w - 60):
        d.rounded_rectangle([x - 26, 12, x + 26, 26], radius=7, fill=(8, 5, 5),
                            outline=(20, 10, 10), width=2)
    rivet_row(d, h - 16, 0, w, step=120)
    img = grain(img, sigma=12, alpha=34)
    erode(ImageDraw.Draw(img), (0, 0, w, h), n=60)
    return img


def make_frame() -> Image.Image:
    """Riveted dark-red frame corner set: full 256 frame tile (transparent middle)."""
    s = 256
    img = Image.new("RGBA", (s, s), (0, 0, 0, 0))
    d = ImageDraw.Draw(img)
    t = 14
    d.rectangle([0, 0, s - 1, t - 1], fill=BLOOD_D + (255,))
    d.rectangle([0, s - t, s - 1, s - 1], fill=BLOOD_D + (255,))
    d.rectangle([0, 0, t - 1, s - 1], fill=BLOOD_D + (255,))
    d.rectangle([s - t, 0, s - 1, s - 1], fill=BLOOD_D + (255,))
    d.rectangle([0, 0, s - 1, 2], fill=GOLD_D + (255,))
    d.rectangle([0, s - 3, s - 1, s - 1], fill=GOLD_D + (255,))
    for x, y in [(28, 7), (s - 28, 7), (28, s - 7), (s - 28, s - 7),
                 (7, 28), (7, s - 28), (s - 7, 28), (s - 7, s - 28)]:
        d.ellipse([x - 4, y - 4, x + 4, y + 4], fill=(20, 14, 10, 255),
                  outline=RIVET + (255,), width=2)
    return img


def make_divider() -> Image.Image:
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
    make_tagband().save(OUT / "tagband.png", optimize=True)
    make_frame().save(OUT / "frame.png", optimize=True)
    make_divider().save(OUT / "divider.png", optimize=True)
    make_seal().save(OUT / "seal-red.png", optimize=True)
    for p in sorted(OUT.glob("*.png")):
        print(f"{p.name:16s} {p.stat().st_size/1024:7.1f} KB")


if __name__ == "__main__":
    main()
