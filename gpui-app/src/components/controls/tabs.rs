use super::*;

pub fn tabs(labels: &[&str], selected: usize) -> Div {
    tabs_with_palette(labels, selected, &current_render_palette(), &[])
}

/// Paint the shared tab surface without taking ownership of its label or
/// interaction. Pages can attach their own ids, badges, click handlers, and
/// keyboard behavior while keeping the same visual language as the common
/// tabs control.
pub fn tab_surface_with_palette(active: bool, palette: &Palette) -> Div {
    let focus_ring = palette.ring;
    div()
        .flex()
        .items_center()
        .justify_center()
        .h(px(28.))
        .px_3()
        .rounded_md()
        .tab_index(0)
        .text_size(px(12.))
        .font_weight(if active {
            gpui::FontWeight::MEDIUM
        } else {
            gpui::FontWeight::NORMAL
        })
        .border_1()
        .border_color(paint_color(if active {
            palette.brand
        } else {
            palette.border
        }))
        .text_color(paint_color(if active {
            palette.brand
        } else {
            palette.muted_foreground
        }))
        .bg(paint_color(if active {
            palette.brand_light
        } else {
            palette.card
        }))
        .focus_visible(move |style| style.border_color(paint_color(focus_ring)))
}

/// Render tabs with per-tab disabled state. An empty `disabled` slice keeps
/// every tab enabled for the legacy constructor.
pub fn tabs_with_palette(
    labels: &[&str],
    selected: usize,
    palette: &Palette,
    disabled: &[bool],
) -> Div {
    let mut root = div().flex().items_center().gap_1();
    for (index, label) in labels.iter().enumerate() {
        let active = index == selected;
        let is_disabled = disabled.get(index).copied().unwrap_or(false);
        let mut tab = tab_surface_with_palette(active, palette)
            .id(format!("tab-{index}"))
            .tab_index(index as isize);
        if is_disabled {
            tab = tab.opacity(0.5);
        } else {
            let hover = paint_color(palette.accent_surface);
            let foreground = paint_color(palette.foreground);
            tab = tab
                .cursor_pointer()
                .hover(move |style| style.bg(hover).text_color(foreground));
        }
        tab = tab.child((*label).to_owned());
        root = root.child(tab);
    }
    root
}
