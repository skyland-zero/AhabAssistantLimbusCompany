use super::*;

pub(crate) fn starlight_editor(
    app: &mut AhabApp,
    cx: &mut Context<AhabApp>,
    config: &TeamMirrorConfig,
    language: Language,
) -> Div {
    let use_starlight = card(
        div()
            .flex()
            .flex_col()
            .gap_1()
            .child(control_row(
                text("使用开局星光", "Use Starting Starlight").get(language),
                mirror_switch(
                    app,
                    cx,
                    MirrorBool::UseStarlight,
                    config.use_starlight,
                    "starlight-enabled",
                ),
            ))
            .child(
                div().text_size(px(10.)).text_color(rgb(TEXT_MUTED)).child(
                    text(
                        "开局时按下方等级消耗星光点数。",
                        "Spend starlight at the start according to the levels below.",
                    )
                    .get(language),
                ),
            ),
    )
    .p_3()
    .flex_none();

    let mut quick = div().flex().items_center().gap_1().flex_wrap();
    for level in 0..=3_u8 {
        let mut control = button(
            starlight_level_label(level, language),
            ButtonVariant::Outline,
        )
        .id(format!("starlight-all-{level}"))
        .h(px(26.))
        .px_2()
        .py_0();
        control = control
            .on_click(cx.listener(move |view, _, _, cx| {
                view.teams.set_all_starlight(level);
                cx.notify();
            }))
            .on_key_down(cx.listener(move |view, event: &KeyDownEvent, window, cx| {
                if team_activation_key(event) {
                    window.prevent_default();
                    view.teams.set_all_starlight(level);
                    cx.notify();
                }
            }));
        quick = quick.child(control);
    }

    let cost_badge = div()
        .flex()
        .items_center()
        .gap_1()
        .px_2()
        .py_1()
        .rounded_md()
        .bg(palette_rgb(current_render_palette().brand_light))
        .text_size(px(11.))
        .text_color(palette_rgb(current_render_palette().brand))
        .child(icon(ICON_SPARKLES, 13., current_render_palette().brand))
        .child(starlight_cost_label(app.teams.starlight_cost(), language));
    let quick_card = card(
        div()
            .flex()
            .items_center()
            .justify_between()
            .gap_3()
            .flex_wrap()
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .child(
                        div()
                            .text_size(px(11.))
                            .text_color(rgb(TEXT_MUTED))
                            .child(text("一键设置等级", "Set All Levels").get(language)),
                    )
                    .child(quick),
            )
            .child(cost_badge),
    )
    .p_3()
    .flex_none();

    let mut items = div().flex().flex_wrap().gap_2();
    for (index, cost) in STARLIGHT_COSTS.iter().copied().enumerate() {
        let level = config.opening_bonus.get(index).copied().unwrap_or(0).min(3);
        let mut levels = div().flex().gap_1();
        for candidate in 0..=3_u8 {
            let mut control = button(
                starlight_short_level(candidate),
                if candidate == level {
                    ButtonVariant::Secondary
                } else {
                    ButtonVariant::Ghost
                },
            )
            .id(format!("starlight-{index}-{candidate}"))
            .h(px(24.))
            .px_2()
            .py_0();
            control = control
                .on_click(cx.listener(move |view, _, _, cx| {
                    view.teams.set_starlight_level(index, candidate);
                    cx.notify();
                }))
                .on_key_down(cx.listener(move |view, event: &KeyDownEvent, window, cx| {
                    if team_activation_key(event) {
                        window.prevent_default();
                        view.teams.set_starlight_level(index, candidate);
                        cx.notify();
                    }
                }));
            levels = levels.child(control);
        }
        let cost = div()
            .flex()
            .items_center()
            .gap_1()
            .text_size(px(10.))
            .text_color(rgb(TEXT_MUTED))
            .child(icon(ICON_SPARKLES, 11., current_render_palette().brand))
            .child(starlight_points_label(cost, language));
        items = items.child(
            card(
                div()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .gap_2()
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap_1()
                                    .min_w_0()
                                    .text_size(px(12.))
                                    .text_color(rgb(TEXT))
                                    .child(starlight_name(index, language)),
                            )
                            .child(cost),
                    )
                    .child(levels)
                    .child(
                        div()
                            .text_size(px(10.))
                            .text_color(rgb(TEXT_MUTED))
                            .child(starlight_description(index, language)),
                    ),
            )
            .p_3()
            .flex_basis(px(320.))
            .flex_grow(1.),
        );
    }

    div()
        .flex()
        .flex_col()
        .gap_3()
        .child(use_starlight)
        .child(quick_card)
        .child(items)
}
