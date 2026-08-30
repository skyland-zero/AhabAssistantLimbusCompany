use gpui::{Context, Div, Window, WindowControlArea, div, img, prelude::*, px, rgb, rgba};

use crate::{
    app::{AhabApp, Page},
    assets,
    components::style::Palette,
    i18n::{self, Key as I18nKey},
    model::{Language, ThemeMode},
    shell::ToastKind,
};

mod icons;

use icons::{Icon, icon};

const TITLEBAR_HEIGHT: f32 = 40.0;
const WINDOW_BUTTON_WIDTH: f32 = 48.0;

const NAV_PAGES: [Page; 6] = [
    Page::Home,
    Page::Teams,
    Page::ThemePacks,
    Page::Toolbox,
    Page::Resources,
    Page::Help,
];

pub fn title_bar(
    window: &Window,
    active_page: Page,
    app: &mut AhabApp,
    palette: Palette,
    cx: &mut Context<AhabApp>,
) -> Div {
    let language = app.state.settings.language;

    let mut brand = div()
        .id("titlebar-brand")
        .flex()
        .items_center()
        .gap_2()
        .pl_3()
        .pr_3()
        .h_full()
        .window_control_area(WindowControlArea::Drag)
        .child(
            img(assets::image_source(assets::title_banner()))
                .w(px(38.))
                .h(px(20.))
                .flex_none(),
        )
        .child(
            div()
                .text_size(px(12.))
                .font_weight(gpui::FontWeight::SEMIBOLD)
                .text_color(rgb(palette.foreground.rgb_hex()))
                .child(i18n::text(language, I18nKey::TitlebarTitle)),
        );

    if !cfg!(target_os = "windows") {
        brand = brand.on_click(|event, window, _| {
            if event.click_count() == 2 {
                window.zoom_window();
            }
        });
    }

    let is_busy = app.home.is_busy();
    let is_paused = app.home.execution.state == crate::model::ExecutionState::Paused;

    let mut pages = div().flex().items_center().gap_1();
    for (index, page) in NAV_PAGES.into_iter().enumerate() {
        pages = pages.child(nav_item(
            NavItemConfig {
                page,
                active: active_page == page,
                tab_index: index as isize,
                language,
                palette,
                is_busy,
                is_paused,
            },
            cx,
        ));
    }

    let mut drag_spacer = div()
        .id("titlebar-drag-spacer")
        .flex_1()
        .h_full()
        .window_control_area(WindowControlArea::Drag);

    if !cfg!(target_os = "windows") {
        drag_spacer = drag_spacer.on_click(|event, window, _| {
            if event.click_count() == 2 {
                window.zoom_window();
            }
        });
    }

    let theme_mode = app.state.settings.themeMode;
    let (theme_icon, theme_key) = match theme_mode {
        ThemeMode::Light => (Icon::Sun, I18nKey::ThemeLight),
        ThemeMode::Dark => (Icon::Moon, I18nKey::ThemeDark),
        ThemeMode::System => (Icon::Monitor, I18nKey::ThemeSystem),
    };
    let theme_label = i18n::text(language, theme_key);
    let mut theme_button = utility_button("titlebar-theme-toggle", theme_label, 6, palette);
    theme_button = theme_button
        .child(icon(theme_icon, 14.).text_color(rgb(palette.muted_foreground.rgb_hex())));
    theme_button = theme_button.on_click(cx.listener(move |view, _, _, cx| {
        let next_mode = match view.state.settings.themeMode {
            ThemeMode::Light => ThemeMode::Dark,
            ThemeMode::Dark => ThemeMode::System,
            ThemeMode::System => ThemeMode::Light,
        };
        let next_key = match next_mode {
            ThemeMode::Light => I18nKey::ThemeLight,
            ThemeMode::Dark => I18nKey::ThemeDark,
            ThemeMode::System => I18nKey::ThemeSystem,
        };
        let message = i18n::text(view.state.settings.language, next_key).to_owned();
        view.set_theme_mode(next_mode);
        view.show_toast(ToastKind::Info, message, cx);
        cx.notify();
    }));

    let settings_active = active_page == Page::Settings;
    let mut settings_button = utility_button(
        "titlebar-settings",
        i18n::text(language, I18nKey::NavSettings),
        7,
        palette,
    );
    settings_button = settings_button.child(icon(Icon::Settings, 14.).text_color(rgb(
        if settings_active {
            palette.brand.rgb_hex()
        } else {
            palette.muted_foreground.rgb_hex()
        },
    )));
    if settings_active {
        settings_button = settings_button
            .bg(rgb(palette.secondary.rgb_hex()))
            .text_color(rgb(palette.brand.rgb_hex()));
    }
    settings_button = settings_button.on_click(cx.listener(|view, _, window, cx| {
        view.select_page(Page::Settings, window, cx);
    }));

    let utilities = div()
        .flex()
        .items_center()
        .gap_1()
        .pr_2()
        .child(theme_button)
        .child(settings_button);

    let controls = div()
        .id("titlebar-window-controls")
        .h_full()
        .flex()
        .children([
            window_button(WindowButtonConfig {
                id: "titlebar-minimize",
                label: i18n::text(language, I18nKey::TitlebarMinimize),
                icon_kind: Icon::Minus,
                area: WindowControlArea::Min,
                enabled: window.is_minimizable(),
                platform_supported: window.window_controls().minimize,
                danger: false,
                palette,
            }),
            window_button(WindowButtonConfig {
                id: "titlebar-maximize",
                label: if window.is_maximized() {
                    i18n::text(language, I18nKey::TitlebarRestore)
                } else {
                    i18n::text(language, I18nKey::TitlebarMaximize)
                },
                icon_kind: if window.is_maximized() {
                    Icon::Restore
                } else {
                    Icon::Square
                },
                area: WindowControlArea::Max,
                enabled: window.is_resizable(),
                platform_supported: window.window_controls().maximize,
                danger: false,
                palette,
            }),
            window_button(WindowButtonConfig {
                id: "titlebar-close",
                label: i18n::text(language, I18nKey::TitlebarClose),
                icon_kind: Icon::Close,
                area: WindowControlArea::Close,
                enabled: true,
                platform_supported: true,
                danger: true,
                palette,
            }),
        ]);

    div()
        .h(px(TITLEBAR_HEIGHT))
        .flex_none()
        .flex()
        .items_center()
        .border_b_1()
        .border_color(rgba(titlebar_separator_hex(palette)))
        .bg(rgb(palette.card.rgb_hex()))
        .child(brand)
        .child(pages)
        .child(drag_spacer)
        .child(utilities)
        .child(controls)
}

fn titlebar_separator_hex(palette: Palette) -> u32 {
    let alpha = u32::from(palette.input.alpha()) * 3 / 5;
    (palette.input.rgb_hex() << 8) | alpha
}

struct NavItemConfig {
    page: Page,
    active: bool,
    tab_index: isize,
    language: Language,
    palette: Palette,
    is_busy: bool,
    is_paused: bool,
}

fn nav_item(config: NavItemConfig, cx: &mut Context<AhabApp>) -> impl IntoElement {
    let NavItemConfig {
        page,
        active,
        tab_index,
        language,
        palette,
        is_busy,
        is_paused,
    } = config;
    let mut item = div()
        .id(format!("tab-{:?}", page))
        .tab_index(tab_index)
        .aria_label(page.label_for(language))
        .flex()
        .items_center()
        .gap(px(6.))
        .rounded_md()
        .px(px(8.))
        .py(px(4.))
        .text_size(px(12.))
        .cursor_pointer()
        .focus_visible(|style| style.border_1().border_color(rgb(palette.ring.rgb_hex())))
        .active(|style| style.bg(rgb(palette.secondary.rgb_hex())))
        .on_click(cx.listener(move |view, _, window, cx| {
            view.select_page(page, window, cx);
        }))
        .child(icon(page_icon(page), 13.).text_color(rgb(if active {
            palette.brand_foreground.rgb_hex()
        } else {
            palette.muted_foreground.rgb_hex()
        })))
        .child(page.label_for(language));

    if page == Page::Home {
        let dot_color = if is_busy {
            if is_paused {
                rgb(palette.warning.rgb_hex())
            } else {
                rgb(palette.success.rgb_hex())
            }
        } else {
            rgba(0)
        };
        item = item.child(
            div()
                .w(px(6.))
                .h(px(6.))
                .rounded_full()
                .bg(dot_color)
                .flex_none(),
        );
    }

    if active {
        item = item
            .bg(rgb(palette.brand.rgb_hex()))
            .text_color(rgb(palette.brand_foreground.rgb_hex()));
    } else {
        item = item
            .text_color(rgb(palette.muted_foreground.rgb_hex()))
            .hover(|style| {
                style
                    .bg(rgb(palette.secondary.rgb_hex()))
                    .text_color(rgb(palette.foreground.rgb_hex()))
            });
    }

    item
}

fn utility_button(
    id: &'static str,
    label: &'static str,
    tab_index: isize,
    palette: Palette,
) -> gpui::Stateful<Div> {
    div()
        .id(id)
        .tab_index(tab_index)
        .aria_label(label)
        .w(px(28.))
        .h(px(28.))
        .flex()
        .items_center()
        .justify_center()
        .rounded_md()
        .cursor_pointer()
        .text_color(rgb(palette.muted_foreground.rgb_hex()))
        .focus_visible(|style| style.border_1().border_color(rgb(palette.ring.rgb_hex())))
        .hover(|style| style.text_color(rgb(palette.foreground.rgb_hex())))
}

struct WindowButtonConfig {
    id: &'static str,
    label: &'static str,
    icon_kind: Icon,
    area: WindowControlArea,
    enabled: bool,
    platform_supported: bool,
    danger: bool,
    palette: Palette,
}

fn window_button(config: WindowButtonConfig) -> impl IntoElement {
    let WindowButtonConfig {
        id,
        label,
        icon_kind,
        area,
        enabled,
        platform_supported,
        danger,
        palette,
    } = config;
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

fn page_icon(page: Page) -> Icon {
    match page {
        Page::Home => Icon::Home,
        Page::Teams => Icon::Users,
        Page::ThemePacks => Icon::Palette,
        Page::Toolbox => Icon::Wrench,
        Page::Resources => Icon::Package,
        Page::Help => Icon::Help,
        Page::Settings => Icon::Settings,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::components::style::AccentId;

    #[test]
    fn primary_navigation_contains_six_pages_and_settings_is_secondary() {
        assert_eq!(NAV_PAGES.len(), 6);
        assert!(!NAV_PAGES.contains(&Page::Settings));
        assert_eq!(Page::ALL.len(), 7);
        assert!(Page::ALL.contains(&Page::Settings));
    }

    #[test]
    fn all_page_icons_are_defined() {
        for page in Page::ALL {
            let _ = page_icon(page);
        }
    }

    #[test]
    fn titlebar_separator_reduces_theme_input_contrast() {
        let light = titlebar_separator_hex(Palette::light(AccentId::Crimson));
        let dark = titlebar_separator_hex(Palette::dark(AccentId::Crimson));

        assert_eq!(light & 0xff, 0x99);
        assert_eq!(dark & 0xff, 0x16);
    }
}
