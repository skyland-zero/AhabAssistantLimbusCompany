use gpui::{Context, Div, Svg, div, prelude::*, px, rgb, rgba, svg};

use crate::{
    app::{AhabApp, Page},
    components::style::Palette,
    i18n::{self, Key as I18nKey},
    model::{Language, ThemeMode},
    shell::ToastKind,
};

const NAV_PAGES: [Page; 6] = [
    Page::Home,
    Page::Teams,
    Page::ThemePacks,
    Page::Toolbox,
    Page::Resources,
    Page::Help,
];

/// The monochrome SVGs keep shell controls independent from a second icon
/// dependency while preserving the Lucide-style line icon shape used by `ui`.
#[derive(Clone, Copy)]
#[allow(dead_code)]
pub(super) enum Icon {
    Home,
    Users,
    Palette,
    Wrench,
    Package,
    Help,
    Sun,
    Moon,
    Monitor,
    Settings,
    Minus,
    Square,
    Restore,
    Close,
}

pub(super) fn icon(kind: Icon, size: f32) -> Svg {
    let data: &'static [u8] = match kind {
        Icon::Home => br#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="m3 9 9-7 9 7v11a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z"/><polyline points="9 22 9 12 15 12 15 22"/></svg>"#,
        Icon::Users => br#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M16 21v-2a4 4 0 0 0-4-4H6a4 4 0 0 0-4 4v2"/><circle cx="9" cy="7" r="4"/><path d="M22 21v-2a4 4 0 0 0-3-3.87M16 3.13a4 4 0 0 1 0 7.75"/></svg>"#,
        Icon::Palette => br#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="13.5" cy="6.5" r=".5"/><circle cx="17.5" cy="10.5" r=".5"/><circle cx="8.5" cy="7.5" r=".5"/><circle cx="6.5" cy="12.5" r=".5"/><path d="M12 2a10 10 0 0 0-7.35 16.77A4 4 0 0 0 8 20h1a2 2 0 0 0 2-2v-.5a2.5 2.5 0 0 1 2.5-2.5H15a7 7 0 0 0 7-7 6 6 0 0 0-1.12-3.5A10 10 0 0 0 12 2z"/></svg>"#,
        Icon::Wrench => br#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M14.7 6.3a1 1 0 0 0 0 1.4l1.6 1.6a1 1 0 0 0 1.4 0l3.77-3.77a6 6 0 0 1-7.94 7.94l-6.91 6.91a2.12 2.12 0 0 1-3-3l6.91-6.91a6 6 0 0 1 7.94-7.94z"/></svg>"#,
        Icon::Package => br#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="m16.5 9.4-9-5.19M21 16V8a2 2 0 0 0-1-1.73l-7-4a2 2 0 0 0-2 0l-7 4A2 2 0 0 0 3 8v8a2 2 0 0 0 1 1.73l7 4a2 2 0 0 0 2 0l7-4A2 2 0 0 0 21 16z"/><polyline points="3.27 6.96 12 12.01 20.73 6.96"/><line x1="12" y1="22.08" x2="12" y2="12"/></svg>"#,
        Icon::Help => br#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="10"/><path d="M9.09 9a3 3 0 0 1 5.83 1c0 2-3 2-3 4"/><line x1="12" y1="17" x2="12.01" y2="17"/></svg>"#,
        Icon::Sun => br#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="4"/><path d="M12 2v2M12 20v2M4.93 4.93l1.41 1.41M17.66 17.66l1.41 1.41M2 12h2M20 12h2M6.34 17.66l-1.41 1.41M19.07 4.93l-1.41 1.41"/></svg>"#,
        Icon::Moon => br#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M21 12.79A9 9 0 1 1 11.21 3 7 7 0 0 0 21 12.79z"/></svg>"#,
        Icon::Monitor => br#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect x="2" y="3" width="20" height="14" rx="2" ry="2"/><line x1="8" y1="21" x2="16" y2="21"/><line x1="12" y1="17" x2="12" y2="21"/></svg>"#,
        Icon::Settings => br#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="3"/><path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 1 1-2.83 2.83l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 1 1-4 0v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 1 1-2.83-2.83l.06-.06A1.65 1.65 0 0 0 4.6 15a1.65 1.65 0 0 0-1.51-1H3a2 2 0 1 1 0-4h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 1 1 2.83-2.83l.06.06A1.65 1.65 0 0 0 9 4.6h.01A1.65 1.65 0 0 0 10 3.09V3a2 2 0 1 1 4 0v.09A1.65 1.65 0 0 0 15 4.6a1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 1 1 2.83 2.83l-.06.06A1.65 1.65 0 0 0 19.4 9v.01A1.65 1.65 0 0 0 20.91 10H21a2 2 0 1 1 0 4h-.09A1.65 1.65 0 0 0 19.4 15z"/></svg>"#,
        Icon::Minus => br#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round"><line x1="5" y1="12" x2="19" y2="12"/></svg>"#,
        Icon::Square => br#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linejoin="round"><rect x="5" y="5" width="14" height="14" rx="1"/></svg>"#,
        Icon::Restore => br#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polyline points="8 8 8 3 13 3"/><path d="M8 3a9 9 0 1 0 8.66 6.5"/><rect x="10" y="10" width="10" height="10" rx="1"/></svg>"#,
        Icon::Close => br#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round"><line x1="18" y1="6" x2="6" y2="18"/><line x1="6" y1="6" x2="18" y2="18"/></svg>"#,
    };

    svg().data(data).size(px(size)).flex_none()
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

pub fn tab_bar(
    active_page: Page,
    app: &mut AhabApp,
    palette: Palette,
    cx: &mut Context<AhabApp>,
) -> Div {
    let language = app.state.settings.language;
    let mut pages = div().flex().items_center().gap_1();
    for (index, page) in NAV_PAGES.into_iter().enumerate() {
        pages = pages.child(nav_item(
            page,
            active_page == page,
            index as isize,
            language,
            palette,
            cx,
        ));
    }

    let theme_mode = app.state.settings.themeMode;
    let (theme_icon, theme_key) = match theme_mode {
        ThemeMode::Light => (Icon::Sun, I18nKey::ThemeLight),
        ThemeMode::Dark => (Icon::Moon, I18nKey::ThemeDark),
        ThemeMode::System => (Icon::Monitor, I18nKey::ThemeSystem),
    };
    let theme_label = i18n::text(language, theme_key);
    let mut theme_button = utility_button("tab-theme-toggle", theme_label, 6, palette);
    theme_button = theme_button
        .child(icon(theme_icon, 14.).text_color(rgb(palette.muted_foreground.rgb_hex())));
    let theme_message = theme_label.to_owned();
    theme_button = theme_button.on_click(cx.listener(move |view, _, _, cx| {
        let next_mode = match view.state.settings.themeMode {
            ThemeMode::Light => ThemeMode::Dark,
            ThemeMode::Dark => ThemeMode::System,
            ThemeMode::System => ThemeMode::Light,
        };
        view.set_theme_mode(next_mode);
        view.show_toast(ToastKind::Info, theme_message.clone(), cx);
        cx.notify();
    }));

    let settings_active = active_page == Page::Settings;
    let mut settings_button = utility_button(
        "tab-settings",
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
    settings_button = settings_button.on_click(cx.listener(|view, _, _, cx| {
        view.select_page(Page::Settings, cx);
    }));

    div()
        .h(px(36.))
        .flex_none()
        .flex()
        .items_center()
        .justify_between()
        .px(px(10.))
        .bg(rgba((palette.card.rgb_hex() << 8) | 0x99))
        .child(pages)
        .child(
            div()
                .flex()
                .items_center()
                .gap_1()
                .child(theme_button)
                .child(settings_button),
        )
}

fn nav_item(
    page: Page,
    active: bool,
    tab_index: isize,
    language: Language,
    palette: Palette,
    cx: &mut Context<AhabApp>,
) -> impl IntoElement {
    let mut item = div()
        .id(format!("tab-{:?}", page))
        .tab_index(tab_index)
        .aria_label(page.label_for(language))
        .flex()
        .items_center()
        .gap(px(6.))
        .rounded_md()
        .px(px(10.))
        .py(px(4.))
        .text_size(px(12.))
        .cursor_pointer()
        .focus_visible(|style| style.border_1().border_color(rgb(palette.ring.rgb_hex())))
        .active(|style| style.bg(rgb(palette.secondary.rgb_hex())))
        .on_click(cx.listener(move |view, _, _, cx| {
            view.select_page(page, cx);
        }))
        .child(icon(page_icon(page), 14.).text_color(rgb(if active {
            palette.brand_foreground.rgb_hex()
        } else {
            palette.muted_foreground.rgb_hex()
        })))
        .child(page.label_for(language));

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

#[cfg(test)]
mod tests {
    use super::*;

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
}
