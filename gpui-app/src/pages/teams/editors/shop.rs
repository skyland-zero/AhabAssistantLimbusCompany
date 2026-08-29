use super::*;

pub(crate) fn shop_editor(
    app: &mut AhabApp,
    cx: &mut Context<AhabApp>,
    config: &TeamMirrorConfig,
    language: Language,
) -> Div {
    let mut discard = div().flex().flex_wrap().gap_2();
    for (index, name) in SYSTEM_NAMES.iter().enumerate() {
        let selected = discard_value(&config.discard_systems, index);
        let mut control =
            system_choice(index, selected, true, language).id(format!("discard-system-{name}"));
        control = control.on_click(cx.listener(move |view, _, _, cx| {
            view.teams.toggle_discard_system(index);
            cx.notify();
        }));
        discard = discard.child(control);
    }

    let restrictions = [
        (
            text("不治疗", "Do Not Heal"),
            MirrorBool::DoNotHeal,
            config.do_not_heal,
        ),
        (
            text("不购买", "Do Not Buy"),
            MirrorBool::DoNotBuy,
            config.do_not_buy,
        ),
        (
            text("不合成", "Do Not Fuse"),
            MirrorBool::DoNotFuse,
            config.do_not_fuse,
        ),
        (
            text("不出售", "Do Not Sell"),
            MirrorBool::DoNotSell,
            config.do_not_sell,
        ),
        (
            text("不升级", "Do Not Enhance"),
            MirrorBool::DoNotEnhance,
            config.do_not_enhance,
        ),
    ];
    let mut restriction_items = Vec::new();
    for (index, (label, field, value)) in restrictions.into_iter().enumerate() {
        restriction_items.push(control_row(
            label.get(language),
            mirror_switch(app, cx, field, value, format!("shop-restriction-{index}")),
        ));
    }
    let restriction_rows = settings_grid(restriction_items, 180.);

    let fusions = [
        (
            text("仅激进合成", "Aggressive Fusion Only"),
            MirrorBool::OnlyAggressiveFuse,
            config.only_aggressive_fuse,
        ),
        (
            text("不合成体系饰品", "Do Not Fuse System Gifts"),
            MirrorBool::DoNotSystemFuse,
            config.do_not_system_fuse,
        ),
        (
            text("仅合成体系饰品", "System Gifts Only"),
            MirrorBool::OnlySystemFuse,
            config.only_system_fuse,
        ),
        (
            text("激进模式也升级", "Enhance in Aggressive Mode"),
            MirrorBool::AggressiveAlsoEnhance,
            config.aggressive_also_enhance,
        ),
        (
            text("激进模式保留体系", "Keep System Gifts in Aggressive Mode"),
            MirrorBool::AggressiveSaveSystems,
            config.aggressive_save_systems,
        ),
    ];
    let mut fusion_items = Vec::new();
    for (index, (label, field, value)) in fusions.into_iter().enumerate() {
        fusion_items.push(control_row(
            label.get(language),
            mirror_switch(app, cx, field, value, format!("fusion-{index}")),
        ));
    }
    let fusion_rows = settings_grid(fusion_items, 180.);

    let after_level = card(
        div()
            .flex()
            .flex_col()
            .gap_2()
            .child(control_row(
                text("四级饰品后执行策略", "After Tier 4 Fusion").get(language),
                mirror_switch(
                    app,
                    cx,
                    MirrorBool::AfterLevelIv,
                    config.after_level_IV,
                    "shop-after-level-iv",
                ),
            ))
            .child(if config.after_level_IV {
                team_select(
                    app,
                    cx,
                    TeamSelectConfig {
                        select: TeamSelect::AfterLevelIv,
                        current: config.after_level_IV_select.to_string(),
                        options: vec![
                            ("0".to_owned(), after_level_label(0, language).to_owned()),
                            ("1".to_owned(), after_level_label(1, language).to_owned()),
                            ("2".to_owned(), after_level_label(2, language).to_owned()),
                            ("3".to_owned(), after_level_label(3, language).to_owned()),
                        ],
                        id: "shop-after-level-iv-action".to_owned(),
                        width: 180.,
                        on_change: Rc::new(|teams, value| {
                            if let Ok(value) = value.parse::<u8>() {
                                teams.set_mirror_u8(MirrorU8::AfterLevelIvSelect, value);
                            }
                        }),
                    },
                )
                .into_any_element()
            } else {
                div().into_any_element()
            }),
    );

    let keyword_refresh = app
        .team_inputs
        .keyword_refresh
        .clone()
        .map(|input| div().w(px(80.)).child(input))
        .unwrap_or_else(|| div().child(text("初始化中", "Initializing").get(language)));
    let normal_refresh = app
        .team_inputs
        .normal_refresh
        .clone()
        .map(|input| div().w(px(80.)).child(input))
        .unwrap_or_else(|| div().child(text("初始化中", "Initializing").get(language)));
    let refresh = card(
        div()
            .flex()
            .flex_col()
            .gap_2()
            .child(
                div()
                    .text_size(px(13.))
                    .text_color(rgb(TEXT))
                    .child(text("商店刷新上限", "Shop Refresh Limits").get(language)),
            )
            .child(control_row(
                text("关键词刷新", "Keyword Refreshes").get(language),
                keyword_refresh,
            ))
            .child(control_row(
                text("普通刷新", "Normal Refreshes").get(language),
                normal_refresh,
            )),
    );

    let mut floors = div().flex().flex_wrap().gap_2();
    for floor in 0..5 {
        let ignored = config.ignore_shop.get(floor).copied().unwrap_or(false);
        let mut control = button(
            if ignored {
                format!("{} {}F", text("已忽略", "Ignored").get(language), floor + 1)
            } else {
                format!("{}F", floor + 1)
            },
            if ignored {
                ButtonVariant::Destructive
            } else {
                ButtonVariant::Outline
            },
        )
        .id(format!("ignore-shop-floor-{}", floor + 1))
        .h(px(32.))
        .px_3()
        .py_0();
        control = control
            .on_click(cx.listener(move |view, _, _, cx| {
                view.teams.toggle_ignore_shop(floor);
                cx.notify();
            }))
            .on_key_down(cx.listener(move |view, event: &KeyDownEvent, window, cx| {
                if team_activation_key(event) {
                    window.prevent_default();
                    view.teams.toggle_ignore_shop(floor);
                    cx.notify();
                }
            }));
        floors = floors.child(control);
    }
    let ignore = card(
        div()
            .flex()
            .flex_col()
            .gap_2()
            .child(
                div()
                    .text_size(px(13.))
                    .text_color(rgb(TEXT))
                    .child(text("忽略商店楼层", "Ignore Shop Floors").get(language)),
            )
            .child(floors),
    );

    div()
        .flex()
        .flex_col()
        .gap_4()
        .child(field_block(
            text("商店策略", "Shop Strategy").get(language),
            team_select(
                app,
                cx,
                TeamSelectConfig {
                    select: TeamSelect::ShopStrategy,
                    current: config.shop_strategy.to_string(),
                    options: vec![
                        ("0".to_owned(), shop_strategy_label(0, language).to_owned()),
                        ("1".to_owned(), shop_strategy_label(1, language).to_owned()),
                        ("2".to_owned(), shop_strategy_label(2, language).to_owned()),
                    ],
                    id: "shop-strategy".to_owned(),
                    width: 180.,
                    on_change: Rc::new(|teams, value| {
                        if let Ok(value) = value.parse::<u8>() {
                            teams.set_mirror_u8(MirrorU8::ShopStrategy, value);
                        }
                    }),
                },
            ),
        ))
        .child(field_block(
            text("舍弃饰品体系", "Discard Gift Systems").get(language),
            div()
                .flex()
                .flex_col()
                .gap_2()
                .child(
                    div().text_size(px(10.)).text_color(rgb(TEXT_MUTED)).child(
                        text(
                            "选择不参与商店购买、合成与保留的体系。",
                            "Choose systems to avoid in shop purchases, fusion and retention.",
                        )
                        .get(language),
                    ),
                )
                .child(discard),
        ))
        .child(field_block(
            text("基础操作限制", "Shop Action Restrictions").get(language),
            div()
                .flex()
                .flex_col()
                .gap_2()
                .child(
                    div().text_size(px(10.)).text_color(rgb(TEXT_MUTED)).child(
                        text(
                            "限制会在镜牢商店阶段按队伍配置执行。",
                            "Restrictions are applied during Mirror Dungeon shop phases.",
                        )
                        .get(language),
                    ),
                )
                .child(restriction_rows),
        ))
        .child(field_block(
            text("合成策略", "Fusion Strategy").get(language),
            fusion_rows,
        ))
        .child(settings_grid(vec![after_level, refresh, ignore], 220.))
}
