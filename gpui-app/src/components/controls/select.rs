use super::*;

/// Select surface. Menu ownership and keyboard navigation stay with the page;
/// this primitive provides the field, focus ring, loading state, and a real
/// Lucide chevron rather than a Unicode glyph.
pub fn select(label: impl Into<String>, options: &[&str], selected: usize) -> Div {
    select_with_palette(
        label,
        options,
        selected,
        &current_render_palette(),
        ControlState::default(),
    )
}

pub fn select_with_palette(
    label: impl Into<String>,
    options: &[&str],
    selected: usize,
    palette: &Palette,
    state: ControlState,
) -> Div {
    let selected = options.get(selected).copied().unwrap_or_default();
    let focus_ring = palette.ring;
    let mut control = div()
        .flex()
        .items_center()
        .justify_between()
        .w_full()
        .px_3()
        .py_2()
        .skin_rounded(palette, false)
        .border_1()
        .border_color(paint_color(palette.input))
        .bg(paint_color(palette.card))
        .text_color(paint_color(palette.foreground));
    if state.focused {
        control = control.border_color(paint_color(palette.ring));
    }
    if state.is_inert() {
        control = control.opacity(0.5);
    } else {
        let hover = paint_color(palette.accent_surface);
        control = control
            .tab_index(0)
            .focus_visible(move |style| style.border_color(paint_color(focus_ring)))
            .cursor_pointer()
            .hover(move |style| style.bg(hover));
    }

    let mut value = div().flex().items_center().gap_2();
    if state.loading {
        value = value.child(icon(
            Icon::LoaderCircle,
            px(14.),
            paint_color(palette.muted_foreground),
        ));
    }
    value = value.child(selected.to_owned());

    control
        .child(
            div()
                .text_color(paint_color(palette.muted_foreground))
                .child(label.into()),
        )
        .child(div().flex().items_center().gap_2().child(value).child(icon(
            Icon::ChevronDown,
            px(14.),
            paint_color(palette.muted_foreground),
        )))
}

/// Render a Select trigger. The owning page controls whether a popup is
/// visible and supplies the option callbacks; keeping that state outside the
/// primitive avoids a second mutable copy of the selected model value.
pub fn select_trigger(label: impl Into<String>, open: bool, palette: &Palette) -> Div {
    let mut trigger = div()
        .flex()
        .items_center()
        .justify_between()
        .w_full()
        .min_w_0()
        .h(px(30.))
        .px_2p5()
        .skin_rounded(palette, false)
        .tab_index(0)
        .border_1()
        .border_color(paint_color(palette.input))
        .bg(paint_color(palette.card))
        .text_size(px(12.))
        .text_color(paint_color(palette.foreground))
        .focus_visible({
            let ring = palette.ring;
            move |style| style.border_color(paint_color(ring))
        });
    let hover = paint_color(palette.accent_surface);
    trigger = trigger.cursor_pointer().hover(move |style| style.bg(hover));
    if open {
        trigger = trigger.border_color(paint_color(palette.ring));
    }
    trigger
        .child(div().min_w_0().truncate().child(label.into()))
        .child(icon(
            Icon::ChevronDown,
            px(13.),
            paint_color(palette.muted_foreground),
        ))
}

/// Paint a Select popup below its trigger. Options are regular GPUI elements
/// so callers can attach click and keyboard listeners without coupling the
/// design system to a particular page state.
pub fn select_popup(options: impl IntoElement, palette: &Palette) -> Div {
    div()
        .absolute()
        .top(px(34.))
        .left_0()
        .right_0()
        .p_1()
        .skin_rounded(palette, false)
        .border_1()
        .border_color(paint_color(palette.input))
        .bg(paint_color(palette.popover))
        .text_color(paint_color(palette.popover_foreground))
        .child(options)
}

/// Paint one Select option. The caller owns its selection and event handler.
pub fn select_option(label: impl Into<String>, selected: bool, palette: &Palette) -> Div {
    let mut option = div()
        .flex()
        .items_center()
        .w_full()
        .min_w_0()
        .min_h(px(28.))
        .px_2()
        .skin_radius_sm(palette)
        .tab_index(0)
        .cursor_pointer()
        .text_size(px(12.))
        .text_color(paint_color(if selected {
            palette.brand
        } else {
            palette.popover_foreground
        }))
        .bg(paint_color(if selected {
            palette.brand_light
        } else {
            palette.popover
        }));
    let hover = paint_color(palette.accent_surface);
    option = option.hover(move |style| style.bg(hover));
    option.child(label.into())
}

/// Resolve the selected option label and the value list for a select control.
///
/// Every page-owned select shares this derivation so the fallback behavior for
/// a missing current value cannot diverge between pages.
pub fn select_options_state(options: &[(String, String)], current: &str) -> (String, Vec<String>) {
    let selected_index = options
        .iter()
        .position(|(value, _)| value == current)
        .unwrap_or(0);
    let selected_label = options
        .get(selected_index)
        .map(|(_, label)| label.clone())
        .unwrap_or_else(|| current.to_owned());
    let values = options
        .iter()
        .map(|(value, _)| value.clone())
        .collect::<Vec<_>>();
    (selected_label, values)
}

/// Compute the next index for a keyboard navigation key.
///
/// Shared by the page select implementations so Enter/Space/arrows/Home/End
/// semantics cannot drift; `None` means the key does not move the selection.
pub fn select_keyboard_index(
    key: &str,
    current_index: usize,
    value_count: usize,
    open: bool,
) -> Option<usize> {
    let modulo = value_count.max(1);
    match key {
        "left" | "arrowleft" | "up" | "arrowup" => {
            Some((current_index as isize - 1).rem_euclid(modulo as isize) as usize)
        }
        "right" | "arrowright" => Some((current_index + 1) % modulo),
        "down" | "arrowdown" if open => Some((current_index + 1) % modulo),
        "down" | "arrowdown" => Some(current_index),
        "home" => Some(0),
        "end" => value_count.checked_sub(1),
        _ => None,
    }
}
