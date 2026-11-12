#!/usr/bin/env python3
"""Export the GPUI palette tables to Native Shell theme files.

One-time migration tool (deleted together with ``gpui-app/``):

    python scripts/export_nsh_theme.py

It reads the static token tables of the GPUI client -- the per-skin
``Palette`` literals in ``gpui-app/src/components/style/palette/skins.rs``
and the accent ramps / shapes in ``gpui-app/src/components/style/tokens.rs``
-- and writes:

    nsh-app/theme.json          the app's default theme (``default`` skin)
    nsh-app/theme/<skin>.json   one theme per skin, for ``setTheme()``

Token names are the shadcn/ui CSS variable names NSH resolves classes
against, so ``text-muted-foreground`` / ``bg-brand`` behave the same in TSX
as the GPUI palette did.
"""

from __future__ import annotations

import re
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
GPUI = ROOT / "gpui-app" / "src" / "components" / "style"
TOKENS_RS = GPUI / "tokens.rs"
SKINS_RS = GPUI / "palette" / "skins.rs"
OUT_DIR = ROOT / "nsh-app"
OUT_DEFAULT = OUT_DIR / "theme.json"
OUT_DIR_SKINS = OUT_DIR / "theme"

# Rust field -> theme token name. ``accent_surface`` is shadcn's ``accent``:
# the subtle surface behind hover and selection, not the brand colour.
FIELD_TO_TOKEN = {
    "background": "background",
    "foreground": "foreground",
    "card": "card",
    "card_foreground": "card-foreground",
    "popover": "popover",
    "popover_foreground": "popover-foreground",
    "primary": "primary",
    "primary_foreground": "primary-foreground",
    "secondary": "secondary",
    "secondary_foreground": "secondary-foreground",
    "muted": "muted",
    "muted_foreground": "muted-foreground",
    "accent_surface": "accent",
    "accent_foreground": "accent-foreground",
    "destructive": "destructive",
    "border": "border",
    "input": "input",
    "ring": "ring",
    "success": "success",
    "success_light": "success-light",
    "warning": "warning",
    "warning_light": "warning-light",
    "danger": "danger",
    "danger_light": "danger-light",
    "danger_foreground": "danger-foreground",
    "selection": "selection",
    "scrim": "scrim",
    "decor_line": "decor-line",
    "hilite": "hilite",
    "glow": "glow",
    "brand": "brand",
    "brand_hover": "brand-hover",
    "brand_light": "brand-light",
    "brand_foreground": "brand-foreground",
}

# Skin -> the accent that belongs with it when the theme is used as-is.
SKIN_ACCENT = {
    "Default": "Crimson",
    "Glass": "Crimson",
    "Archive": "Crimson",
    "Mist": "Crimson",
    "Limbus": "LimbusBrass",
}

SKIN_SLUG = {
    "Default": "default",
    "Glass": "glass",
    "Archive": "archive",
    "Mist": "mist",
    "Limbus": "limbus",
}

SKIN_TITLE = {
    "Default": "AALC · Default",
    "Glass": "AALC · Glass",
    "Archive": "AALC · Archive",
    "Mist": "AALC · Mist",
    "Limbus": "AALC · Limbus",
}


def parse_accent_tokens(source: str) -> dict[tuple[str, str], dict[str, int]]:
    """``AccentTokens::for_scheme`` arms -> {(accent, scheme): {field: rgba}}."""
    block = source.split("pub const fn for_scheme", 1)[1]
    block = block.split("\n    const fn new(", 1)[0]
    arm = re.compile(
        r"\(AccentId::(\w+), ColorScheme::(\w+)\) => \{\s*(?://[^\n]*\n\s*)*"
        r"Self::new\(([^)]*)\)"
    )
    names = ("brand", "brand_hover", "brand_light", "brand_foreground", "decor")
    table: dict[tuple[str, str], dict[str, int]] = {}
    for match in arm.finditer(block):
        accent, scheme, args = match.group(1), match.group(2), match.group(3)
        values = [int(part.strip(), 0) for part in args.split(",")]
        if len(values) != len(names):
            raise ValueError(f"{accent}/{scheme}: expected 5 values, got {len(values)}")
        table[(accent, scheme)] = {
            name: (value & 0x00FF_FFFF) << 8 | 0xFF
            for name, value in zip(names, values)
        }
    return table


def parse_shapes(source: str) -> dict[str, dict[str, int]]:
    """``Shape::for_skin`` arms -> {skin: {radius_sm..xl}}."""
    arm = re.compile(
        r"SkinId::(\w+) => Self::new\((\d+), (\d+), (\d+), (\d+), ShadowLevel::\w+\)"
    )
    shapes: dict[str, dict[str, int]] = {}
    for match in arm.finditer(source):
        skin, sm, md, lg, xl = match.groups()
        shapes[skin] = {
            "radius-sm": int(sm),
            "radius": int(md),
            "radius-lg": int(lg),
            "radius-xl": int(xl),
        }
    return shapes


def parse_locals(source: str) -> dict[str, str]:
    """The ``let`` bindings at the top of ``tokens()``.

    ``decor_line`` and ``glow`` are initialised from the accent, then written
    as field shorthands (``decor_line,``) inside every ``Palette`` literal.
    """
    body = source.split("pub(super) const fn tokens", 1)[1]
    body = body.split("match (skin, scheme)", 1)[0]
    binding = re.compile(r"^\s*let (\w+) = (.+);$", re.MULTILINE)
    return {match.group(1): match.group(2) for match in binding.finditer(body)}


def parse_skins(source: str) -> dict[tuple[str, str], dict[str, str]]:
    """The ``tokens()`` match arms -> {(skin, scheme): {field: expression}}."""
    skins: dict[tuple[str, str], dict[str, str]] = {}
    current: tuple[str, str] | None = None
    fields: dict[str, str] = {}
    header = re.compile(r"^\s*\(SkinId::(\w+), ColorScheme::(\w+)\) => Palette \{$")
    field = re.compile(r"^\s{12}(\w+): (.+),$")
    shorthand = re.compile(r"^\s{12}(\w+),$")
    for line in source.splitlines():
        if current is None:
            match = header.match(line)
            if match:
                current = (match.group(1), match.group(2))
                fields = {}
            continue
        if line == "        },":
            skins[current] = fields
            current = None
            continue
        match = field.match(line)
        if match:
            fields[match.group(1)] = match.group(2)
            continue
        match = shorthand.match(line)
        if match:
            fields[match.group(1)] = match.group(1)
    if current is not None:
        raise ValueError(f"unterminated Palette arm for {current}")
    return skins


def resolve(
    expression: str, accent: dict[str, int], locals_: dict[str, str] | None = None
) -> int:
    """Turn one Rust token expression into an ``0xRRGGBBAA`` value."""
    expression = expression.strip()
    if locals_ and re.fullmatch(r"\w+", expression) and expression in locals_:
        return resolve(locals_[expression], accent, locals_)
    literal = re.fullmatch(r"ColorToken::rgba?\((0x[0-9a-fA-F]+)\)", expression)
    if literal:
        value = int(literal.group(1), 0)
        if expression.startswith("ColorToken::rgb("):
            return (value & 0x00FF_FFFF) << 8 | 0xFF
        return value
    if expression == "decor_line":
        return accent["decor"]
    tinted = re.fullmatch(r"tint\(decor_line, (0x[0-9a-fA-F]+)\)", expression)
    if tinted:
        alpha = int(tinted.group(1), 0) & 0xFF
        return (accent["decor"] & 0xFFFF_FF00) | alpha
    aliased = re.fullmatch(r"accent_tokens\.(\w+)", expression)
    if aliased:
        return accent[aliased.group(1)]
    raise ValueError(f"unsupported token expression: {expression}")


def css(value: int) -> str:
    red, green, blue, alpha = (
        (value >> 24) & 0xFF,
        (value >> 16) & 0xFF,
        (value >> 8) & 0xFF,
        value & 0xFF,
    )
    if alpha == 0xFF:
        return f"#{red:02x}{green:02x}{blue:02x}"
    return f"rgba({red}, {green}, {blue}, {alpha / 255:.3f})"


def build_theme(
    name: str,
    skin: str,
    palettes: dict[tuple[str, str], dict[str, str]],
    accents: dict[tuple[str, str], dict[str, int]],
    shapes: dict[str, dict[str, int]],
    locals_: dict[str, str],
) -> dict[str, object]:
    accent_id = SKIN_ACCENT[skin]
    theme_vars: dict[str, str] = {}
    for token, points in shapes[skin].items():
        theme_vars[token] = f"{points}px"
    css_vars: dict[str, dict[str, str]] = {"theme": theme_vars}
    for scheme in ("Light", "Dark"):
        palette = palettes.get((skin, scheme))
        if palette is None:
            raise ValueError(f"missing Palette arm for {skin}/{scheme}")
        accent = accents[(accent_id, scheme)]
        resolved: dict[str, str] = {}
        for field, token in FIELD_TO_TOKEN.items():
            if field not in palette:
                raise ValueError(f"{skin}/{scheme}: palette has no field {field}")
            resolved[token] = css(resolve(palette[field], accent, locals_))
        css_vars[scheme.lower()] = resolved
    return {"name": name, "cssVars": css_vars}


def main() -> None:
    tokens_rs = TOKENS_RS.read_text(encoding="utf-8")
    skins_rs = SKINS_RS.read_text(encoding="utf-8")

    accents = parse_accent_tokens(tokens_rs)
    shapes = parse_shapes(tokens_rs)
    locals_ = parse_locals(skins_rs)
    palettes = parse_skins(skins_rs)

    missing = [
        (skin, scheme)
        for skin in SKIN_ACCENT
        for scheme in ("Light", "Dark")
        if (skin, scheme) not in palettes
    ]
    if missing:
        raise SystemExit(f"skins.rs is missing Palette arms: {missing}")

    OUT_DIR_SKINS.mkdir(parents=True, exist_ok=True)
    import json

    for skin, slug in SKIN_SLUG.items():
        theme = build_theme(SKIN_TITLE[skin], skin, palettes, accents, shapes, locals_)
        target = OUT_DEFAULT if skin == "Default" else OUT_DIR_SKINS / f"{slug}.json"
        target.write_text(
            json.dumps(theme, indent=2, ensure_ascii=False) + "\n", encoding="utf-8"
        )
        print(f"wrote {target.relative_to(ROOT)}")

    print(
        f"exported {len(SKIN_SLUG)} skins x 2 schemes x "
        f"{len(FIELD_TO_TOKEN)} colour tokens"
    )


if __name__ == "__main__":
    main()
