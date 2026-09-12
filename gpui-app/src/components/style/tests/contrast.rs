//! WCAG 2.1 AA regression net for every skin x scheme x accent combination.

use super::support::*;
use crate::components::style::*;

/// WCAG AA (4.5:1) for the text/surface pairs the UI actually paints.
/// This is the regression net for the dark-on-dark destructive button and
/// the white-on-bright-accent label that shipped before the rework.
#[test]
fn text_pairs_meet_wcag_aa_for_every_skin() {
    let mut failures = Vec::new();
    for palette in all_palettes() {
        let pairs: [(&str, ColorToken, ColorToken); 11] = [
            (
                "foreground/background",
                palette.foreground,
                palette.background,
            ),
            ("foreground/card", palette.foreground, palette.card),
            (
                "card_foreground/card",
                palette.card_foreground,
                palette.card,
            ),
            (
                "popover_foreground/popover",
                palette.popover_foreground,
                palette.popover,
            ),
            (
                "secondary_foreground/secondary",
                palette.secondary_foreground,
                palette.secondary,
            ),
            (
                "muted_foreground/background",
                palette.muted_foreground,
                palette.background,
            ),
            (
                "muted_foreground/card",
                palette.muted_foreground,
                palette.card,
            ),
            (
                "primary_foreground/primary",
                palette.primary_foreground,
                palette.primary,
            ),
            (
                "brand_foreground/brand",
                palette.brand_foreground,
                palette.brand,
            ),
            (
                "danger_foreground/danger",
                palette.danger_foreground,
                palette.danger,
            ),
            (
                "accent_foreground/background",
                palette.accent_foreground,
                palette.background,
            ),
        ];
        for (name, text, surface) in pairs {
            let ratio = painted_contrast(palette, text, surface);
            if ratio < 4.5 {
                failures.push(format!("{} {name} = {ratio:.2}:1", label(palette)));
            }
        }
    }
    assert!(
        failures.is_empty(),
        "contrast failures:\n{}",
        failures.join("\n")
    );
}

/// The limbus stencilled title is painted on a procedural band rather than
/// on a palette surface, so it needs its own guard.
#[test]
fn limbus_header_titles_stay_readable_on_the_band() {
    for accent in AccentId::ALL {
        let dark = Palette::for_skin(ColorScheme::Dark, accent, SkinId::Limbus);
        let ratio = contrast(dark.accent_foreground, LIMBUS_BAND_TOP);
        assert!(
            ratio >= 4.5,
            "limbus dark {accent:?} header title on band = {ratio:.2}:1"
        );

        let light = Palette::for_skin(ColorScheme::Light, accent, SkinId::Limbus);
        let ratio = contrast(light.accent_foreground, LIMBUS_BAND_TOP_LIGHT);
        assert!(
            ratio >= 4.5,
            "limbus light {accent:?} header title on band = {ratio:.2}:1"
        );
    }
}

/// Only the dark schemes flip to dark ink; light accents that are bright
/// enough to need it (amber, emerald) flip as well.
#[test]
fn accent_foregrounds_are_contrast_picked_not_hardcoded() {
    for accent in AccentId::ALL {
        let light = AccentTokens::for_scheme(accent, ColorScheme::Light);
        let dark = AccentTokens::for_scheme(accent, ColorScheme::Dark);
        assert!(
            contrast(light.brand_foreground, light.brand) >= 4.5,
            "light {accent:?} brand foreground is illegible"
        );
        assert!(
            contrast(dark.brand_foreground, dark.brand) >= 4.5,
            "dark {accent:?} brand foreground is illegible"
        );
    }
    // Amber and emerald are the light-scheme accents that need dark ink.
    assert_eq!(
        AccentTokens::for_scheme(AccentId::Amber, ColorScheme::Light)
            .brand_foreground
            .rgb_hex(),
        0x241705
    );
    assert_eq!(
        AccentTokens::for_scheme(AccentId::Emerald, ColorScheme::Light)
            .brand_foreground
            .rgb_hex(),
        0x041a10
    );
    // Every dark-scheme accent takes dark ink on its bright chip.
    for accent in AccentId::ALL {
        let foreground = AccentTokens::for_scheme(accent, ColorScheme::Dark).brand_foreground;
        assert!(
            luminance(foreground) < 0.2,
            "dark {accent:?} should use dark ink on a bright accent"
        );
    }
}
