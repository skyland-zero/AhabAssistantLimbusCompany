use super::*;

use gpui::{AnyView, App, Render, Window};

use crate::theme::Palette;
use crate::theme::SkinId;

struct AccentTooltip {
    label: String,
    palette: Palette,
}

impl Render for AccentTooltip {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .px_2()
            .py_1()
            .rounded_sm()
            .border_1()
            .border_color(palette_rgb(self.palette.input))
            .bg(palette_rgb(self.palette.popover))
            .text_color(palette_rgb(self.palette.popover_foreground))
            .text_size(px(11.))
            .child(self.label.clone())
    }
}

fn accent_tooltip(label: String, palette: Palette) -> impl Fn(&mut Window, &mut App) -> AnyView {
    move |_window, cx| {
        cx.new(|_| AccentTooltip {
            label: label.clone(),
            palette,
        })
        .into()
    }
}

pub fn appearance_card(
    _app: &mut AhabApp,
    cx: &mut Context<AhabApp>,
    theme: ThemeMode,
    language: Language,
    accent: &str,
    skin: &str,
) -> Div {
    let mut modes = segmented_group();
    for (candidate, label) in [
        (ThemeMode::Light, text("浅色", "Light")),
        (ThemeMode::Dark, text("深色", "Dark")),
        (ThemeMode::System, text("跟随系统", "System")),
    ] {
        let mut control = button(
            label.get(language),
            if theme == candidate {
                ButtonVariant::Secondary
            } else {
                ButtonVariant::Ghost
            },
        )
        .id(format!("settings-theme-{candidate:?}"))
        .px_3()
        .py_1()
        .text_size(px(12.));
        let message = label.get(language).to_owned();
        let key_message = message.clone();
        control = control
            .on_click(cx.listener(move |view, _, _, cx| {
                view.set_theme_mode(candidate);
                view.show_toast(crate::shell::ToastKind::Info, message.clone(), cx);
                cx.notify();
            }))
            .on_key_down(cx.listener(move |view, event: &KeyDownEvent, window, cx| {
                if is_activation_key(event) {
                    window.prevent_default();
                    view.set_theme_mode(candidate);
                    view.show_toast(crate::shell::ToastKind::Info, key_message.clone(), cx);
                    cx.notify();
                }
            }));
        modes = modes.child(control);
    }

    let palette = current_render_palette();
    let dark_accent = matches!(palette.scheme, ColorScheme::Dark);
    let mut accents = div().flex().items_center().gap_2();
    for preset in ACCENT_PRESETS {
        let selected = accent == preset.id;
        let color = if dark_accent {
            preset.dark.brand.rgb_hex()
        } else {
            preset.light.brand.rgb_hex()
        };
        let tooltip_label = format!("{} / {}", preset.name, preset.name_en);
        let toast_label = match language {
            Language::ZhCn => preset.name,
            Language::EnUs => preset.name_en,
        };
        let message = format!("{}: {toast_label}", text("强调色", "Accent").get(language));
        let mut control = div()
            .id(format!("settings-accent-{}", preset.id))
            .w(px(24.))
            .h(px(24.))
            .rounded_full()
            .tab_index(0)
            .cursor_pointer()
            .aria_label(tooltip_label.clone())
            .tooltip(accent_tooltip(tooltip_label, palette))
            .focus_visible(|style| style.border_color(palette_rgb(palette.ring)))
            .bg(gpui_rgb(color));
        if selected {
            control = control.border_2().border_color(rgb(TEXT)).opacity(1.);
        } else {
            control = control.opacity(0.7);
        }
        let id = preset.id;
        let key_message = message.clone();
        control = control
            .on_click(cx.listener(move |view, _, _, cx| {
                view.set_accent(id);
                view.show_toast(crate::shell::ToastKind::Info, message.clone(), cx);
                cx.notify();
            }))
            .on_key_down(cx.listener(move |view, event: &KeyDownEvent, window, cx| {
                if is_activation_key(event) {
                    window.prevent_default();
                    view.set_accent(id);
                    view.show_toast(crate::shell::ToastKind::Info, key_message.clone(), cx);
                    cx.notify();
                }
            }));
        accents = accents.child(control);
    }

    let mut skins = segmented_group();
    for preset in SkinId::ALL {
        let id = preset.as_str();
        let selected = skin == id;
        let label = match language {
            Language::ZhCn => preset.name_zh(),
            Language::EnUs => preset.name_en(),
        };
        let mut control = button(
            label,
            if selected {
                ButtonVariant::Secondary
            } else {
                ButtonVariant::Ghost
            },
        )
        .id(format!("settings-skin-{id}"))
        .px_3()
        .py_1()
        .text_size(px(12.));
        let message = format!("{}: {label}", text("风格", "Skin").get(language));
        let key_message = message.clone();
        control = control
            .on_click(cx.listener(move |view, _, _, cx| {
                view.set_skin(id);
                view.show_toast(crate::shell::ToastKind::Info, message.clone(), cx);
                cx.notify();
            }))
            .on_key_down(cx.listener(move |view, event: &KeyDownEvent, window, cx| {
                if is_activation_key(event) {
                    window.prevent_default();
                    view.set_skin(id);
                    view.show_toast(crate::shell::ToastKind::Info, key_message.clone(), cx);
                    cx.notify();
                }
            }));
        skins = skins.child(control);
    }

    let mut languages = segmented_group();    for (candidate, label) in [(Language::ZhCn, "简体中文"), (Language::EnUs, "English")] {
        let mut control = button(
            label,
            if language == candidate {
                ButtonVariant::Secondary
            } else {
                ButtonVariant::Ghost
            },
        )
        .id(format!("settings-language-{candidate:?}"))
        .px_3()
        .py_1()
        .text_size(px(12.));
        control = control
            .on_click(cx.listener(move |view, _, _, cx| {
                view.set_language(candidate);
                view.show_toast(crate::shell::ToastKind::Info, label, cx);
                cx.notify();
            }))
            .on_key_down(cx.listener(move |view, event: &KeyDownEvent, window, cx| {
                if is_activation_key(event) {
                    window.prevent_default();
                    view.set_language(candidate);
                    view.show_toast(crate::shell::ToastKind::Info, label, cx);
                    cx.notify();
                }
            }));
        languages = languages.child(control);
    }

    let body = div()
        .flex()
        .flex_col()
        .gap(px(12.))
        .px_4()
        .pb_4()
        .child(settings_list(vec![
            setting_line(text("主题模式", "Theme Mode").get(language), modes),
            setting_line(text("强调色", "Accent Color").get(language), accents),
            setting_line(text("风格", "Skin").get(language), skins),
            setting_line(
                text("语言 / Language", "Language / 语言").get(language),
                languages,
            ),
        ]));
    settings_card(text("外观", "Appearance").get(language), body)
}
