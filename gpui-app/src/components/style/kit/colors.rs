//! Projection of the app's [`Palette`] onto GPUI Kit's `ThemeColor`.
//!
//! This file is an exhaustive field table rather than a set of behaviours, so
//! it carries the `AGENTS.md` 3.2 exception for a compatibility mapping that
//! cannot be split further: splitting 138 assignments across files would hide
//! which upstream token is still unmapped, which is the only thing this module
//! has to get right.
//!
//! The struct literal below is deliberately **complete**: `ThemeColor` has no
//! `..Default::default()`. When GPUI Kit adds a token, the build fails here
//! instead of silently rendering that surface black.
//!
//! # Mapping rules
//!
//! The app's palette is shadcn-shaped and GPUI Kit's is shadcn-shaped plus a
//! per-variant expansion, so most tokens copy across directly. Three rules
//! cover everything that has no direct counterpart:
//!
//! 1. **Variant states are derived, not invented.** `hover` and `active` move
//!    the base token's lightness away from the background, so a hover state
//!    can never drift into a colour the skin never chose.
//! 2. **Foregrounds on saturated fills pick themselves** via
//!    [`on_color`], which keeps the WCAG AA guarantee the palette tests assert
//!    instead of hardcoding black/white per skin.
//! 3. **The legacy `red`/`green`/`blue`/... ramp collapses onto the semantic
//!    tokens.** Pages select meaning, never a hue, so those names must not
//!    become a second palette.

use gpui::{Hsla, hsla};

use super::super::palette::Palette;
use super::super::runtime::palette_hsla;

/// Lightness above which a fill is bright enough to need dark text.
///
/// Every colour in the shipped skins is a mid-tone or darker, so a single
/// threshold is enough; the palette contrast tests still assert the resulting
/// text/fill pairs, so a future skin with an ambiguous mid-tone fill fails
/// there rather than becoming unreadable here.
pub(super) const BRIGHT_FILL_LIGHTNESS: f32 = 0.55;

pub(super) fn on_color(background: Hsla) -> Hsla {
    if background.l > BRIGHT_FILL_LIGHTNESS {
        hsla(0.0, 0.0, 0.06, 1.0)
    } else {
        hsla(0.0, 0.0, 0.97, 1.0)
    }
}

/// Move a fill's lightness by `delta`, clamped to the valid range.
fn shift(color: Hsla, delta: f32) -> Hsla {
    Hsla {
        l: (color.l + delta).clamp(0.0, 1.0),
        ..color
    }
}

/// Interactive hover state for a fill, moving away from the background.
fn hover_fill(color: Hsla, dark: bool) -> Hsla {
    shift(color, if dark { 0.06 } else { -0.06 })
}

/// Pressed/active state for a fill, further from the background than hover.
fn active_fill(color: Hsla, dark: bool) -> Hsla {
    shift(color, if dark { 0.12 } else { -0.12 })
}

/// Translate one palette into the full GPUI Kit colour table.
pub(super) fn colors(p: Palette) -> gpui_component::ThemeColor {
    use gpui_component::ThemeColor as C;
    let dark = p.is_dark();

    let background = palette_hsla(p.background);
    let foreground = palette_hsla(p.foreground);
    let card = palette_hsla(p.card);
    let card_foreground = palette_hsla(p.card_foreground);
    let popover = palette_hsla(p.popover);
    let popover_foreground = palette_hsla(p.popover_foreground);
    let shadcn_primary = palette_hsla(p.primary);
    let shadcn_primary_foreground = palette_hsla(p.primary_foreground);
    let secondary = palette_hsla(p.secondary);
    let secondary_foreground = palette_hsla(p.secondary_foreground);
    let muted = palette_hsla(p.muted);
    let muted_foreground = palette_hsla(p.muted_foreground);
    let accent = palette_hsla(p.accent_surface);
    let accent_foreground = palette_hsla(p.accent_foreground);
    let border = palette_hsla(p.border);
    let input = palette_hsla(p.input);
    let ring = palette_hsla(p.ring);
    let success = palette_hsla(p.success);
    let success_surface = palette_hsla(p.success_light);
    let warning = palette_hsla(p.warning);
    let warning_surface = palette_hsla(p.warning_light);
    let danger = palette_hsla(p.danger);
    let danger_surface = palette_hsla(p.danger_light);
    let danger_foreground = palette_hsla(p.danger_foreground);
    let brand = palette_hsla(p.brand);
    let brand_hover = palette_hsla(p.brand_hover);
    let brand_surface = palette_hsla(p.brand_light);
    let brand_foreground = palette_hsla(p.brand_foreground);
    let decor = palette_hsla(p.decor_line);
    let selection = palette_hsla(p.selection);
    let overlay = palette_hsla(p.scrim);

    // `Palette::destructive` is the shadcn-compat alias and is not read by any
    // control: `ButtonVariant::Destructive` and `BadgeTone::Danger` both use
    // `danger` / `danger_light` / `danger_foreground`. GPUI Kit has no separate
    // "destructive text" slot, so giving it one here would create a second red
    // the palette contrast tests do not cover. The two agree on every shipped
    // skin except Limbus dark, where `danger` is the deliberately darker fill.
    let success_text = on_color(success);
    let warning_text = on_color(warning);
    let brand_text = palette_hsla(p.brand_foreground);

    C {
        // ---- core surfaces -------------------------------------------------
        background,
        foreground,
        border,
        input,
        ring,
        // Links carry the accent colour; the skins have no separate link hue
        // and inventing one would put them outside the accent system.
        link: brand,
        link_hover: brand_hover,
        link_active: brand_hover,
        // GPUI Kit paints the window edges and the title/status bars from
        // dedicated tokens. The skins use `decor_line` for exactly those
        // hairlines, so they follow it rather than the generic border.
        window_border: decor,
        title_bar: background,
        title_bar_border: decor,
        status_bar: background,
        status_bar_border: decor,
        overlay,
        selection,
        caret: brand,
        // ---- primary / brand ----------------------------------------------
        // GPUI Kit's `primary` drives primary buttons, selected rows and the
        // focus accent. The app calls that colour `brand`: `ButtonVariant::
        // Default` is `brand`/`brand_foreground`/`brand_hover`, and it is the
        // token 30 call sites read. `Palette::primary` is the shadcn neutral
        // that only the slider fill and the switch track use, so it keeps
        // exactly those two slots below instead of becoming the action colour.
        primary: brand,
        primary_foreground: brand_foreground,
        primary_hover: brand_hover,
        primary_active: active_fill(brand, dark),
        secondary,
        secondary_foreground,
        // Hover for neutral surfaces is the accent-tinted surface, matching
        // `ButtonVariant::Secondary`/`Ghost`/`Outline` and the switch's
        // unchecked hover in `controls/switch.rs`.
        secondary_hover: accent,
        secondary_active: active_fill(secondary, dark),
        muted,
        muted_foreground,
        // ---- accents -------------------------------------------------------
        accent,
        accent_foreground,
        // ---- status --------------------------------------------------------
        success,
        success_hover: hover_fill(success, dark),
        success_active: success_surface,
        success_foreground: success_text,
        warning,
        warning_hover: hover_fill(warning, dark),
        warning_active: warning_surface,
        warning_foreground: warning_text,
        danger,
        danger_hover: hover_fill(danger, dark),
        danger_active: danger_surface,
        danger_foreground,
        info: brand,
        info_hover: brand_hover,
        info_active: brand_surface,
        info_foreground: brand_text,
        // ---- buttons -------------------------------------------------------
        button: secondary,
        button_hover: accent,
        button_active: active_fill(secondary, dark),
        button_foreground: secondary_foreground,
        button_primary: brand,
        button_primary_hover: brand_hover,
        button_primary_active: active_fill(brand, dark),
        button_primary_foreground: brand_foreground,
        button_secondary: secondary,
        button_secondary_hover: accent,
        button_secondary_active: active_fill(secondary, dark),
        button_secondary_foreground: secondary_foreground,
        button_danger: danger,
        button_danger_hover: hover_fill(danger, dark),
        button_danger_active: active_fill(danger, dark),
        button_danger_foreground: danger_foreground,
        button_success: success,
        button_success_hover: hover_fill(success, dark),
        button_success_active: active_fill(success, dark),
        button_success_foreground: success_text,
        button_warning: warning,
        button_warning_hover: hover_fill(warning, dark),
        button_warning_active: active_fill(warning, dark),
        button_warning_foreground: warning_text,
        button_info: brand,
        button_info_hover: brand_hover,
        button_info_active: active_fill(brand, dark),
        button_info_foreground: brand_text,
        // ---- list / table surfaces -----------------------------------------
        // The skins treat a card, a list row and a table cell as one surface,
        // so all three families read from the same two tokens.
        list: card,
        list_hover: hover_fill(card, dark),
        list_active: accent,
        list_active_border: ring,
        list_even: card,
        list_head: muted,
        group_box: card,
        group_box_foreground: card_foreground,
        accordion: card,
        table: card,
        table_hover: hover_fill(card, dark),
        table_active: accent,
        table_active_border: ring,
        table_even: card,
        table_head: muted,
        table_head_foreground: muted_foreground,
        table_foot: muted,
        table_foot_foreground: muted_foreground,
        table_row_border: border,
        description_list_label: muted,
        description_list_label_foreground: muted_foreground,
        // ---- popovers, tabs, sidebar ---------------------------------------
        popover,
        popover_foreground,
        tab: background,
        tab_active: card,
        tab_active_foreground: foreground,
        tab_bar: muted,
        tab_bar_segmented: secondary,
        tab_foreground: muted_foreground,
        sidebar: background,
        sidebar_foreground: foreground,
        sidebar_border: border,
        sidebar_accent: accent,
        sidebar_accent_foreground: accent_foreground,
        sidebar_primary: brand,
        sidebar_primary_foreground: brand_foreground,
        // ---- form + scrollbar chrome ---------------------------------------
        // The switch and the slider are the two controls the app keeps on the
        // shadcn neutral instead of the accent (see `controls/switch.rs`,
        // which offers `switch_accent` for the Home cards as an override).
        switch: shadcn_primary,
        switch_thumb: shadcn_primary_foreground,
        slider_bar: shadcn_primary,
        slider_thumb: shadcn_primary,
        progress_bar: brand,
        skeleton: muted,
        scrollbar: muted,
        scrollbar_thumb: border,
        scrollbar_thumb_hover: muted_foreground,
        drag_border: ring,
        drop_target: ring,
        // ---- charts ---------------------------------------------------------
        // The app has no chart page yet; a future one gets skin-following
        // colours here rather than picking from the legacy hue ramp.
        chart_1: brand,
        chart_2: success,
        chart_3: warning,
        chart_4: danger,
        chart_5: brand_surface,
        chart_bullish: success,
        chart_bearish: danger,
        // ---- legacy hue ramp ------------------------------------------------
        // Collapsed onto semantics on purpose: pages select meaning, so these
        // names must not become a second palette that skins cannot control.
        red: danger,
        red_light: danger_surface,
        green: success,
        green_light: success_surface,
        blue: brand,
        blue_light: brand_surface,
        yellow: warning,
        yellow_light: warning_surface,
        magenta: brand,
        magenta_light: brand_surface,
        cyan: brand,
        cyan_light: brand_surface,
    }
}
