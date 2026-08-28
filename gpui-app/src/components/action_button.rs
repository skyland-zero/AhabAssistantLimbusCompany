use gpui::{Div, Svg, prelude::*, px};

use super::{ButtonVariant, button};

/// Compact action button shared by page toolbars and card actions.
pub fn action_button(
    label: impl Into<String>,
    variant: ButtonVariant,
    icon: Option<Svg>,
    height: f32,
) -> Div {
    let mut control = button("", variant)
        .h(px(height))
        .px_3()
        .py_0()
        .text_size(px(12.));
    if let Some(icon) = icon {
        control = control.child(icon);
    }
    control.child(label.into())
}
