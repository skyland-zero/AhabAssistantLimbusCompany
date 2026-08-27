//! Theme-pack management page backed by the shared Mock IPC state.
//!
//! The layout mirrors `ui/src/pages/ThemePacksPage.tsx`: a compact action bar,
//! an optional hard-mirror warning, and a single independently scrolling list.
//! The page deliberately keeps all mutations in `ThemePacksState`.

use gpui::{Context, Div, Svg, div, prelude::*, px, rgb, svg};

use crate::{
    app::{ACCENT, AhabApp, BACKGROUND, BORDER, SURFACE, TEXT, TEXT_MUTED},
    components::{
        BadgeTone, ButtonVariant, badge, button, card, empty_state, scroll_area_with_id, slider,
        switch,
    },
    model::{Language, ThemePack},
};

const ICON_SORT: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="2"><path d="m3 6 3-3 3 3"/><path d="M6 3v18"/><path d="m21 18-3 3-3-3"/><path d="M18 21V3"/></svg>"#;
const ICON_RESET: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="2"><path d="M3 12a9 9 0 1 0 3-6.7"/><path d="M3 4v6h6"/></svg>"#;
const ICON_ALERT: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="2"><path d="m10.3 2.9-8.6 15a2 2 0 0 0 1.7 3h17.2a2 2 0 0 0 1.7-3l-8.6-15a2 2 0 0 0-3.4 0Z"/><path d="M12 9v4"/><path d="M12 17h.01"/></svg>"#;

#[derive(Clone, Copy)]
struct Localized {
    zh: &'static str,
    en: &'static str,
}

impl Localized {
    fn get(self, language: Language) -> &'static str {
        match language {
            Language::ZhCn => self.zh,
            Language::EnUs => self.en,
        }
    }
}

const fn text(zh: &'static str, en: &'static str) -> Localized {
    Localized { zh, en }
}

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
        Some(icon(ICON_SORT, 14., ACCENT)),
    )
    .id("theme-sort");
    sort = sort.on_click(cx.listener(move |view, _, _, cx| {
        view.theme_packs.set_sort_by_weight(!sort_by_weight);
        cx.notify();
    }));

    let mut enable_all = action_button(
        text("全部启用", "Enable All").get(language),
        ButtonVariant::Outline,
        None,
    )
    .id("theme-enable-all");
    enable_all = enable_all.on_click(cx.listener(|view, _, _, cx| {
        view.theme_packs.set_all_enabled(true);
        cx.notify();
    }));

    let mut disable_all = action_button(
        text("全部停用", "Disable All").get(language),
        ButtonVariant::Outline,
        None,
    )
    .id("theme-disable-all");
    disable_all = disable_all.on_click(cx.listener(|view, _, _, cx| {
        view.theme_packs.set_all_enabled(false);
        cx.notify();
    }));

    let mut reset = action_button(
        text("恢复默认权重", "Reset Weights").get(language),
        ButtonVariant::Ghost,
        Some(icon(ICON_RESET, 14., TEXT_MUTED)),
    )
    .id("theme-reset");
    reset = reset.on_click(cx.listener(|view, _, _, cx| {
        view.theme_packs.reset_weights();
        cx.notify();
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
                .bg(rgb(0x3d301b))
                .px_6()
                .py_2()
                .text_size(px(12.))
                .text_color(rgb(ACCENT))
                .child(icon(ICON_ALERT, 14., ACCENT))
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
                .text_color(rgb(0x4dcc89))
                .child(feedback),
        );
    }
    root.child(
        scroll_area_with_id("theme-packs-scroll", div().w_full().p_6().child(list))
            .flex_1()
            .min_h_0(),
    )
}

fn pack_row(
    _app: &mut AhabApp,
    cx: &mut Context<AhabApp>,
    pack: ThemePack,
    language: Language,
) -> Div {
    let enabled = pack.enabled;
    let id = pack.id.clone();
    let mut toggle = switch(enabled).id(format!("theme-enabled-{id}"));
    toggle = toggle.on_click(cx.listener(move |view, _, _, cx| {
        view.theme_packs.toggle_enabled(&id);
        cx.notify();
    }));

    let id_for_slider = pack.id.clone();
    let mut weight_slider = slider(pack.weight as f32, 0., 10.)
        .id(format!("theme-weight-slider-{}", pack.id))
        .flex_1()
        .min_w(px(80.))
        .min_h(px(16.));
    if enabled {
        weight_slider = weight_slider.on_click(cx.listener(move |view, _, _, cx| {
            view.theme_packs.cycle_weight(&id_for_slider);
            cx.notify();
        }));
    } else {
        weight_slider = weight_slider.opacity(0.5).cursor_not_allowed();
    }

    let card_content = div()
        .flex()
        .items_center()
        .gap_4()
        .px_5()
        .py_3()
        .child(toggle)
        .child(
            div().w(px(176.)).min_w_0().flex_none().child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .child(
                        div()
                            .min_w_0()
                            .text_size(px(14.))
                            .text_color(rgb(TEXT))
                            .child(pack.name),
                    )
                    .child(badge(pack.tier, BadgeTone::Neutral)),
            ),
        )
        .child(
            div()
                .flex()
                .items_center()
                .gap_3()
                .min_w_0()
                .flex_1()
                .child(
                    div()
                        .flex_none()
                        .text_size(px(12.))
                        .text_color(rgb(TEXT_MUTED))
                        .child(text("权重", "Weight").get(language)),
                )
                .child(weight_slider)
                .child(
                    div()
                        .w(px(24.))
                        .flex_none()
                        .font_family("Consolas")
                        .text_size(px(12.))
                        .text_color(rgb(TEXT))
                        .child(pack.weight.to_string()),
                ),
        );

    let mut row = card(card_content).p_0();
    if !enabled {
        row = row.opacity(0.6);
    }
    row
}

fn action_button(label: &'static str, variant: ButtonVariant, icon: Option<Svg>) -> Div {
    let mut control = button("", variant)
        .h(px(28.))
        .px_3()
        .py_0()
        .text_size(px(12.));
    if let Some(icon) = icon {
        control = control.child(icon);
    }
    control.child(label)
}

fn icon(data: &'static str, size: f32, color: u32) -> Svg {
    svg()
        .data(data.as_bytes())
        .size(px(size))
        .text_color(rgb(color))
}

#[cfg(test)]
mod tests {
    use crate::state::ThemePacksState;

    #[test]
    fn mock_theme_pack_total_matches_page_baseline() {
        assert_eq!(ThemePacksState::default().total_weight(), 28);
    }
}
