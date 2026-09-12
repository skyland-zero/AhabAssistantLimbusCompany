#![allow(dead_code)]

//! Design tokens shared by the native GPUI controls.
//!
//! The values in this module are the sRGB values obtained from the design
//! token definitions. Keeping the conversion at the boundary means pages
//! never need to approximate an `oklch` color themselves.
//! [`Palette`] is a value object: callers derive a new value from persisted
//! settings instead of mutating a second theme state.

mod palette;
mod runtime;
mod tokens;

#[allow(unused_imports)]
pub use palette::{
    LIMBUS_BAND_BOTTOM, LIMBUS_BAND_BOTTOM_LIGHT, LIMBUS_BAND_TOP, LIMBUS_BAND_TOP_LIGHT, Palette,
    tint,
};
#[allow(unused_imports)]
pub use runtime::{
    ACCENT, BACKGROUND, BORDER, DANGER, FONT_2XS, FONT_LG, FONT_MD, FONT_SM, FONT_XL, FONT_XS,
    GREEN, RADIUS_LG, RADIUS_MD, RADIUS_SM, RADIUS_XL, SPACE_1, SPACE_2, SPACE_3, SPACE_4, SPACE_5,
    SPACE_6, SURFACE, SURFACE_HOVER, ShapeExt, TEXT, TEXT_MUTED, apply_card_shadow,
    current_render_palette, palette_hsla, palette_rgb, render_rgb, render_rgba,
    set_current_render_palette, shape_rounded, skin_rounded,
};
#[allow(unused_imports)]
pub use tokens::{
    ACCENT_PRESETS, AccentId, AccentPreset, AccentTokens, ColorScheme, ColorToken, Decor,
    ShadowLevel, Shape, SkinId,
};

#[cfg(test)]
mod tests;
