use super::*;

/// A compact on/off control. State changes are supplied by the caller (usually
/// an Entity); this helper only renders the current state.
pub fn switch(checked: bool) -> Div {
    switch_with_palette(checked, &current_render_palette(), ControlState::default())
}

/// Accent-colored switch used by Home's task cards. The default switch keeps
/// the shadcn/Radix primary color so Settings and Theme Packs match the web UI.
pub fn switch_accent(checked: bool) -> Div {
    let palette = current_render_palette();
    switch_with_track(
        checked,
        &palette,
        ControlState::default(),
        palette.brand,
        palette.brand_hover,
    )
}

pub fn switch_with_palette(checked: bool, palette: &Palette, state: ControlState) -> Div {
    switch_with_track(checked, palette, state, palette.primary, palette.primary)
}

fn switch_with_track(
    checked: bool,
    palette: &Palette,
    state: ControlState,
    checked_track: ColorToken,
    checked_hover: ColorToken,
) -> Div {
    let track = if checked {
        checked_track
    } else {
        palette.input
    };
    let thumb = if checked {
        palette.primary_foreground
    } else if matches!(palette.scheme, style::ColorScheme::Dark) {
        palette.foreground
    } else {
        palette.background
    };
    let hover_track = if checked {
        checked_hover
    } else {
        palette.accent_surface
    };
    let focus_ring = palette.ring;

    let mut control = div()
        .flex()
        .items_center()
        .w(px(36.))
        .h(px(20.))
        .p(px(1.))
        .rounded_full()
        .tab_index(0)
        .border_1()
        .border_color(paint_color(palette.border))
        .bg(paint_color(track))
        .focus_visible(move |style| style.border_color(paint_color(focus_ring)));
    if checked {
        control = control.justify_end();
    }
    if state.focused {
        control = control.border_color(paint_color(palette.ring));
    }
    if state.is_inert() {
        control = control.opacity(0.5);
    } else {
        let hover_track = paint_color(hover_track);
        control = control
            .cursor_pointer()
            .hover(move |style| style.bg(hover_track));
    }
    control.child(
        div()
            .w(px(16.))
            .h(px(16.))
            .rounded_full()
            .bg(paint_color(thumb)),
    )
}
