//! Theme-pack management page backed by the shared Mock IPC state.
//!
//! The layout provides a compact action bar, an optional hard-mirror warning,
//! and a single independently scrolling list.
//! The page deliberately keeps all mutations in `ThemePacksState`.

use gpui::{Context, Div, KeyDownEvent, div, prelude::*, px};

use crate::{
    app::{ACCENT, AhabApp, BACKGROUND, BORDER, SURFACE, TEXT, TEXT_MUTED},
    components::style::current_render_palette,
    components::{
        ButtonVariant, action_button, empty_state, is_activation_key, palette_rgb,
        render_rgb as rgb, scroll_area_with_id, svg_icon, switch,
    },
    i18n::paired as text,
    model::{Language, ThemePack},
};

mod icons;
mod rows;

use icons::{ICON_ALERT, ICON_RESET, ICON_SORT};
use rows::pack_row;

pub fn render(app: &mut AhabApp, cx: &mut Context<AhabApp>) -> Div {
    let language = app.state.settings.language;
    let sort_by_weight = app.theme_packs.sort_by_weight;
    let total_weight = app.theme_packs.total_weight();
    let packs = app.theme_packs.sorted_packs();
    let feedback = app.theme_packs.feedback.clone();

    let mut sort = action_button(
        text("按权重排序", "Sort by Weight").get(language),
        if sort_by_weight {
            ButtonVariant::Secondary
        } else {
            ButtonVariant::Outline
        },
        Some(svg_icon(ICON_SORT, 14., ACCENT)),
        28.,
    )
    .id("theme-sort");
    sort = sort
        .on_click(cx.listener(move |view, _, _, cx| {
            view.theme_packs.set_sort_by_weight(!sort_by_weight);
            view.show_toast(
                crate::shell::ToastKind::Info,
                text("主题包排序已更新", "Theme-pack sorting updated").get(language),
                cx,
            );
            cx.notify();
        }))
        .on_key_down(cx.listener(move |view, event: &KeyDownEvent, window, cx| {
            if is_activation_key(event) {
                window.prevent_default();
                view.theme_packs.set_sort_by_weight(!sort_by_weight);
                view.show_toast(
                    crate::shell::ToastKind::Info,
                    text("主题包排序已更新", "Theme-pack sorting updated").get(language),
                    cx,
                );
                cx.notify();
            }
        }));

    let mut enable_all = action_button(
        text("全部启用", "Enable All").get(language),
        ButtonVariant::Outline,
        None,
        28.,
    )
    .id("theme-enable-all");
    enable_all = enable_all
        .on_click(cx.listener(move |view, _, _, cx| {
            view.theme_packs.set_all_enabled(true);
            view.show_toast(
                crate::shell::ToastKind::Success,
                text("主题包已全部启用", "All theme packs enabled").get(language),
                cx,
            );
            cx.notify();
        }))
        .on_key_down(cx.listener(move |view, event: &KeyDownEvent, window, cx| {
            if is_activation_key(event) {
                window.prevent_default();
                view.theme_packs.set_all_enabled(true);
                view.show_toast(
                    crate::shell::ToastKind::Success,
                    text("主题包已全部启用", "All theme packs enabled").get(language),
                    cx,
                );
                cx.notify();
            }
        }));

    let mut disable_all = action_button(
        text("全部停用", "Disable All").get(language),
        ButtonVariant::Outline,
        None,
        28.,
    )
    .id("theme-disable-all");
    disable_all = disable_all
        .on_click(cx.listener(move |view, _, _, cx| {
            view.theme_packs.set_all_enabled(false);
            view.show_toast(
                crate::shell::ToastKind::Info,
                text("主题包已全部停用", "All theme packs disabled").get(language),
                cx,
            );
            cx.notify();
        }))
        .on_key_down(cx.listener(move |view, event: &KeyDownEvent, window, cx| {
            if is_activation_key(event) {
                window.prevent_default();
                view.theme_packs.set_all_enabled(false);
                view.show_toast(
                    crate::shell::ToastKind::Info,
                    text("主题包已全部停用", "All theme packs disabled").get(language),
                    cx,
                );
                cx.notify();
            }
        }));

    let mut reset = action_button(
        text("恢复默认权重", "Reset Weights").get(language),
        ButtonVariant::Ghost,
        Some(svg_icon(ICON_RESET, 14., TEXT_MUTED)),
        28.,
    )
    .id("theme-reset");
    reset = reset
        .on_click(cx.listener(move |view, _, _, cx| {
            view.theme_packs.reset_weights();
            view.show_toast(
                crate::shell::ToastKind::Success,
                text("主题包权重已恢复", "Theme-pack weights reset").get(language),
                cx,
            );
            cx.notify();
        }))
        .on_key_down(cx.listener(move |view, event: &KeyDownEvent, window, cx| {
            if is_activation_key(event) {
                window.prevent_default();
                view.theme_packs.reset_weights();
                view.show_toast(
                    crate::shell::ToastKind::Success,
                    text("主题包权重已恢复", "Theme-pack weights reset").get(language),
                    cx,
                );
                cx.notify();
            }
        }));

    let mut actions = div()
        .flex()
        .items_center()
        .justify_end()
        .gap_1p5()
        .flex_wrap();
    actions = actions
        .child(sort)
        .child(enable_all)
        .child(disable_all)
        .child(reset);

    let toolbar = div()
        .flex()
        .items_center()
        .justify_between()
        .gap_3()
        .border_b_1()
        .border_color(rgb(BORDER))
        .bg(rgb(SURFACE))
        .px_5()
        .py(px(10.))
        .child(
            div()
                .flex()
                .items_center()
                .gap_2()
                .text_size(px(12.))
                .text_color(rgb(TEXT_MUTED))
                .child(text("总权重：", "Total Weight:").get(language))
                .child(
                    div()
                        .font_family("Consolas")
                        .text_size(px(13.))
                        .text_color(rgb(TEXT))
                        .child(total_weight.to_string()),
                ),
        )
        .child(actions);

    let warning = if app.theme_packs.data.hardMirrorActive {
        Some(
            div()
                .flex()
                .items_center()
                .gap_2()
                .border_b_1()
                .border_color(rgb(BORDER))
                .bg(palette_rgb(current_render_palette().warning_light))
                .px_6()
                .py_2()
                .text_size(px(12.))
                .text_color(palette_rgb(current_render_palette().warning))
                .child(svg_icon(
                    ICON_ALERT,
                    14.,
                    current_render_palette().warning.rgb_hex(),
                ))
                .child(text(
                    "困难镜牢周期进行中：建议优先筛选高星主题包并提高其权重",
                    "Hard Mirror Dungeon cycle active: recommend raising high-tier pack weights",
                )
                .get(language)),
        )
    } else {
        None
    };

    let list = if packs.is_empty() {
        div().w_full().child(empty_state(
            text("暂无主题包", "No theme packs").get(language),
            text(
                "主题包资源尚未加载，完成资源同步后再试。",
                "Theme-pack resources are not loaded yet. Try again after syncing resources.",
            )
            .get(language),
        ))
    } else {
        let mut rows = div().flex().flex_col().gap_2();
        for pack in packs {
            rows = rows.child(pack_row(app, cx, pack, language));
        }
        rows
    };

    let mut root = div()
        .size_full()
        .flex()
        .flex_col()
        .bg(rgb(BACKGROUND))
        .child(toolbar);
    if let Some(warning) = warning {
        root = root.child(warning);
    }
    if let Some(feedback) = feedback {
        root = root.child(
            div()
                .px_6()
                .py_2()
                .text_size(px(12.))
                .text_color(palette_rgb(current_render_palette().success))
                .child(localized_feedback(&feedback, language)),
        );
    }
    root.child(
        scroll_area_with_id("theme-packs-scroll", div().w_full().p_6().child(list))
            .flex_1()
            .min_h_0(),
    )
}

fn localized_feedback(feedback: &str, language: Language) -> String {
    if matches!(language, Language::ZhCn) {
        return feedback.to_owned();
    }
    match feedback {
        "已恢复默认权重" => "Default weights restored".to_owned(),
        "主题包设置已保存" => "Theme-pack settings saved".to_owned(),
        _ => feedback.to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::rows::slider_weight_from_position;
    use crate::state::ThemePacksState;

    #[test]
    fn mock_theme_pack_total_matches_page_baseline() {
        assert_eq!(ThemePacksState::default().total_weight(), 28);
    }

    #[test]
    fn slider_mouse_position_is_bounded_to_weight_range() {
        assert_eq!(slider_weight_from_position(-4., 180.), 0);
        assert_eq!(slider_weight_from_position(90., 180.), 5);
        assert_eq!(slider_weight_from_position(220., 180.), 10);
        assert_eq!(slider_weight_from_position(20., 0.), 0);
    }
}
