#!/usr/bin/env python3
"""Export the GPUI palette tables to Native Shell theme files.

One-time migration tool (deleted together with ``gpui-app/``):

    python scripts/export_nsh_theme.py

It reads the static token tables of the GPUI client -- the per-skin
``Palette`` literals in ``gpui-app/src/components/style/palette/skins.rs``
and the accent ramps / shapes in ``gpui-app/src/components/style/tokens.rs``
-- and writes:

    nsh-app/theme.json              the app's boot theme (``default`` skin)
    nsh-app/theme/skins.generated.ts  every skin x accent table, for ``setTheme()``

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
OUT_DIR_SKINS = OUT_DIR / "themes"
OUT_TS = OUT_DIR_SKINS / "skins.generated.ts"

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

ACCENT_SLUG = {
    "Crimson": "crimson",
    "Blue": "blue",
    "Amber": "amber",
    "Emerald": "emerald",
    "Violet": "violet",
    "LimbusBrass": "limbus-brass",
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


# Decoration tokens NSH needs but the GPUI `Palette` keeps as constants or
# builds in code. They are theme tokens here so presentation code never writes a
# colour literal: the card-header band (limbus), the skin's top-edge wash and
# its grain (mist). `None` means "derive from the palette or the accent".
#
# `(skin, scheme)` -> {token: 0xRRGGBBAA or "glow"}. Limbus keeps the band
# values from `palette/mod.rs` (`LIMBUS_BAND_*`); every other skin leaves the
# band transparent because nothing draws it.
DECOR_TOKENS: dict[tuple[str, str], dict[str, int | str]] = {
    ("Limbus", "Dark"): {
        "band-top": 0x3E0D0DFF,
        "band-bottom": 0x240A0AFF,
        "wash": 0x00000000,
        "grain": 0x00000000,
    },
    ("Limbus", "Light"): {
        "band-top": 0xE6DCC8FF,
        "band-bottom": 0xD8C9ABFF,
        "wash": 0x00000000,
        "grain": 0x00000000,
    },
    # The mist wash is the accent-tinted red plate `skin_background` lays over
    # the page; the grain is its 2%-white one-pixel hatch.
    ("Mist", "Dark"): {"wash": 0x8C1D1D5C, "grain": 0xFFFFFF05},
    ("Mist", "Light"): {"wash": 0x8C1D1D5C, "grain": 0xFFFFFF05},
    # Glass paints its top wash from `glow`, which is accent-tinted.
    ("Glass", "Dark"): {"wash": "glow"},
    ("Glass", "Light"): {"wash": "glow"},
}


def resolve_decor(skin: str, scheme: str, accent: dict[str, int]) -> dict[str, str]:
    """The decoration tokens for one skin and scheme, accent included."""
    transparent = "rgba(0, 0, 0, 0.000)"
    values: dict[str, str] = {}
    for token in ("band-top", "band-bottom", "wash", "grain"):
        value = DECOR_TOKENS.get((skin, scheme), {}).get(token, 0)
        if value == "glow":
            value = (accent["decor"] & 0x00FF_FFFF) << 8 | 0x3D
        if isinstance(value, int) and value & 0xFF == 0:
            values[token] = transparent
        else:
            values[token] = css(int(value))
    return values


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


def resolve_scheme(
    skin: str,
    scheme: str,
    accent_id: str,
    palettes: dict[tuple[str, str], dict[str, str]],
    accents: dict[tuple[str, str], dict[str, int]],
    locals_: dict[str, str],
) -> dict[str, str]:
    """Every colour token of one skin, scheme and accent, in CSS form."""
    palette = palettes.get((skin, scheme))
    if palette is None:
        raise ValueError(f"missing Palette arm for {skin}/{scheme}")
    accent = accents[(accent_id, scheme)]
    resolved: dict[str, str] = {}
    for field, token in FIELD_TO_TOKEN.items():
        if field not in palette:
            raise ValueError(f"{skin}/{scheme}: palette has no field {field}")
        resolved[token] = css(resolve(palette[field], accent, locals_))
    resolved.update(resolve_decor(skin, scheme, accent))
    return resolved


def build_theme(
    name: str,
    skin: str,
    palettes: dict[tuple[str, str], dict[str, str]],
    accents: dict[tuple[str, str], dict[str, int]],
    shapes: dict[str, dict[str, int]],
    locals_: dict[str, str],
) -> dict[str, object]:
    theme_vars = {token: f"{points}px" for token, points in shapes[skin].items()}
    css_vars: dict[str, dict[str, str]] = {"theme": theme_vars}
    for scheme in ("Light", "Dark"):
        css_vars[scheme.lower()] = resolve_scheme(
            skin, scheme, SKIN_ACCENT[skin], palettes, accents, locals_
        )
    return {"name": name, "cssVars": css_vars}


def ts_tokens(tokens: dict[str, str], indent: str) -> str:
    lines = ["{"]
    for token, value in tokens.items():
        lines.append(f'{indent}  "{token}": "{value}",')
    lines.append(f"{indent}}}")
    return "\n".join(lines)


def build_ts(
    palettes: dict[tuple[str, str], dict[str, str]],
    accents: dict[tuple[str, str], dict[str, int]],
    shapes: dict[str, dict[str, int]],
    locals_: dict[str, str],
) -> str:
    """The skin x accent table the app composes its runtime theme from."""
    accent_slugs = {accent: slug for accent, slug in ACCENT_SLUG.items()}
    skins = list(SKIN_SLUG)

    out = [
        "// Generated by `python3 scripts/export_nsh_theme.py` from the GPUI",
        "// palette tables. Do not edit: rerun the script instead.",
        "//",
        "// `SKIN_TOKENS` holds every colour token of each skin and appearance for",
        "// the accent that skin ships with; `ACCENT_OVERRIDES` holds only what",
        "// another accent changes, so `theme/index.ts` can compose skin x accent",
        "// at run time instead of carrying 5 x 6 copies of the same table.",
        "",
        "export type SkinName = "
        + " | ".join(f'"{SKIN_SLUG[skin]}"' for skin in skins)
        + ";",
        'export type AccentName = "'
        + '" | "'.join(accent_slugs[accent] for accent in ACCENT_SLUG)
        + '";',
        "",
        "/** Colour tokens by the theme's CSS variable name, without the `--`. */",
        "export type SchemeTokens = Record<string, string>;",
        "",
        "/** Radius scale per skin, in points. */",
        "export const SKIN_RADII: Record<SkinName, Record<string, string>> = {",
    ]
    for skin in skins:
        key = SKIN_SLUG[skin]
        radii = ", ".join(f'"{token}": "{points}px"' for token, points in shapes[skin].items())
        out.append(f'  "{key}": {{ {radii} }},')
    out += [
        "};",
        "",
        "/** The accent a skin is designed around (Limbus is brass, the rest crimson). */",
        "export const SKIN_DEFAULT_ACCENT: Record<SkinName, AccentName> = {",
    ]
    for skin in skins:
        out.append(f'  "{SKIN_SLUG[skin]}": "{accent_slugs[SKIN_ACCENT[skin]]}",')
    out += [
        "};",
        "",
        "/** Every colour token of each skin, for its own accent. */",
        "export const SKIN_TOKENS: Record<SkinName, { light: SchemeTokens; dark: SchemeTokens }> = {",
    ]
    for skin in skins:
        default = SKIN_ACCENT[skin]
        out.append(f'  "{SKIN_SLUG[skin]}": {{')
        for scheme in ("Light", "Dark"):
            tokens = resolve_scheme(skin, scheme, default, palettes, accents, locals_)
            out.append(f"    {scheme.lower()}: {ts_tokens(tokens, '    ')},")
        out.append("  },")
    out += [
        "};",
        "",
        "/** What an accent other than the skin's own changes (empty when it is the same). */",
        "export const ACCENT_OVERRIDES: Record<",
        "  SkinName,",
        "  Partial<Record<AccentName, { light?: SchemeTokens; dark?: SchemeTokens }>>",
        "> = {",
    ]
    for skin in skins:
        out.append(f'  "{SKIN_SLUG[skin]}": {{')
        default = SKIN_ACCENT[skin]
        base = {
            scheme: resolve_scheme(skin, scheme, default, palettes, accents, locals_)
            for scheme in ("Light", "Dark")
        }
        for accent in ACCENT_SLUG:
            if accent == default:
                continue
            diffs = {}
            for scheme in ("Light", "Dark"):
                tokens = resolve_scheme(skin, scheme, accent, palettes, accents, locals_)
                diffs[scheme.lower()] = {
                    token: value
                    for token, value in tokens.items()
                    if base[scheme].get(token) != value
                }
            out.append(f'    "{accent_slugs[accent]}": {{')
            for scheme_key, tokens in diffs.items():
                if not tokens:
                    continue
                out.append(f"      {scheme_key}: {ts_tokens(tokens, '      ')},")
            out.append("    },")
        out.append("  },")
    out.append("};")
    return "\n".join(out) + "\n"


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

    theme = build_theme(SKIN_TITLE["Default"], "Default", palettes, accents, shapes, locals_)
    OUT_DEFAULT.write_text(
        json.dumps(theme, indent=2, ensure_ascii=False) + "\n", encoding="utf-8"
    )
    print(f"wrote {OUT_DEFAULT.relative_to(ROOT)}")

    OUT_TS.write_text(build_ts(palettes, accents, shapes, locals_), encoding="utf-8")
    print(f"wrote {OUT_TS.relative_to(ROOT)}")

    print(
        f"exported {len(SKIN_SLUG)} skins x {len(ACCENT_SLUG)} accents x 2 schemes, "
        f"{len(FIELD_TO_TOKEN)} colour tokens + {len(DECOR_TOKENS[('Limbus', 'Dark')])} decoration tokens"
    )


if __name__ == "__main__":
    main()
