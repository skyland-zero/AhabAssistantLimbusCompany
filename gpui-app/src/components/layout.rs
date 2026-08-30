use gpui::{Div, div, prelude::*, px};

use super::{card, current_render_palette, palette_rgb};

/// Shared outer surface for top-level pages.
///
/// Home has its own split-pane layout, while the remaining pages use this
/// page frame so their background, padding, and vertical rhythm stay aligned
/// with Home's cards.
pub fn page_root() -> Div {
    div()
        .size_full()
        .min_w_0()
        .min_h_0()
        .flex()
        .flex_col()
        // Keep every standalone page aligned to the console's content rhythm
        // with a symmetric 10px outer inset.
        .gap_2()
        .pl(px(10.0))
        .pr(px(10.0))
        .pt(px(10.0))
        .pb(px(10.0))
        .bg(palette_rgb(current_render_palette().background))
}

/// Shared card surface for page-level action bars.
pub fn page_toolbar(child: impl IntoElement) -> Div {
    card(child).w_full().p_3()
}

/// Wraps children into responsive columns without needing a window-size
/// breakpoint. Each child grows to use the available row and drops to the
/// next row when the requested minimum width cannot be met.
pub fn settings_grid(children: impl IntoIterator<Item = Div>, min_width: f32) -> Div {
    let mut grid = div().flex().flex_wrap().gap_2();
    for child in children {
        grid = grid.child(div().flex_1().min_w(px(min_width)).child(child));
    }
    grid
}
