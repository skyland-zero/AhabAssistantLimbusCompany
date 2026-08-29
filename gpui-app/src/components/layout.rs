use gpui::{Div, div, prelude::*, px};

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
