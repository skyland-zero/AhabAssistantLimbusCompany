use gpui::{Div, div, prelude::*, px};

use super::card;

/// Shared outer surface for top-level pages.
///
/// Deliberately paints **no** background: the root window `Div` already owns
/// `palette.background`, and the skin's background artwork layer is inserted
/// between the two. An opaque page root used to sit on top of that layer and
/// hid it completely, so the limbus nebula plate had never actually been
/// visible.
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
