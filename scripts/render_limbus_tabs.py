#!/usr/bin/env python3
"""Limbus-skin mockups v3 for ALL 7 tabs: real strings, detailed controls.

Same geometry as the app; reskin only. Output:
artifacts/visual/limbus-tabs/tab-{home,teams,themes,toolbox,resources,help,settings}.png
"""
from __future__ import annotations

import math
from pathlib import Path

from PIL import Image, ImageDraw, ImageFont

ROOT = Path(__file__).resolve().parents[1]
LIM = ROOT / "gpui-app" / "resources" / "assets" / "themes" / "limbus"
OUTDIR = ROOT / "artifacts" / "visual" / "limbus-tabs"

W, H = 1440, 900
BG = (11, 10, 14)
CARD = (24, 15, 18)
CARD2 = (18, 12, 14)
INSET = (10, 8, 10)
GOLD = (216, 168, 0)
GOLD_B = (232, 196, 60)
GOLD_D = (150, 112, 0)
BONE = (216, 208, 188)
MUTED = (150, 138, 112)
FAINT = (70, 44, 44)
BLOOD = (185, 40, 40)
BLOOD_D = (74, 16, 16)
DARK_TXT = (20, 12, 8)
GREEN = (60, 180, 110)
GREEN_D = (14, 44, 28)
HI = (90, 60, 20)


def font(size: int, bold: bool = False):
    cands = ["msyhbd.ttc", "segoeuib.ttf"] if bold else []
    cands += ["msyh.ttc", "segoeui.ttf", "arial.ttf"]
    for c in cands:
        p = Path("C:/Windows/Fonts") / c
        if p.exists():
            try:
                return ImageFont.truetype(str(p), size)
            except Exception:
                continue
    return ImageFont.load_default()


F_TAB = font(15)
F_S = font(13)
F_SM = font(12)
F_H = font(19, True)
F_B = font(14)
F_T = font(14, True)
F_BIG = font(22, True)
F_MONO = font(13)


def ls(d, xy, text, fnt, fill, track=2):
    """Letter-spaced text for headings."""
    x, y = xy
    for ch in text:
        d.text((x, y), ch, font=fnt, fill=fill)
        x += d.textlength(ch, font=fnt) + track
    return x


def icon(d: ImageDraw.ImageDraw, kind: str, cx: int, cy: int, s: int = 15, color=MUTED):
    r = s / 2
    w = 2
    if kind == "home":
        d.line([(cx - r, cy), (cx, cy - r - 1), (cx + r, cy)], fill=color, width=w)
        d.rectangle([cx - r + 3, cy, cx + r - 3, cy + r + 1], outline=color, width=w)
    elif kind == "users":
        d.ellipse([cx - r - 2, cy - r - 1, cx - r + 5, cy + 1], outline=color, width=w)
        d.arc([cx - r - 2, cy - 2, cx + r + 2, cy + r + 4], 200, 340, fill=color, width=w)
        d.ellipse([cx + 1, cy - r + 3, cx + 6, cy + 1], outline=color, width=2)
    elif kind == "palette":
        d.ellipse([cx - r, cy - r, cx + r, cy + r], outline=color, width=w)
        d.ellipse([cx - 1, cy - 4, cx + 3, cy], fill=color)
        d.ellipse([cx - 5, cy + 1, cx - 1, cy + 5], fill=color)
    elif kind == "wrench":
        d.line([(cx - r, cy + r), (cx + r - 2, cy - r + 2)], fill=color, width=w + 1)
        d.ellipse([cx + 1, cy - r - 1, cx + r + 2, cy - 1], outline=color, width=w)
    elif kind == "package":
        d.rectangle([cx - r, cy - r + 2, cx + r, cy + r], outline=color, width=w)
        d.line([(cx - r, cy - 2), (cx, cy + 2), (cx + r, cy - 2)], fill=color, width=w)
        d.line([(cx, cy + 2), (cx, cy + r)], fill=color, width=w)
    elif kind == "gear":
        d.ellipse([cx - 4, cy - 4, cx + 4, cy + 4], outline=color, width=w)
        for a in range(0, 360, 45):
            t = math.radians(a)
            d.line([(cx + 5 * math.cos(t), cy + 5 * math.sin(t)),
                    (cx + (r + 1) * math.cos(t), cy + (r + 1) * math.sin(t))],
                   fill=color, width=w)
    elif kind == "sun":
        d.ellipse([cx - 3, cy - 3, cx + 3, cy + 3], outline=color, width=w)
        for a in range(0, 360, 45):
            t = math.radians(a)
            d.line([(cx + 5 * math.cos(t), cy + 5 * math.sin(t)),
                    (cx + (r + 1) * math.cos(t), cy + (r + 1) * math.sin(t))],
                   fill=color, width=1)
    elif kind == "search":
        d.ellipse([cx - r + 1, cy - r, cx + 1, cy + 2], outline=color, width=w)
        d.line([(cx + 1, cy + 2), (cx + r, cy + r)], fill=color, width=w)
    elif kind == "refresh":
        d.arc([cx - r, cy - r, cx + r, cy + r], 40, 320, fill=color, width=w)
        d.polygon([(cx + r - 1, cy - r - 2), (cx + r + 3, cy - 1), (cx + r - 4, cy)],
                  fill=color)
    elif kind == "play":
        d.polygon([(cx - 3, cy - 5), (cx + 5, cy), (cx - 3, cy + 5)], fill=color)
    elif kind == "pause":
        d.rectangle([cx - 4, cy - 5, cx - 1, cy + 5], fill=color)
        d.rectangle([cx + 1, cy - 5, cx + 4, cy + 5], fill=color)
    elif kind == "stop":
        d.rectangle([cx - 4, cy - 4, cx + 4, cy + 4], fill=color)
    elif kind == "camera":
        d.rectangle([cx - r, cy - r + 3, cx + r, cy + r], outline=color, width=w)
        d.ellipse([cx - 3, cy - 2, cx + 3, cy + 4], outline=color, width=w)
        d.line([(cx - 4, cy - r + 3), (cx - 2, cy - r), (cx + 2, cy - r),
                (cx + 4, cy - r + 3)], fill=color, width=w)
    elif kind == "monitor":
        d.rectangle([cx - r, cy - r, cx + r, cy + 2], outline=color, width=w)
        d.line([(cx, cy + 2), (cx, cy + r)], fill=color, width=w)
        d.line([(cx - 4, cy + r), (cx + 4, cy + r)], fill=color, width=w)
    elif kind == "trash":
        d.rectangle([cx - 4, cy - 2, cx + 4, cy + r], outline=color, width=2)
        d.line([(cx - r, cy - 2), (cx + r, cy - 2)], fill=color, width=w)
        d.line([(cx - 4, cy), (cx - 4, cy + r - 2)], fill=color, width=1)
        d.line([(cx + 4, cy), (cx + 4, cy + r - 2)], fill=color, width=1)
    elif kind == "pencil":
        d.line([(cx - r + 2, cy + r - 2), (cx + r - 2, cy - r + 2)], fill=color, width=3)
    elif kind == "check":
        d.line([(cx - r, cy), (cx - 1, cy + r - 2), (cx + r, cy - r + 2)],
               fill=color, width=w)
    elif kind == "alert":
        d.polygon([(cx, cy - r), (cx + r, cy + r - 1), (cx - r, cy + r - 1)],
                  outline=color)
        d.line([(cx, cy - 2), (cx, cy + 2)], fill=color, width=w)
        d.point((cx, cy + 4), fill=color)
    elif kind == "clock":
        d.ellipse([cx - r, cy - r, cx + r, cy + r], outline=color, width=w)
        d.line([(cx, cy), (cx, cy - r + 3)], fill=color, width=w)
        d.line([(cx, cy), (cx + 3, cy + 1)], fill=color, width=w)
    elif kind == "crosshair":
        d.ellipse([cx - r + 1, cy - r + 1, cx + r - 1, cy + r - 1], outline=color, width=w)
        for dx, dy in ((0, -r - 1), (0, r + 1), (-r - 1, 0), (r + 1, 0)):
            d.line([(cx + dx, cy + dy),
                    (cx + dx * 0.55, cy + dy * 0.55)], fill=color, width=w)
    elif kind == "pill":
        d.rounded_rectangle([cx - r, cy - 4, cx + r, cy + 4], radius=4,
                            outline=color, width=w)
        d.line([(cx - 1, cy - 4), (cx - 1, cy + 4)], fill=color, width=1)
    elif kind == "gift":
        d.rectangle([cx - r, cy - 2, cx + r, cy + r], outline=color, width=w)
        d.rectangle([cx - r - 1, cy - r + 1, cx + r + 1, cy - 2], outline=color, width=2)
        d.line([(cx, cy - r + 1), (cx, cy + r)], fill=color, width=1)
    elif kind == "zap":
        d.polygon([(cx + 1, cy - r), (cx - 3, cy + 1), (cx, cy + 1),
                   (cx - 1, cy + r), (cx + 3, cy - 1), (cx, cy - 1)], fill=color)
    elif kind == "chev":
        d.line([(cx - 3, cy - 4), (cx + 2, cy), (cx - 3, cy + 4)], fill=color, width=w)
    elif kind == "calendar":
        d.rectangle([cx - r + 1, cy - r + 3, cx + r - 1, cy + r], outline=color, width=w)
        d.line([(cx - r + 1, cy - 1), (cx + r - 1, cy - 1)], fill=color, width=w)


class S:
    def __init__(self):
        bg = Image.open(LIM / "bg.png").convert("RGB").resize((W, H))
        self.img = Image.blend(Image.new("RGB", (W, H), BG), bg, 0.42)
        self.d = ImageDraw.Draw(self.img, "RGBA")

    # ---------- primitives ----------
    def titlebar(self, active: str):
        d = self.d
        d.rectangle([0, 0, W, 56], fill=(12, 8, 10, 255))
        d.rectangle([0, 54, W, 56], fill=GOLD_D)
        d.rectangle([0, 52, W, 54], fill=(5, 3, 4, 255))
        d.rectangle([14, 14, 48, 46], fill=BLOOD_D, outline=GOLD_D, width=1)
        d.rectangle([14, 14, 48, 22], fill=(96, 22, 22))
        d.text((19, 25), "LCB", font=font(11, True), fill=DARK_TXT)
        ls(d, (58, 15), "AALC", font(17, True), BONE, 1)
        items = [("主控台", "home"), ("队伍管理", "users"), ("主题包", "palette"),
                 ("工具箱", "wrench"), ("资源中心", "package"), ("帮助", "clock")]
        x = 200
        for t, ic in items:
            tw = 128
            if t == active:
                d.rectangle([x, 10, x + tw, 46], fill=GOLD)
                d.rectangle([x, 43, x + tw, 46], fill=GOLD_D)
                icon(d, ic, x + 24, 28, 15, DARK_TXT)
                d.text((x + 38, 17), t, font=F_TAB, fill=DARK_TXT)
            else:
                icon(d, ic, x + 24, 28, 15, FAINT)
                d.text((x + 38, 17), t, font=F_TAB, fill=MUTED)
            x += tw + 2
        icon(d, "sun", x + 330, 28, 15, MUTED)
        if active == "设置":
            d.rectangle([x + 352, 12, x + 390, 44], fill=(52, 26, 14), outline=GOLD, width=1)
        icon(d, "gear", x + 371, 28, 15, GOLD_B if active == "设置" else MUTED)
        d.text((W - 132, 18), "—", font=F_TAB, fill=MUTED)
        d.rectangle([W - 96, 20, W - 82, 34], outline=MUTED, width=2)
        d.text((W - 52, 16), "✕", font=F_TAB, fill=MUTED)

    def card(self, box, title=None, corners=False):
        d = self.d
        x0, y0, x1, y1 = box
        d.rectangle(box, fill=CARD, outline=FAINT, width=1)
        d.line([(x0 + 1, y0 + 1), (x1 - 1, y0 + 1)], fill=HI, width=1)
        if title:
            band = Image.open(LIM / "tagband.png").convert("RGB")
            band = band.resize((x1 - x0, 44))
            self.img.paste(band, (x0, y0))
            d = self.d = ImageDraw.Draw(self.img, "RGBA")
            d.rectangle([x0, y0 + 44, x1, y0 + 46], fill=GOLD_D)
            ls(d, (x0 + 22, y0 + 9), title, F_H, GOLD_B, 3)
            d.rectangle([x0, y0, x1, y0 + 46], outline=FAINT, width=1)
        if corners:
            fr = Image.open(LIM / "frame.png").convert("RGBA")
            fr = fr.resize((x1 - x0 + 12, y1 - y0 + 12))
            self.img.paste(fr, (x0 - 6, y0 - 6), fr)
            d = self.d = ImageDraw.Draw(self.img, "RGBA")
        return y0 + 52

    def gold_btn(self, box, label, ic=None):
        x0, y0, x1, y1 = box
        self.d.rectangle([x0, y1 - 3, x1, y1], fill=GOLD_D)
        self.d.rectangle([x0, y0, x1, y1 - 3], fill=GOLD)
        self.d.line([(x0, y0), (x1, y0)], fill=GOLD_B, width=1)
        if ic:
            icon(self.d, ic, x0 + 30, (y0 + y1) // 2, 14, DARK_TXT)
            self.d.text((x0 + 44, y0 + 8), label, font=F_B, fill=DARK_TXT)
        else:
            tw = self.d.textlength(label, font=F_B)
            self.d.text(((x0 + x1 - tw) / 2, y0 + 8), label, font=F_B, fill=DARK_TXT)

    def outline_btn(self, box, label, color=None, ic=None):
        c = color or FAINT
        self.d.rectangle(box, outline=c, width=1)
        x0, y0, x1, _ = box
        if ic:
            icon(self.d, ic, x0 + 26, y0 + 20, 14, BONE if color is None else color)
            self.d.text((x0 + 40, y0 + 8), label, font=F_B,
                        fill=BONE if color is None else color)
        else:
            tw = self.d.textlength(label, font=F_B)
            self.d.text(((x0 + x1 - tw) / 2, y0 + 8), label, font=F_B,
                        fill=BONE if color is None else color)

    def badge(self, x, y, label, tone="dim"):
        w = int(self.d.textlength(label, font=F_SM)) + 30
        if tone == "gold":
            self.d.rounded_rectangle([x, y, x + w, y + 26], radius=13,
                                     fill=(52, 26, 14), outline=GOLD, width=1)
            self.d.ellipse([x + 9, y + 9, x + 15, y + 15], fill=GOLD_B)
            self.d.text((x + 20, y + 4), label, font=F_SM, fill=GOLD_B)
        elif tone == "red":
            self.d.rounded_rectangle([x, y, x + w, y + 26], radius=13,
                                     fill=(58, 20, 20), outline=BLOOD, width=1)
            self.d.text((x + 12, y + 4), label, font=F_SM, fill=(232, 120, 120))
        elif tone == "green":
            self.d.rounded_rectangle([x, y, x + w, y + 26], radius=13,
                                     fill=GREEN_D, outline=GREEN, width=1)
            self.d.ellipse([x + 9, y + 9, x + 15, y + 15], fill=GREEN)
            self.d.text((x + 20, y + 4), label, font=F_SM, fill=GREEN)
        else:
            self.d.rounded_rectangle([x, y, x + w, y + 26], radius=13,
                                     fill=CARD2, outline=FAINT, width=1)
            self.d.text((x + 12, y + 4), label, font=F_SM, fill=MUTED)
        return w

    def switch(self, x, y, on=True):
        self.d.rounded_rectangle([x, y, x + 58, y + 28], radius=14,
                                 fill=GOLD if on else (40, 22, 20))
        self.d.rounded_rectangle([x, y, x + 58, y + 28], radius=14, outline=GOLD_D, width=1)
        kx = x + 30 if on else x + 2
        self.d.ellipse([kx, y + 3, kx + 24, y + 25], fill=CARD if on else FAINT,
                       outline=(6, 4, 5))

    def field(self, box, text, dim=False):
        self.d.rectangle(box, fill=INSET, outline=FAINT, width=1)
        x0, y0, _, _ = box
        self.d.text((x0 + 14, y0 + 8), text, font=F_MONO if len(text) <= 4 else F_B,
                    fill=MUTED if dim else BONE)

    def slider(self, x0, y, x1, frac):
        self.d.rectangle([x0, y, x1, y + 8], fill=(40, 22, 20))
        for i in range(11):
            tx = x0 + (x1 - x0) * i / 10
            self.d.line([(tx, y - 3), (tx, y + 11)], fill=FAINT, width=1)
        pos = x0 + (x1 - x0) * frac
        self.d.rectangle([x0, y, pos, y + 8], fill=GOLD)
        self.d.ellipse([pos - 11, y - 8, pos + 11, y + 16], fill=GOLD_B, outline=GOLD_D)
        self.d.ellipse([pos - 4, y - 1, pos + 4, y + 9], fill=CARD)

    def progress(self, x0, y, x1, frac, color=GOLD):
        self.d.rectangle([x0, y, x1, y + 10], fill=(40, 22, 20), outline=FAINT, width=1)
        self.d.rectangle([x0 + 2, y + 2, x0 + 2 + (x1 - x0 - 4) * frac, y + 8], fill=color)
        for i in range(1, 10):
            tx = x0 + (x1 - x0) * i / 10
            self.d.line([(tx, y), (tx, y + 10)], fill=(6, 4, 5), width=1)

    def scrollbar(self, x, y0, y1, frac=0.3, pos=0.0):
        self.d.rectangle([x, y0, x + 8, y1], fill=(8, 5, 6))
        h = (y1 - y0) * frac
        self.d.rectangle([x, y0 + (y1 - y0 - h) * pos, x + 8,
                          y0 + (y1 - y0 - h) * pos + h], fill=GOLD_D)

    def hazard(self):
        for i in range(0, W, 28):
            self.d.polygon([(i, H - 12), (i + 14, H - 12), (i + 6, H), (i - 8, H)],
                           fill=GOLD_D)
        self.d.rectangle([0, H - 14, W, H - 12], fill=GOLD)

    def save(self, name):
        self.hazard()
        p = OUTDIR / f"tab-{name}.png"
        self.img.save(p, optimize=True)
        print(f"saved {p.name} {(p.stat().st_size/1024):.1f} KB")


# ================= pages =================

def pg_home():
    s = S()
    s.titlebar("主控台")
    LX, RX = 16, W - 16 - 288
    s.card((LX, 72, RX - 8, 158))
    stats = [("今日日常", "2 / 3", "calendar"), ("镜牢层数", "第 3 层", "chev"),
             ("通行证", "Lv.42", "zap"), ("脑啡肽", "120/180", "pill")]
    for i, (t, v, ic) in enumerate(stats):
        x = LX + 26 + i * 262
        icon(s.d, ic, x + 12, 94, 14, GOLD_D)
        s.d.text((x + 32, 84), t, font=F_S, fill=MUTED)
        s.d.text((x, 106), v, font=font(17, True), fill=BONE)
        if i < 3:
            s.d.line([(x + 230, 84), (x + 230, 140)], fill=FAINT, width=1)
    tasks = [("窗口设置", "提起游戏窗口 · 固定 1920×1080", "已完成", "dim", "gear"),
             ("日常任务", "经验本 ×2 · 纽本 ×1 · TEAM #1", "运行中", "gold", "calendar"),
             ("领取奖励", "日常 / 周常 / 邮件", "排队", "dim", "gift"),
             ("狂气换体", "狂气 → 体力 · 溢出合成饼", "排队", "dim", "pill"),
             ("镜牢挑战", "困难镜牢 · 权重 chick +1", "排队", "dim", "crosshair")]
    y = 170
    for t, desc, st, tone, ic in tasks[:4]:
        s.d.rectangle([LX, y, RX - 8, y + 104], fill=CARD, outline=FAINT, width=1)
        s.d.line([(LX + 1, y + 1), (RX - 9, y + 1)], fill=HI, width=1)
        s.d.rectangle([LX, y, LX + 5, y + 104],
                      fill=GOLD if st == "运行中" else BLOOD_D if t == "窗口设置" else FAINT)
        icon(s.d, ic, LX + 32, y + 28, 15, GOLD)
        ls(s.d, (LX + 52, y + 12), t, F_T, GOLD_B if st == "运行中" else BONE, 2)
        w = s.badge(LX + 50, y + 44, st, tone)
        s.d.text((LX + 66 + w, y + 48), desc, font=F_S, fill=MUTED)
        s.switch(RX - 108, y + 20, on=(st == "运行中"))
        y += 114
    s.d.rectangle([LX, H - 96, RX - 8, H - 28], fill=CARD, outline=GOLD, width=1)
    s.gold_btn((LX + 16, H - 82, LX + 170, H - 42), "开始执行", "play")
    s.outline_btn((LX + 182, H - 82, LX + 310, H - 42), "暂停", ic="pause")
    s.d.text((LX + 326, H - 72), "全选", font=F_B, fill=MUTED)
    s.d.text((LX + 380, H - 72), "清空", font=F_B, fill=MUTED)
    s.progress(LX + 460, H - 64, RX - 60, 0.62)
    s.d.text((RX - 150, H - 84), "78%", font=F_B, fill=GOLD_B)
    s.card((RX, 72, W - 16, 316), "设备")
    s.d.text((RX + 24, 142), "mumu:0", font=F_T, fill=BONE)
    s.badge(RX + 120, 140, "已连接", "green")
    s.d.rectangle([RX + 24, 178, W - 40, 292], fill=(8, 6, 4), outline=GOLD, width=1)
    s.d.text((RX + 96, 214), "MEPHISTOPHELES", font=font(15, True), fill=FAINT)
    s.d.ellipse([RX + 96, 244, RX + 104, 252], fill=BLOOD)
    s.d.text((RX + 110, 240), "LIVE · 2s/帧", font=F_S, fill=BLOOD)
    s.card((RX, 328, W - 16, H - 28), "日志")
    for i, (lv, t) in enumerate([("镜牢", "进入第 3 层 · chick +1"), ("战斗", "Clash 胜利 · #1"),
                                 ("系统", "预览帧 540px 已推送"), ("奖励", "纽 ×1200 入账")]):
        c = GOLD_B if i == 0 else MUTED
        s.d.text((RX + 24, 398 + i * 34), f"[{lv}]", font=F_S, fill=c)
        s.d.text((RX + 90, 398 + i * 34), t, font=F_S, fill=BONE if i == 0 else MUTED)
    s.d.rectangle([RX + 24, H - 120, W - 40, H - 80], fill=CARD2, outline=FAINT, width=1)
    s.d.text((RX + 36, H - 110), "完成后：什么也不干", font=F_S, fill=MUTED)
    s.save("home")


def pg_teams():
    s = S()
    s.titlebar("队伍管理")
    s.d.rectangle([16, 72, W - 16, 148], fill=CARD, outline=FAINT, width=1)
    for i, t in enumerate(["全部 3", "镜牢 1", "经验本 1", "通用 1"]):
        x = 40 + i * 132
        if i == 0:
            s.d.rectangle([x, 88, x + 112, 132], fill=(52, 26, 14), outline=GOLD, width=1)
            s.d.text((x + 16, 96), t, font=F_B, fill=GOLD_B)
        else:
            s.d.rectangle([x, 88, x + 112, 132], fill=CARD2, outline=FAINT, width=1)
            s.d.text((x + 16, 96), t, font=F_B, fill=MUTED)
    s.gold_btn((W - 184, 88, W - 40, 132), "+ 新建队伍")
    teams = [("编队 1（震颤）", [("镜牢", "dim"), ("已配星光", "gold"), ("舍弃 2 项", "red")],
              ["#1 浮士德", "#2 以实玛利", "#3 良秀", "#4 鸿璐"]),
             ("编队 2（烧伤）", [("经验本", "dim")],
              ["#1 希斯克利夫", "#2 罗佳", "#3 格雷高尔"]),
             ("编队 3（呼吸）", [("通用", "dim"), ("已停用", "dim")],
              ["#1 李箱", "#2 堂吉诃德", "#3 默尔索", "#4 辛克莱", "#5 奥提斯"])]
    spots = [(16, 164), (W // 2 + 8, 164), (16, 486)]
    for (title, badges, members), (px, py) in zip(teams, spots):
        x1 = px + W // 2 - 24
        h = 306 if len(members) > 3 else 290
        s.d.rectangle([px, py, x1, py + h], fill=CARD, outline=FAINT, width=1)
        s.d.line([(px + 1, py + 1), (x1 - 1, py + 1)], fill=(66, 54, 30), width=1)
        s.d.text((px + 24, py + 14), title, font=F_H, fill=BONE)
        bx = px + 280
        for label, tone in badges:
            w = s.badge(bx, py + 14, label, tone)
            bx += w + 8
        icon(s.d, "pencil", x1 - 92, py + 28, 14, MUTED)
        icon(s.d, "trash", x1 - 52, py + 28, 14, BLOOD)
        for i, m in enumerate(members[:4]):
            cx = px + 24 + (i % 2) * 330
            cy = py + 130 + (i // 2) * 56
            s.d.rectangle([cx, cy, cx + 310, cy + 44], fill=CARD2, outline=FAINT, width=1)
            s.d.text((cx + 14, cy + 10), m, font=F_S, fill=MUTED)
            icon(s.d, "chev", cx + 288, cy + 22, 12, FAINT)
    s.save("teams")


def pg_themes():
    s = S()
    s.titlebar("主题包")
    s.d.rectangle([16, 72, W - 16, 148], fill=CARD, outline=FAINT, width=1)
    icon(s.d, "clock", 44, 110, 15, GOLD)
    s.d.text((64, 96), "总权重： 28", font=F_B, fill=MUTED)
    s.d.text((64, 118), "HARD · 困难镜牢周期", font=F_SM, fill=BLOOD)
    btns = ["按权重排序", "全部启用", "全部停用", "恢复默认权重"]
    x = W - 40
    for t in reversed(btns):
        w = len(t) * 15 + 36
        x -= w
        if t == "按权重排序":
            s.d.rectangle([x, 88, x + w, 132], fill=(52, 26, 14), outline=GOLD, width=1)
            icon(s.d, "check", x + 18, 110, 13, GOLD_B)
            s.d.text((x + 34, 96), t, font=F_B, fill=GOLD_B)
        else:
            s.outline_btn((x, 88, x + w, 132), t)
        x -= 12
    packs = [("chick", "鸡包 · 斩击", 1, 1), ("forgot", "道中斩击", 0, 1),
             ("faith", "N 狗 · 打击", 0, 1), ("tearful", "妖精 · 打击", 0, 1),
             ("nagel", "N 大锤 · 打击", 0, 0), ("time", "时间杀人魔", -5, 0),
             ("unloving", "芭芭雅嘎 · 斩击", -5, 0)]
    y = 164
    for pid, desc, wt, on in packs:
        if y > H - 120:
            break
        s.d.rectangle([16, y, W - 16, y + 78], fill=CARD, outline=FAINT, width=1)
        if wt > 0:
            s.d.rectangle([16, y, 20, y + 78], fill=GOLD)
        elif wt < 0:
            s.d.rectangle([16, y, 20, y + 78], fill=BLOOD_D)
        s.d.text((40, y + 10), pid, font=F_MONO, fill=GOLD_B if wt > 0 else BONE)
        s.d.text((40, y + 34), desc, font=F_S, fill=MUTED)
        s.d.text((40, y + 52), f"权重 {wt:+d}", font=F_T,
                 fill=GOLD_B if wt > 0 else (BLOOD if wt < 0 else MUTED))
        s.slider(430, y + 34, 1010, (wt + 5) / 15)
        s.switch(W - 120, y + 25, on=bool(on))
        y += 90
    s.scrollbar(W - 28, 164, H - 28, frac=0.55, pos=0.0)
    s.save("themes")


def pg_toolbox():
    s = S()
    s.titlebar("工具箱")
    tools = [("自动战斗", "循环执行战斗直至手动停止", "运行中", "green", "crosshair", 1),
             ("体力换饼", "狂气转体力并合成脑啡肽模块，防溢出", "待机", "dim", "pill", 0),
             ("辅助截图", "截取游戏窗口并保存到 AALC 目录", "—", "dim", "camera", 0),
             ("分辨率修改", "ADB 改 1080P 横屏 240DPI，可一键还原", "ADB", "dim", "monitor", 0)]
    for i, (t, desc, st, tone, ic, run) in enumerate(tools):
        x0 = 16 + (i % 2) * ((W - 48) // 2 + 16)
        y0 = 72 + (i // 2) * 296
        x1 = x0 + (W - 48) // 2
        s.d.rectangle([x0, y0, x1, y0 + 280], fill=CARD, outline=FAINT, width=1)
        s.d.line([(x0 + 1, y0 + 1), (x1 - 1, y0 + 1)], fill=(66, 54, 30), width=1)
        s.d.ellipse([x0 + 24, y0 + 22, x0 + 60, y0 + 58], outline=GOLD, width=2)
        icon(s.d, ic, x0 + 42, y0 + 40, 15, GOLD_B)
        s.badge(x1 - 140, y0 + 24, st, tone)
        ls(s.d, (x0 + 24, y0 + 76), t, F_H, BONE, 4)
        s.d.text((x0 + 24, y0 + 110), desc, font=F_S, fill=MUTED)
        if t == "分辨率修改":
            s.gold_btn((x0 + 24, y0 + 196, x0 + 330, y0 + 244), "修改 1080P", "monitor")
            s.outline_btn((x0 + 342, y0 + 196, x1 - 24, y0 + 244), "还原默认", ic="refresh")
        elif run:
            s.outline_btn((x0 + 24, y0 + 196, x1 - 24, y0 + 244), "停止", color=BLOOD, ic="stop")
        else:
            s.gold_btn((x0 + 24, y0 + 196, x1 - 24, y0 + 244), "运行", "play")
    s.d.rectangle([16, 680, W - 16, 736], fill=CARD, outline=FAINT, width=1)
    icon(s.d, "alert", 44, 708, 14, GOLD_D)
    s.d.text((66, 700), "工具请求通过 Python sidecar 执行 · 实时预览每 2s 推送", font=F_S, fill=MUTED)
    s.save("toolbox")


def pg_resources():
    s = S()
    s.titlebar("资源中心")
    s.d.rectangle([16, 72, W - 16, 148], fill=CARD, outline=FAINT, width=1)
    s.outline_btn((40, 88, 210, 132), "检查更新", ic="search")
    s.gold_btn((222, 88, 392, 132), "立即同步", "refresh")
    s.d.text((W - 330, 92), "同步进度 78%", font=F_B, fill=GOLD_B)
    s.progress(W - 330, 118, W - 40, 0.78)
    groups = [("OCR 识别包", "v1.4.1", 100, "已同步"), ("队伍快照", "v12", 100, "已同步"),
              ("主题包权重", "v3", 78, "同步中"), ("罪人头像", "v7", 45, "同步中"),
              ("状态图标", "v5", 0, "待同步"), ("帮助文档", "v2", 100, "已同步")]
    for i, (t, v, pct, st) in enumerate(groups):
        x0 = 16 + (i % 3) * ((W - 64) // 3 + 16)
        y0 = 164 + (i // 3) * 232
        x1 = x0 + (W - 64) // 3
        s.d.rectangle([x0, y0, x1, y0 + 216], fill=CARD, outline=FAINT, width=1)
        s.d.line([(x0 + 1, y0 + 1), (x1 - 1, y0 + 1)], fill=(66, 54, 30), width=1)
        icon(s.d, "package", x0 + 34, y0 + 32, 14, GOLD)
        s.d.text((x0 + 52, y0 + 18), t, font=F_T, fill=BONE)
        s.badge(x1 - 110, y0 + 18, v)
        col = GREEN if pct == 100 else GOLD
        s.progress(x0 + 24, y0 + 110, x1 - 24, pct / 100, col)
        s.d.text((x0 + 24, y0 + 130), f"{pct}% · {st}", font=F_S,
                 fill=GREEN if pct == 100 else (GOLD_B if pct else MUTED))
        if pct == 100:
            icon(s.d, "check", x0 + 40, y0 + 172, 13, GREEN)
            s.d.text((x0 + 60, y0 + 162), "校验通过", font=F_S, fill=GREEN)
        elif pct > 0:
            icon(s.d, "refresh", x0 + 40, y0 + 172, 13, GOLD_B)
            s.d.text((x0 + 60, y0 + 162), "正在同步…", font=F_S, fill=GOLD_B)
        else:
            s.d.text((x0 + 40, y0 + 162), "等待同步", font=F_S, fill=MUTED)
    s.save("resources")


def pg_help():
    s = S()
    s.titlebar("帮助")
    s.d.rectangle([16, 72, 256, H - 28], fill=CARD, outline=FAINT, width=1)
    s.d.text((32, 84), "目 录", font=F_S, fill=MUTED)
    toc = ["快速上手", "任务类型", "完成后动作", "工具箱", "常见问题"]
    y = 118
    for i, t in enumerate(toc):
        if i == 0:
            s.d.rectangle([28, y, 244, y + 38], fill=(52, 26, 14))
            s.d.rectangle([28, y, 32, y + 38], fill=GOLD)
            s.d.text((46, y + 8), t, font=F_B, fill=GOLD_B)
        else:
            s.d.text((46, y + 8), t, font=F_B, fill=MUTED)
        y += 48
    s.d.rectangle([248, 220, 252, 300], fill=GOLD_D)
    s.card((272, 72, W - 16, H - 28), "帮助文档")
    s.d.text((W - 260, 84), "简体中文", font=F_S, fill=GOLD_B)
    s.d.text((W - 176, 84), "English", font=F_S, fill=MUTED)
    ls(s.d, (296, 140), "堂吉诃德都能学会的操作方法", font(21, True), BONE, 2)
    s.d.line([(296, 176), (W - 60, 176)], fill=FAINT, width=1)
    steps = ["在「队伍」页面创建编队，选择人格与饰品体系",
             "回到「主控台」，添加任务并绑定队伍",
             "点击任务卡上的「启动」，进度与日志会在右侧实时显示"]
    y = 196
    for i, p in enumerate(steps, 1):
        s.d.ellipse([296, y + 2, 318, y + 24], fill=GOLD)
        s.d.text((303, y), str(i), font=F_T, fill=DARK_TXT)
        s.d.text((330, y + 2), p, font=F_B, fill=MUTED)
        y += 38
    s.d.text((296, y + 6), "完成后动作（队列全完成）：无动作 / 关闭游戏 / 关机 / 自定义命令",
             font=F_S, fill=MUTED)
    y += 40
    s.d.rectangle([296, y, W - 220, y + 108], fill=INSET, outline=FAINT, width=1)
    s.d.rectangle([296, y, 310, y + 108], fill=(52, 26, 14))
    s.d.text((322, y + 14), "git clone https://github.com/KIYI671/...", font=F_MONO, fill=GOLD_B)
    s.d.text((322, y + 42), "uv sync --frozen", font=F_MONO, fill=GOLD_B)
    s.d.text((322, y + 70), ".\\run-gpui.bat", font=F_MONO, fill=GOLD_B)
    y += 130
    s.d.text((296, y), "窗口识别失败？请以管理员身份运行，并确保游戏窗口未最小化。",
             font=F_S, fill=MUTED)
    s.scrollbar(W - 30, 140, H - 40, frac=0.4, pos=0.0)
    s.save("help")


def pg_settings():
    s = S()
    s.titlebar("设置")
    s.d.rectangle([16, 72, 216, H - 28], fill=CARD, outline=FAINT, width=1)
    s.d.text((32, 84), "快速导航", font=F_S, fill=MUTED)
    for i, t in enumerate(["外观", "全局热键", "系统与防护", "实验性功能", "更新与源配置", "任务通知", "关于"]):
        y = 120 + i * 52
        if i == 0:
            s.d.rectangle([28, y, 204, y + 40], fill=(52, 26, 14))
            s.d.rectangle([28, y, 32, y + 40], fill=GOLD)
            s.d.text((44, y + 9), t, font=F_B, fill=GOLD_B)
        else:
            s.d.text((44, y + 9), t, font=F_B, fill=MUTED)

    def card(y0, y1, title):
        s.card((232, y0, W - 16, y1), title, corners=(title == "外观"))
        return y0 + 52
    body = card(72, 344, "外观")
    s.d.text((256, body + 16), "主题模式", font=F_T, fill=BONE)
    for i, t in enumerate(["浅色", "深色", "跟随系统"]):
        bx = W - 344 + i * 108
        if i == 1:
            s.d.rectangle([bx, body + 10, bx + 100, body + 42], fill=GOLD)
            s.d.rectangle([bx, body + 39, bx + 100, body + 42], fill=GOLD_D)
            s.d.text((bx + 26, body + 16), t, font=F_B, fill=DARK_TXT)
        else:
            s.d.rectangle([bx, body + 10, bx + 100, body + 42], fill=CARD2, outline=FAINT, width=1)
            s.d.text((bx + 26, body + 16), t, font=F_B, fill=MUTED)
    s.d.text((256, body + 66), "强调色", font=F_T, fill=BONE)
    for i, c in enumerate([(200, 53, 79), (96, 165, 250), (217, 121, 6),
                           (5, 150, 105), (124, 58, 237), GOLD]):
        x0 = W - 300 + i * 46
        s.d.ellipse([x0 - 3, body + 59, x0 + 33, body + 95], fill=(6, 4, 5))
        s.d.ellipse([x0, body + 62, x0 + 30, body + 92], fill=c,
                    outline=BONE if i == 5 else GOLD_D, width=2)
    s.d.text((256, body + 116), "语言 / Language", font=F_T, fill=BONE)
    s.d.rectangle([W - 300, body + 110, W - 174, body + 142], fill=(52, 26, 14),
                  outline=GOLD, width=1)
    s.d.text((W - 278, body + 116), "简体中文", font=F_B, fill=GOLD_B)
    s.d.rectangle([W - 162, body + 110, W - 48, body + 142], fill=CARD2, outline=FAINT, width=1)
    s.d.text((W - 140, body + 116), "English", font=F_B, fill=MUTED)
    s.d.text((256, body + 166), "风格", font=F_T, fill=BONE)
    s.d.text((256, body + 190), "边狱皮肤：直角卡片 · 金边 · 纹理底", font=F_SM, fill=FAINT)
    s.d.rectangle([W - 300, body + 160, W - 174, body + 192], fill=CARD2, outline=FAINT, width=1)
    s.d.text((W - 278, body + 166), "现代", font=F_B, fill=MUTED)
    s.d.rectangle([W - 162, body + 160, W - 48, body + 192], fill=GOLD)
    s.d.rectangle([W - 162, body + 189, W - 48, body + 192], fill=GOLD_D)
    s.d.text((W - 140, body + 166), "边狱", font=F_B, fill=DARK_TXT)
    body = card(360, 586, "全局热键")
    s.d.text((256, body + 16), "启用全局热键", font=F_T, fill=BONE)
    s.switch(W - 124, body + 12, True)
    for i, (t, k) in enumerate([("启动 / 停止热键", "F10"), ("暂停 / 继续热键", "F11")]):
        yy = body + 58 + i * 56
        s.d.text((256, yy + 8), t, font=F_T, fill=BONE)
        s.field((W - 332, yy, W - 142, yy + 40), k)
        s.d.text((W - 124, yy + 8), "清除", font=F_B, fill=MUTED)
    body = card(602, 868, "系统与防护")
    rows = [("内存占用保护", "电脑总内存占用超过 90% 时自动清理内存防崩溃", True),
            ("最小化到托盘", "窗口最小化时隐藏到系统托盘区", True),
            ("开机自动启动", "跟随 Windows 系统开机自启 AALC", False)]
    yy = body + 16
    for t, desc, on in rows:
        s.d.text((256, yy), t, font=F_T, fill=BONE)
        s.d.text((256, yy + 26), desc, font=F_S, fill=MUTED)
        s.switch(W - 124, yy, on)
        yy += 72
    s.scrollbar(W - 28, 72, H - 28, frac=0.5, pos=0.0)
    s.save("settings")


if __name__ == "__main__":
    OUTDIR.mkdir(parents=True, exist_ok=True)
    pg_home()
    pg_teams()
    pg_themes()
    pg_toolbox()
    pg_resources()
    pg_help()
    pg_settings()
