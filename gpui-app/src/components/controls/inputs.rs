use super::*;

/// A visual seam for simple text labels. The entity-backed [`TextInput`] below
/// is the control to use when IME, selection, and clipboard behavior matters.
pub fn text_input(value: &str, placeholder: &str) -> Div {
    text_input_with_palette(
        value,
        placeholder,
        &current_render_palette(),
        ControlState::default(),
    )
}

pub fn text_input_with_palette(
    value: &str,
    placeholder: &str,
    palette: &Palette,
    state: ControlState,
) -> Div {
    let shown = if value.is_empty() { placeholder } else { value };
    let color = if value.is_empty() {
        palette.muted_foreground
    } else {
        palette.foreground
    };
    let mut control = div()
        .w_full()
        .px_3()
        .py_2()
        .rounded_md()
        .border_1()
        .border_color(paint_color(if state.focused {
            palette.ring
        } else {
            palette.input
        }))
        .bg(paint_color(palette.card))
        .text_color(paint_color(color));
    if state.disabled {
        control = control.opacity(0.5);
    }
    control.child(shown.to_owned())
}

pub fn number_stepper(value: i32, min: i32, max: i32) -> Div {
    let value = clamp_number(value, min, max);
    let palette = current_render_palette();
    div()
        .flex()
        .items_center()
        .justify_between()
        .px_2()
        .py_1()
        .rounded_md()
        .border_1()
        .border_color(paint_color(palette.input))
        .bg(paint_color(palette.card))
        .text_color(paint_color(palette.foreground))
        .child("-")
        .child(value.to_string())
        .child("+")
}

pub fn clamp_number(value: i32, min: i32, max: i32) -> i32 {
    if min > max {
        min
    } else {
        value.clamp(min, max)
    }
}
