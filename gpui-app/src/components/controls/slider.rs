use super::*;

/// Render a bounded slider. Use [`normalize_slider`] for the model value
/// before passing it here. The knob is deliberately static; callers provide
/// pointer/keyboard updates to the canonical model.
pub fn slider(value: f32, min: f32, max: f32) -> Div {
    slider_with_palette(
        value,
        min,
        max,
        &current_render_palette(),
        ControlState::default(),
    )
}

pub fn slider_with_palette(
    value: f32,
    min: f32,
    max: f32,
    palette: &Palette,
    state: ControlState,
) -> Div {
    let normalized = normalize_slider(value, min, max);
    let width = 180.0;
    let track_y = 5.0;
    let knob_left = (width * normalized - 8.0).clamp(0.0, width - 16.0);
    let mut track = div()
        .relative()
        .w(px(width))
        .h(px(16.))
        .rounded_md()
        .tab_index(0)
        .focus_visible({
            let ring = palette.ring;
            move |style| style.border_color(paint_color(ring))
        })
        .child(
            div()
                .absolute()
                .left_0()
                .top(px(track_y))
                .w_full()
                .h(px(6.))
                .rounded_md()
                .bg(paint_color(palette.muted)),
        )
        .child(
            div()
                .absolute()
                .left_0()
                .top(px(track_y))
                .w(px(width * normalized))
                .h(px(6.))
                .rounded_md()
                .bg(paint_color(palette.primary)),
        )
        .child(
            div()
                .absolute()
                .left(px(knob_left))
                .top_0()
                .w(px(16.))
                .h(px(16.))
                .rounded_full()
                .border_1()
                .border_color(paint_color(palette.primary))
                .bg(paint_color(ColorToken::rgb(0xffffff)))
                .shadow_sm(),
        );
    if state.focused {
        track = track.border_1().border_color(paint_color(palette.ring));
    }
    if state.is_inert() {
        track = track.opacity(0.5);
    }
    track
}

pub fn normalize_slider(value: f32, min: f32, max: f32) -> f32 {
    if !value.is_finite() || !min.is_finite() || !max.is_finite() || max <= min {
        return 0.0;
    }
    ((value - min) / (max - min)).clamp(0.0, 1.0)
}
