use super::*;

pub fn tabs(labels: &[&str], selected: usize) -> Div {
    tabs_with_palette(labels, selected, &current_render_palette(), &[])
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
        let focus_ring = palette.ring;
        let mut tab = div()
            .id(format!("tab-{index}"))
            .tab_index(index as isize)
            .px_3()
            .py_2()
            .rounded_md()
            .border_1()
            .border_color(paint_color(if active {
                palette.brand
            } else {
                palette.border
            }))
            .text_size(px(12.))
            .text_color(paint_color(if active {
                palette.foreground
            } else {
                palette.muted_foreground
            }))
            .bg(paint_color(if active {
                palette.brand_light
            } else {
                palette.card
            }))
            .focus_visible(move |style| style.border_color(paint_color(focus_ring)));
        if is_disabled {
            tab = tab.opacity(0.5);
        } else {
            let hover = paint_color(palette.accent_surface);
            tab = tab.cursor_pointer().hover(move |style| style.bg(hover));
        }
        tab = tab.child((*label).to_owned());
        root = root.child(tab);
    }
    root
}
