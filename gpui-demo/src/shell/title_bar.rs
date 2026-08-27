use gpui::{Div, Window, WindowControlArea, div, img, prelude::*, px, rgb, rgba};

use super::tab_bar::{Icon, icon};
use crate::{
    assets,
    components::style::Palette,
    i18n::{self, Key as I18nKey},
    model::Language,
};

const TITLEBAR_HEIGHT: f32 = 40.0;
const WINDOW_BUTTON_WIDTH: f32 = 48.0;

pub fn title_bar(window: &Window, language: Language, palette: Palette) -> Div {
    let mut drag_region = div()
        .id("titlebar-drag-region")
        .flex_1()
        .h_full()
        .flex()
        .items_center()
        .gap_2()
        .pl_3()
        .window_control_area(WindowControlArea::Drag)
        .child(
            img(assets::image_source(assets::title_banner()))
                .w(px(41.))
                .h(px(22.))
                .flex_none(),
        )
        .child(
            div()
                .text_size(px(11.))
                .text_color(rgb(palette.muted_foreground.rgb_hex()))
                .child(format!(
                    "· {}",
                    i18n::text(language, I18nKey::TitlebarTitle)
                )),
        );

    // Windows performs dragging and titlebar double-click handling from the
    // WindowControlArea hit test. Other platforms need the GPUI zoom fallback.
    if !cfg!(target_os = "windows") {
        drag_region = drag_region.on_click(|event, window, _| {
            if event.click_count() == 2 {
                window.zoom_window();
            }
        });
    }

    let controls = div()
        .id("titlebar-window-controls")
        .h_full()
        .flex()
        .children([
            window_button(
                "titlebar-minimize",
                i18n::text(language, I18nKey::TitlebarMinimize),
                Icon::Minus,
                WindowControlArea::Min,
                window.is_minimizable(),
                window.window_controls().minimize,
                false,
                palette,
            ),
            window_button(
                "titlebar-maximize",
                if window.is_maximized() {
                    i18n::text(language, I18nKey::TitlebarRestore)
                } else {
                    i18n::text(language, I18nKey::TitlebarMaximize)
                },
                if window.is_maximized() {
                    Icon::Restore
                } else {
                    Icon::Square
                },
                WindowControlArea::Max,
                window.is_resizable(),
                window.window_controls().maximize,
                false,
                palette,
            ),
            window_button(
                "titlebar-close",
                i18n::text(language, I18nKey::TitlebarClose),
                Icon::Close,
                WindowControlArea::Close,
                true,
                true,
                true,
                palette,
            ),
        ]);

    div()
        .h(px(TITLEBAR_HEIGHT))
        .flex_none()
        .flex()
        .items_center()
        .border_b_1()
        .border_color(rgba(palette.input.rgba_hex()))
        .bg(rgb(palette.card.rgb_hex()))
        .child(drag_region)
        .child(controls)
}

fn window_button(
    id: &'static str,
    label: &'static str,
    icon_kind: Icon,
    area: WindowControlArea,
    enabled: bool,
    platform_supported: bool,
    danger: bool,
    palette: Palette,
) -> impl IntoElement {
    let mut button = div()
        .id(id)
        .aria_label(label)
        .w(px(WINDOW_BUTTON_WIDTH))
        .h_full()
        .flex()
        .items_center()
        .justify_center()
        .text_color(rgb(if enabled {
            palette.muted_foreground.rgb_hex()
        } else {
            palette.input.rgb_hex()
        }))
        .window_control_area(area);

    if enabled && platform_supported {
        button = button.hover(|style| {
            if danger {
                style
                    .bg(rgb(palette.danger.rgb_hex()))
                    .text_color(rgb(palette.foreground.rgb_hex()))
            } else {
                style
                    .bg(rgb(palette.secondary.rgb_hex()))
                    .text_color(rgb(palette.foreground.rgb_hex()))
            }
        });
        button = button.active(|style| {
            if danger {
                style
                    .bg(rgb(palette.danger.rgb_hex()))
                    .text_color(rgb(palette.muted_foreground.rgb_hex()))
            } else {
                style
                    .bg(rgb(palette.secondary.rgb_hex()))
                    .text_color(rgb(palette.muted_foreground.rgb_hex()))
            }
        });
        button = button
            .focus_visible(|style| style.border_1().border_color(rgb(palette.ring.rgb_hex())));

        // Windows consumes these hit-test areas natively. On platforms where
        // the platform window does not consume them, keep the controls usable
        // through ordinary GPUI click events without double dispatch on Windows.
        if !cfg!(target_os = "windows") {
            button = match area {
                WindowControlArea::Min => button.on_click(|_, window, _| window.minimize_window()),
                WindowControlArea::Max => button.on_click(|_, window, _| window.zoom_window()),
                WindowControlArea::Close => button.on_click(|_, window, _| window.remove_window()),
                WindowControlArea::Drag => button,
            };
        }
    }

    button.child(icon(icon_kind, 16.).text_color(rgb(if enabled {
        palette.muted_foreground.rgb_hex()
    } else {
        palette.input.rgb_hex()
    })))
}
