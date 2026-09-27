use gpui::{ElementId, SharedString, Svg, prelude::*, px};

use gpui_component::button::Button;

use super::{ButtonVariant, button};

/// Compact action button shared by page toolbars and card actions.
///
/// `id` is forwarded to [`Button`] because GPUI Kit keys the focus handle by
/// it; it must be unique within the window, not just within the parent.
pub fn action_button(
    id: impl Into<ElementId>,
    label: impl Into<SharedString>,
    variant: ButtonVariant,
    icon: Option<Svg>,
    height: f32,
) -> Button {
    let mut control = button(id, label, variant)
        .h(px(height))
        .px_3()
        .py_0()
        .text_size(px(12.));
    if let Some(icon) = icon {
        control = control.child(icon);
    }
    control
}
