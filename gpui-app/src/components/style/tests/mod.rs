//! Palette, geometry, decoration and accent regression tests.

mod contrast;
mod scale;
mod support;

use super::*;

#[test]
fn css_palettes_have_distinct_light_and_dark_surfaces() {
    let light = Palette::default();
    let dark = Palette::dark(AccentId::Crimson);
    assert_ne!(light.background, dark.background);
    assert_ne!(light.card, dark.card);
    // Light keeps the transparent web border; dark gained a visible rule
    // so cards stop floating without an outline.
    assert!(light.border.is_transparent());
    assert!(!dark.border.is_transparent());
    assert_eq!(dark.input.alpha(), 0x26);
}

#[test]
fn every_skin_separates_light_from_dark() {
    for skin in SkinId::ALL {
        for accent in AccentId::ALL {
            let light = Palette::for_skin(ColorScheme::Light, accent, skin);
            let dark = Palette::for_skin(ColorScheme::Dark, accent, skin);
            assert_ne!(
                light,
                dark,
                "{} ignores the colour scheme for {accent:?}",
                skin.as_str()
            );
            assert_ne!(light.background, dark.background);
            let (light_lum, dark_lum) = (
                support::luminance(light.background),
                support::luminance(dark.background),
            );
            assert!(
                light_lum - dark_lum > 0.35,
                "{} light/dark backgrounds are too close ({light_lum} vs {dark_lum})",
                skin.as_str()
            );
        }
    }
}

#[test]
fn skin_ids_are_stable_and_unknown_values_fall_back() {
    for skin in SkinId::ALL {
        assert_eq!(SkinId::parse(skin.as_str()), skin);
        assert!(!skin.name_zh().is_empty());
        assert!(!skin.name_en().is_empty());
        assert!(!skin.blurb_zh().is_empty());
        assert!(!skin.blurb_en().is_empty());
    }
    // Documented aliases stay readable for hand-written settings files.
    assert_eq!(SkinId::parse("modern"), SkinId::Default);
    assert_eq!(SkinId::parse("aero"), SkinId::Glass);
    assert_eq!(SkinId::parse("paper"), SkinId::Archive);
    assert_eq!(SkinId::parse("blood"), SkinId::Mist);
    assert_eq!(SkinId::parse("old-value"), SkinId::Default);
    assert_eq!(SkinId::parse(""), SkinId::Default);
    assert!(SkinId::Limbus.is_limbus());
    assert!(SkinId::Glass.is_glass());
    assert!(SkinId::Archive.is_archive());
    assert!(SkinId::Mist.is_mist());
    assert!(!SkinId::Default.is_limbus());
}

#[test]
fn every_skin_owns_a_distinct_decoration_language() {
    use std::collections::HashSet;
    let mut seen = HashSet::new();
    for skin in SkinId::ALL {
        assert_eq!(
            skin.decor(),
            Palette::for_skin(ColorScheme::Dark, AccentId::Crimson, skin).decor
        );
        assert!(
            seen.insert(format!("{:?}", skin.decor())),
            "{} shares a decoration language with another skin",
            skin.as_str()
        );
    }
    assert_eq!(seen.len(), SkinId::ALL.len());
}

#[test]
fn shape_radii_are_monotonic_for_every_skin() {
    for skin in SkinId::ALL {
        let shape = Shape::for_skin(skin);
        assert!(shape.radius_sm <= shape.radius_md);
        assert!(shape.radius_md <= shape.radius_lg);
        assert!(shape.radius_lg <= shape.radius_xl);
        assert!(shape.pill, "{}", skin.as_str());
        assert!(shape.border_width >= 1);
        assert_eq!(shape.radius(false), shape.radius_md);
        assert_eq!(shape.radius(true), shape.radius_lg);
        // Squared-off skins are the point of the frame language.
        if matches!(skin, SkinId::Limbus) {
            assert_eq!(shape.radius_md, 0);
        }
    }
}

#[test]
fn limbus_skin_replaces_neutral_surfaces_but_keeps_brand() {
    let modern = Palette::dark(AccentId::LimbusBrass);
    let limbus = Palette::limbus_dark(AccentId::LimbusBrass);
    assert_eq!(limbus.skin, SkinId::Limbus);
    assert_eq!(limbus.background.rgb_hex(), 0x0b0a0e);
    assert_eq!(limbus.card.rgb_hex(), 0x1b1114);
    assert_eq!(limbus.ring.rgb_hex(), 0xd8a800);
    assert!(!limbus.border.is_transparent());
    assert_eq!(limbus.brand, modern.brand);
    assert_ne!(limbus.background, modern.background);
    // The scheme axis is real now: the light scheme is parchment, not a
    // re-run of the dark surfaces.
    let light = Palette::limbus_light(AccentId::LimbusBrass);
    assert_eq!(light.background.rgb_hex(), 0xece3d2);
    assert_eq!(light.card.rgb_hex(), 0xf6efe1);
    assert!(support::luminance(light.background) > support::luminance(limbus.background));
}

#[test]
fn framed_skins_derive_the_frame_tone_from_the_accent() {
    // A non-brass accent must not keep a brass frame: that was the
    // violet-buttons-with-gold-brackets clash.
    let violet = Palette::limbus_dark(AccentId::Violet);
    assert_eq!(violet.ring, violet.decor_line);
    assert_ne!(violet.ring.rgb_hex(), 0xd8a800);
    let brass = Palette::limbus_dark(AccentId::LimbusBrass);
    assert_eq!(brass.decor_line.rgb_hex(), 0xd8a800);
}

#[test]
fn all_accent_ids_are_stable_and_unknown_values_fall_back() {
    for accent in AccentId::ALL {
        assert_eq!(AccentId::parse(accent.as_str()), accent);
        assert_ne!(
            Palette::light(accent).brand,
            Palette::light(accent).brand_hover
        );
        assert!(
            !AccentTokens::for_scheme(accent, ColorScheme::Light)
                .decor
                .is_transparent()
        );
        assert!(
            !AccentTokens::for_scheme(accent, ColorScheme::Dark)
                .decor
                .is_transparent()
        );
    }
    assert_eq!(AccentId::parse("old-value"), AccentId::Crimson);
    assert!(AccentTokens::is_signature(AccentId::LimbusBrass));
    assert!(!AccentTokens::is_signature(AccentId::Violet));
}

#[test]
fn accent_presets_cover_all_ids_without_changing_legacy_values() {
    assert_eq!(ACCENT_PRESETS.len(), AccentId::ALL.len());
    assert_eq!(ACCENT_PRESETS[0].id, "crimson");
    assert_eq!(ACCENT_PRESETS[4].id, "violet");
    assert_eq!(ACCENT_PRESETS[5].id, "limbus-brass");
    for (accent, light_brand, dark_brand) in [
        (AccentId::Crimson, 0xc8354f, 0xe05a72),
        (AccentId::Blue, 0x2563eb, 0x60a5fa),
        (AccentId::Amber, 0xd97706, 0xfbbf24),
        (AccentId::Emerald, 0x059669, 0x34d399),
        (AccentId::Violet, 0x7c3aed, 0xa78bfa),
    ] {
        assert_eq!(Palette::light(accent).brand.rgb_hex(), light_brand);
        assert_eq!(Palette::dark(accent).brand.rgb_hex(), dark_brand);
    }
}

#[test]
fn limbus_brass_has_scheme_specific_tokens_and_foregrounds() {
    let light = Palette::light(AccentId::LimbusBrass);
    let dark = Palette::dark(AccentId::LimbusBrass);
    assert_eq!(light.brand.rgb_hex(), 0x7a5517);
    assert_eq!(light.brand_hover.rgb_hex(), 0x5e3f10);
    assert_eq!(light.brand_light.rgb_hex(), 0xf4e5c2);
    assert_eq!(light.brand_foreground.rgb_hex(), 0xfafafa);
    assert_eq!(dark.brand.rgb_hex(), 0xd1aa52);
    assert_eq!(dark.brand_hover.rgb_hex(), 0xad8434);
    assert_eq!(dark.brand_light.rgb_hex(), 0x4a381d);
    assert_eq!(dark.brand_foreground.rgb_hex(), 0x17120a);
    // The rounded default skin keeps its neutral rule and ring; only the
    // framed skins pull in the accent's frame tone.
    assert_eq!(light.decor_line.rgb_hex(), 0xa1a1a1);
    assert_eq!(light.ring.rgb_hex(), 0xa1a1a1);
    assert_eq!(dark.decor_line.rgb_hex(), 0x737373);
}

#[test]
fn legacy_page_colors_follow_the_current_render_snapshot() {
    for palette in support::all_palettes() {
        set_current_render_palette(palette);
        assert_eq!(
            render_rgb(BACKGROUND),
            gpui::rgba(palette.background.rgba_hex())
        );
        assert_eq!(render_rgb(SURFACE), gpui::rgba(palette.card.rgba_hex()));
        assert_eq!(render_rgb(TEXT), gpui::rgba(palette.foreground.rgba_hex()));
        assert_eq!(render_rgb(ACCENT), gpui::rgba(palette.brand.rgba_hex()));
        assert_eq!(
            render_rgba((SURFACE << 8) | 0x33),
            gpui::rgba((palette.card.rgb_hex() << 8) | 0x33)
        );
    }
}

#[test]
fn legacy_page_accent_colors_follow_the_selected_accent() {
    let palette = Palette::dark(AccentId::Emerald);
    set_current_render_palette(palette);
    assert_eq!(render_rgb(0xd9a441), gpui::rgba(palette.brand.rgba_hex()));
    assert_eq!(render_rgb(0x8de3b2), gpui::rgba(palette.success.rgba_hex()));
}
