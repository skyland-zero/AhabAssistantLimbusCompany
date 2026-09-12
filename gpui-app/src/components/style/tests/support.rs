//! Shared helpers for the palette regression tests.

use crate::components::style::*;

// -------------------------------------------------------------------------
// Contrast helpers. Relative luminance and the WCAG 2.1 contrast ratio live
// here (not in shipping code) so the palette can be regression-locked without
// paying for the maths at render time.
// -------------------------------------------------------------------------

pub(super) fn channel(value: u8) -> f32 {
    let srgb = f32::from(value) / 255.0;
    if srgb <= 0.040_45 {
        srgb / 12.92
    } else {
        ((srgb + 0.055) / 1.055).powf(2.4)
    }
}

pub(super) fn srgb(token: ColorToken) -> (f32, f32, f32) {
    let hex = token.rgb_hex();
    (
        channel(((hex >> 16) & 0xff) as u8),
        channel(((hex >> 8) & 0xff) as u8),
        channel((hex & 0xff) as u8),
    )
}

pub(super) fn luminance(token: ColorToken) -> f32 {
    let (r, g, b) = srgb(token);
    0.2126 * r + 0.7152 * g + 0.0722 * b
}

/// Composite `over` onto `under`, honouring `over`'s alpha.
pub(super) fn flatten(over: ColorToken, under: ColorToken) -> ColorToken {
    if over.alpha() == 0xff {
        return over;
    }
    let alpha = f32::from(over.alpha()) / 255.0;
    let mix = |shift: u32| -> u32 {
        let o = (over.rgb_hex() >> shift) & 0xff;
        let u = (under.rgb_hex() >> shift) & 0xff;
        // Alpha compositing in linear space would be more correct, but the
        // tokens are authored in sRGB and that is what GPUI blends.
        let blended = f32::from(o as u8) * alpha + f32::from(u as u8) * (1.0 - alpha);
        (blended.round().clamp(0.0, 255.0) as u32) & 0xff
    };
    ColorToken::rgb((mix(16) << 16) | (mix(8) << 8) | mix(0))
}

pub(super) fn contrast(a: ColorToken, b: ColorToken) -> f32 {
    let (light, dark) = {
        let (la, lb) = (luminance(a), luminance(b));
        if la >= lb { (la, lb) } else { (lb, la) }
    };
    (light + 0.05) / (dark + 0.05)
}

/// Contrast of a text token against a surface token, both resolved against
/// the page background first so translucent skins are measured as painted.
pub(super) fn painted_contrast(palette: Palette, text: ColorToken, surface: ColorToken) -> f32 {
    let ground = flatten(surface, palette.background);
    contrast(flatten(text, ground), ground)
}

/// Every skin/scheme/accent combination, for exhaustive sweeps.
pub(super) fn all_palettes() -> Vec<Palette> {
    let mut palettes = Vec::new();
    for skin in SkinId::ALL {
        for scheme in [ColorScheme::Light, ColorScheme::Dark] {
            for accent in AccentId::ALL {
                palettes.push(Palette::for_skin(scheme, accent, skin));
            }
        }
    }
    palettes
}

pub(super) fn label(palette: Palette) -> String {
    format!(
        "{}/{}/{:?}",
        palette.skin.as_str(),
        match palette.scheme {
            ColorScheme::Light => "light",
            ColorScheme::Dark => "dark",
        },
        palette.accent
    )
}

/// Read every Rust source under `relative` (from the crate root).
pub(super) fn source_files(relative: &str) -> Vec<(std::path::PathBuf, String)> {
    fn walk(dir: &std::path::Path, out: &mut Vec<std::path::PathBuf>) {
        let Ok(entries) = std::fs::read_dir(dir) else {
            return;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                walk(&path, out);
            } else if path.extension().is_some_and(|ext| ext == "rs") {
                out.push(path);
            }
        }
    }
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(relative);
    let mut paths = Vec::new();
    walk(&root, &mut paths);
    paths.sort();
    paths
        .into_iter()
        .map(|path| {
            let source = std::fs::read_to_string(&path).unwrap_or_default();
            (path, source)
        })
        .collect()
}

/// Collect the numeric literals that follow every occurrence of `needle`.
pub(super) fn literals_after(source: &str, needle: &str) -> Vec<String> {
    let mut found = Vec::new();
    let mut rest = source;
    while let Some(index) = rest.find(needle) {
        rest = &rest[index + needle.len()..];
        let digits: String = rest
            .chars()
            .take_while(|c| c.is_ascii_digit() || *c == '.')
            .collect();
        if !digits.is_empty() {
            found.push(digits);
        }
    }
    found
}
