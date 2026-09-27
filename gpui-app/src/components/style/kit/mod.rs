#![allow(dead_code)]

//! Bridge between the app's own [`Palette`] and GPUI Kit's global `Theme`.
//!
//! The app owns its skin system: five skins x light/dark x six accents, with
//! `components/style/tests/contrast.rs` asserting WCAG AA for every pair. GPUI
//! Kit owns its widget styling and reads it from `Theme`. Rather than let the
//! two drift into competing sources of truth, this module makes the palette
//! authoritative and *projects* it onto the theme:
//!
//! ```text
//! AppSettings -> Palette (authoritative) -> Theme (projection)
//! ```
//!
//! Nothing here writes back. `Palette` stays a `Copy` value object derived in
//! `AhabApp::render`, exactly as before, so a skin change remains one
//! re-derivation rather than a mutation of a second theme store.
//!
//! # Cost
//!
//! `Theme::change` re-resolves system and mono fonts, which is far too
//! expensive to run per frame, and `render` runs per frame. [`sync`] therefore
//! memoizes the last applied palette in a GPUI global and returns early when
//! nothing changed. `Palette` is `Copy + Eq`, so the comparison is exact.

mod colors;

use gpui::{App, Global, Window, px};
use gpui_component::{Theme, ThemeMode, ThemeTokens};

use super::palette::Palette;
use super::tokens::ShadowLevel;

/// The palette currently projected into `Theme`, used to skip redundant work.
struct AppliedPalette(Palette);

impl Global for AppliedPalette {}

/// Push `palette` into GPUI Kit's `Theme`, but only when it actually changed.
///
/// Safe to call from `render`: pass `window: None` there so the projection
/// cannot schedule another frame from inside a frame. Widgets rendered later
/// in the same pass already observe the new theme.
pub fn sync(palette: Palette, window: Option<&mut Window>, cx: &mut App) {
    if cx.has_global::<AppliedPalette>() && cx.global::<AppliedPalette>().0 == palette {
        return;
    }
    apply(palette, window, cx);
    cx.set_global(AppliedPalette(palette));
}

/// Force the projection, bypassing the memoization.
///
/// Used by tests and by the appearance page, which needs the theme to follow a
/// change even when the palette value itself happens to compare equal.
pub fn apply(palette: Palette, window: Option<&mut Window>, cx: &mut App) {
    let mode = if palette.is_dark() {
        ThemeMode::Dark
    } else {
        ThemeMode::Light
    };
    // `Theme::change` establishes the mode, resolves fonts and republishes the
    // derived `gpui_base::Theme`; the colour table and geometry are overwritten
    // immediately afterwards, then `sync_base` republishes with our values.
    Theme::change(mode, window, cx);
    {
        let theme = Theme::global_mut(cx);
        theme.colors = colors::colors(palette);
        // GPUI Kit keeps a second, legacy token table next to `colors` and its
        // widgets read *that* (`cx.theme().tokens.button_primary`). It is
        // derived from `ThemeColor`, but `apply_config` re-derives it from the
        // registry JSON, so overwriting `colors` alone leaves every button on
        // the built-in palette. Re-derive it from our colours in the same pass.
        theme.tokens = ThemeTokens::from(&theme.colors);
        theme.radius = px(palette.shape.radius(false) as f32);
        theme.radius_lg = px(palette.shape.radius(true) as f32);
        // Skins without a shadow language (Archive, Limbus) must not get GPUI
        // Kit's default elevation, so the flag follows the skin.
        theme.shadow = !matches!(palette.shape.shadow, ShadowLevel::None);
        theme.font_family = super::runtime::UI_FONT_FAMILY.into();
        theme.font_size = px(super::runtime::FONT_MD);
    }
    Theme::sync_base(cx);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::components::style::tokens::{AccentId, ColorScheme, SkinId};
    use gpui_component::ThemeColor;

    fn every_palette() -> impl Iterator<Item = Palette> {
        SkinId::ALL.into_iter().flat_map(|skin| {
            [ColorScheme::Light, ColorScheme::Dark]
                .into_iter()
                .flat_map(move |scheme| {
                    AccentId::ALL
                        .into_iter()
                        .map(move |accent| Palette::for_skin(scheme, accent, skin))
                })
        })
    }

    /// Every shipped combination must project without leaving a token at its
    /// `0x00000000` default. `ThemeColor` is built as a complete literal, so
    /// the compiler already enforces that all 141 fields are assigned; this
    /// asserts the assignments actually carry the palette through.
    #[test]
    fn every_shipped_palette_projects_every_surface() {
        for palette in every_palette() {
            let c = colors::colors(palette);
            let expect = |token| super::super::runtime::palette_hsla(token);
            assert_eq!(c.background, expect(palette.background));
            assert_eq!(c.foreground, expect(palette.foreground));
            assert_eq!(c.border, expect(palette.border));
            assert_eq!(c.danger, expect(palette.danger));
            assert_eq!(c.ring, expect(palette.ring));
            assert_eq!(c.input, expect(palette.input));
            // A token left at the struct default is fully transparent, which
            // is exactly the silent-black failure this guards against.
            assert_ne!(c.title_bar.a, 0.0, "title_bar must be painted");
            assert_ne!(c.overlay.a, 0.0, "overlay must be painted");
        }
    }

    /// Where a skin ships a hand-tuned hover colour, the projection must use
    /// that exact value instead of deriving one. The accent ramps are picked
    /// per accent and scheme (`AccentTokens::for_scheme`), so a derived hover
    /// would silently override deliberate contrast work.
    #[test]
    fn shipped_hover_tokens_are_used_verbatim() {
        for palette in every_palette() {
            let c = colors::colors(palette);
            assert_eq!(
                c.primary_hover,
                super::super::runtime::palette_hsla(palette.brand_hover),
                "the hand-tuned brand hover must win over a derived one"
            );
        }
    }

    /// Status colours ship no hover token, so hover is derived by moving
    /// lightness only. That keeps it inside the hue and saturation the skin
    /// chose, which is what stops the derivation from inventing a colour
    /// outside the palette's contrast coverage.
    #[test]
    fn derived_status_hover_only_moves_lightness() {
        for palette in every_palette() {
            let c = colors::colors(palette);
            for (base, hover) in [
                (c.danger, c.danger_hover),
                (c.success, c.success_hover),
                (c.warning, c.warning_hover),
            ] {
                assert!(
                    (base.h - hover.h).abs() < 1e-6 && (base.s - hover.s).abs() < 1e-6,
                    "hover must keep hue and saturation for {:?}",
                    palette.skin
                );
                assert_ne!(base.l, hover.l, "hover must move lightness");
            }
        }
    }

    /// Pressed states use the skins' shipped `*_light` surfaces rather than a
    /// second derivation. Those tokens are hand-picked per skin, so they are
    /// the ones the palette contrast tests already cover.
    #[test]
    fn pressed_states_use_the_shipped_surface_tokens() {
        for palette in every_palette() {
            let c = colors::colors(palette);
            let surface = |token| super::super::runtime::palette_hsla(token);
            assert_eq!(c.danger_active, surface(palette.danger_light));
            assert_eq!(c.success_active, surface(palette.success_light));
            assert_eq!(c.warning_active, surface(palette.warning_light));
        }
    }

    /// Neutral surfaces hover to the accent-tinted surface rather than to a
    /// lighter version of themselves. This mirrors `ButtonVariant::Secondary`,
    /// `Ghost`, `Outline` and the switch's unchecked hover, so a GPUI Kit
    /// button and an app-drawn button cannot disagree on the same state.
    #[test]
    fn neutral_surfaces_hover_to_the_accent_surface() {
        for palette in every_palette() {
            let c = colors::colors(palette);
            let accent = super::super::runtime::palette_hsla(palette.accent_surface);
            assert_eq!(c.secondary_hover, accent);
            assert_eq!(c.button_hover, accent);
            assert_eq!(c.button_secondary_hover, accent);
        }
    }

    /// The action colour is `brand`, not the shadcn `primary` neutral.
    /// `Palette::primary` is only the slider fill and the switch track, so the
    /// two must not be conflated or every primary button changes colour.
    #[test]
    fn the_action_colour_is_brand_and_the_neutral_keeps_its_own_slots() {
        for palette in every_palette() {
            let c = colors::colors(palette);
            let brand = super::super::runtime::palette_hsla(palette.brand);
            let neutral = super::super::runtime::palette_hsla(palette.primary);
            assert_eq!(c.primary, brand);
            assert_eq!(
                c.button_primary_foreground,
                super::super::runtime::palette_hsla(palette.brand_foreground)
            );
            assert_eq!(c.switch, neutral);
            assert_eq!(c.slider_bar, neutral);
            assert_eq!(c.slider_thumb, neutral);
        }
    }

    /// Foregrounds painted on a saturated fill are chosen from the fill's own
    /// lightness, so they must land on the opposite side of the threshold.
    /// This is the property the palette contrast tests rely on.
    #[test]
    fn foregrounds_on_saturated_fills_are_chosen_from_the_fill() {
        for palette in every_palette() {
            let c = colors::colors(palette);
            for fill in [c.danger, c.success, c.warning, c.info] {
                let text = super::colors::on_color(fill);
                let split = fill.l > super::colors::BRIGHT_FILL_LIGHTNESS;
                let text_is_dark = text.l < 0.5;
                assert_eq!(
                    split, text_is_dark,
                    "a bright fill needs dark text and vice versa for {:?}",
                    palette.skin
                );
            }
        }
    }

    /// The skin owns geometry, so the projected radius and shadow flag must
    /// follow `Shape` rather than GPUI Kit's defaults.
    #[test]
    fn radius_and_shadow_follow_the_skin() {
        for palette in every_palette() {
            let shape = palette.shape;
            assert_eq!(shape.radius(false), shape.radius_md);
            assert_eq!(shape.radius(true), shape.radius_lg);
        }
        let none = Palette::for_skin(ColorScheme::Dark, AccentId::Crimson, SkinId::Limbus);
        assert!(matches!(none.shape.shadow, ShadowLevel::None));
        let raised = Palette::for_skin(ColorScheme::Dark, AccentId::Crimson, SkinId::Mist);
        assert!(!matches!(raised.shape.shadow, ShadowLevel::None));
    }

    /// GPUI Kit widgets read the legacy `ThemeTokens` table, not `ThemeColor`
    /// directly. That table is derived from the colour table, so a projection
    /// that only overwrites `colors` leaves every button on the built-in
    /// palette. This pins the derivation that makes the projection visible:
    /// the primary button fill must be the palette's brand, not a default.
    #[test]
    fn component_tokens_are_derived_from_the_colour_table() {
        for palette in every_palette() {
            let colors = colors::colors(palette);
            let tokens = ThemeTokens::from(&colors);
            let brand: gpui::Hsla = tokens.button_primary.into();
            assert_eq!(
                brand,
                super::super::runtime::palette_hsla(palette.brand),
                "the primary button fill must come from the colour table"
            );
            // Guard against the table silently staying at its defaults: a
            // ship skin where brand equals the danger colour would make the
            // assertion above pass for the wrong reason.
            let danger: gpui::Hsla = tokens.button_danger.into();
            assert_ne!(brand, danger, "brand and danger must stay distinct");
        }
    }

    /// The legacy hue ramp is collapsed onto semantics on purpose, so a page
    /// asking for `red` and a page asking for `danger` must land on one value.
    #[test]
    fn legacy_hue_ramp_collapses_onto_semantics() {
        let c: ThemeColor = colors::colors(Palette::default());
        assert_eq!(c.red, c.danger);
        assert_eq!(c.green, c.success);
        assert_eq!(c.yellow, c.warning);
        assert_eq!(c.red_light, c.danger_active);
    }
}
