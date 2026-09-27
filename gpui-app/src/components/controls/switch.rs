use gpui::ElementId;
use gpui_component::Disableable as _;
pub use gpui_component::switch::Switch;

use super::*;

/// A compact on/off control.
///
/// State changes are supplied by the caller (usually an `Entity`); this helper
/// only renders the current state. GPUI Kit's switch routes both pointer and
/// keyboard activation through one `on_change` callback, so call sites no
/// longer hand-roll an `on_key_down` next to every `on_click`.
pub fn switch(id: impl Into<ElementId>, checked: bool) -> Switch {
    let palette = current_render_palette();
    // GPUI Kit draws the checked track from the theme's `primary`, which the
    // bridge maps to the accent. The default switch stays on the shadcn/Radix
    // neutral so Settings and Theme Packs match the web UI, so it overrides the
    // fill explicitly instead of inheriting the accent.
    switch_with_palette(id, checked, &palette, ControlState::default())
        .color(palette_hsla(palette.primary))
}

/// Accent-coloured switch used by Home's task cards.
///
/// The default switch keeps the shadcn/Radix primary colour so Settings and
/// Theme Packs match the web UI; this variant overrides the checked fill with
/// the skin's accent.
pub fn switch_accent(id: impl Into<ElementId>, checked: bool) -> Switch {
    let palette = current_render_palette();
    switch_with_palette(id, checked, &palette, ControlState::default())
        .color(palette_hsla(palette.brand))
}

pub fn switch_with_palette(
    id: impl Into<ElementId>,
    checked: bool,
    _palette: &Palette,
    state: ControlState,
) -> Switch {
    // The track and thumb colours come from the projected theme: the unchecked
    // track is `switch` and the thumb is `switch_thumb`. The checked track is
    // `theme.primary` unless the caller overrides it with `.color(...)`.
    let control = Switch::new(id).checked(checked).disabled(state.is_inert());
    if state.is_inert() {
        control.opacity(0.5)
    } else {
        control
    }
}
