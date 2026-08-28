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
        .rounded_md()
        .tab_index(0)
        .border_1()
        .border_color(paint_color(palette.input))
        .bg(paint_color(palette.card))
        .text_color(paint_color(palette.foreground))
        .focus_visible(move |style| style.border_color(paint_color(focus_ring)));
    if state.focused {
        control = control.border_color(paint_color(palette.ring));
    }
    if state.is_inert() {
        control = control.opacity(0.5);
    } else {
        let hover = paint_color(palette.accent_surface);
        control = control.cursor_pointer().hover(move |style| style.bg(hover));
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
        .rounded_md()
        .tab_index(0)
        .border_1()
        .border_color(paint_color(palette.input))
        .bg(paint_color(palette.card))
        .text_size(px(13.))
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
        .rounded_md()
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
        .rounded_sm()
        .tab_index(0)
        .cursor_pointer()
        .text_size(px(13.))
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
