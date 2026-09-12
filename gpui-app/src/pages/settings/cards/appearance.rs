use super::skin_preview::{accent_tooltip, skin_picker};
use super::*;

use crate::components::style::{AccentId, AccentTokens, FONT_MD, FONT_SM, ShapeExt, SkinId};

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
        .text_size(px(FONT_MD));
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
    let scheme = palette.scheme;
    let dark_accent = palette.is_dark();
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
            control = control
                .border_2()
                .border_color(palette_rgb(palette.foreground));
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

    // The frame skins were designed around a brass rule. Say so instead of
    // silently letting a violet accent fight a gold frame.
    let signature_hint =
        SkinId::parse(skin).is_limbus() && !AccentTokens::is_signature(AccentId::parse(accent));
    let mut accent_row = div()
        .flex()
        .items_center()
        .justify_end()
        .gap_2()
        .child(accents);
    if signature_hint {
        accent_row = accent_row.child(
            div()
                .px_2()
                .py_1()
                .text_size(px(FONT_SM))
                .skin_radius_sm(&palette)
                .bg(palette_rgb(palette.warning_light))
                .text_color(palette_rgb(palette.warning))
                .child(text("边狱签名配色为黄铜", "Limbus is designed around brass").get(language)),
        );
    }

    let mut languages = segmented_group();
    for (candidate, label) in [(Language::ZhCn, "简体中文"), (Language::EnUs, "English")] {
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
        .text_size(px(FONT_MD));
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
        .gap_3()
        .px_4()
        .pb_4()
        .child(settings_list(vec![
            setting_line(text("主题模式", "Theme Mode").get(language), modes),
            setting_line(
                text("强调色", "Accent Color").get(language),
                accent_row,
            ),
            setting_line(
                text("语言 / Language", "Language / 语言").get(language),
                languages,
            ),
        ]))
        // The skin picker needs the full card width, so it sits outside the
        // label/control rows.
        .child(
            div()
                .flex()
                .flex_col()
                .gap_2()
                .child(
                    div()
                        .text_size(px(FONT_MD))
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(rgb(TEXT))
                        .child(text("界面皮肤", "Interface Skin").get(language)),
                )
                .child(skin_picker(language, accent, scheme, skin, cx))
                .child(
                    div()
                        .text_size(px(FONT_SM))
                        .text_color(rgb(TEXT_MUTED))
                        .child(
                            text(
                                "「镜牢主题包」在导航栏单独管理，与界面皮肤无关。",
                                "Mirror Packs are managed from the navigation bar and are unrelated to the interface skin.",
                            )
                            .get(language),
                        ),
                ),
        );
    settings_card(text("外观", "Appearance").get(language), body)
}
