use gpui::{Div, div, prelude::*, px};

use crate::components::style::Palette;

#[allow(dead_code)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ToastKind {
    Success,
    Info,
    Warning,
    Error,
    Loading,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Toast {
    pub id: u64,
    pub kind: ToastKind,
    pub message: String,
}

pub fn toast_layer(toast: Option<&Toast>, palette: Palette) -> Div {
    let Some(toast) = toast else {
        return div();
    };

    let (background, foreground) = match toast.kind {
        ToastKind::Success => (palette.success_light, palette.success),
        ToastKind::Info | ToastKind::Loading => (palette.brand_light, palette.brand),
        ToastKind::Warning => (palette.warning_light, palette.warning),
        ToastKind::Error => (palette.danger_light, palette.danger),
    };

    div()
        .absolute()
        .top(px(80.))
        .left_0()
        .right_0()
        .flex()
        .justify_center()
        .child(
            div()
                .max_w(px(520.))
                .px_4()
                .py_2()
                .rounded_lg()
                .border_1()
                .border_color(gpui::rgba((foreground.rgb_hex() << 8) | 0x66))
                .bg(gpui::rgba((background.rgb_hex() << 8) | 0xf2))
                .text_size(px(12.))
                .text_color(gpui::rgba(foreground.rgba_hex()))
                .child(toast.message.clone()),
        )
}
