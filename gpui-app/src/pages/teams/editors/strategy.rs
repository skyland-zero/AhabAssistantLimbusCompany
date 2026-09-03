use super::*;

fn strategy_select(
    app: &mut AhabApp,
    cx: &mut Context<AhabApp>,
    select: TeamSelect,
    current: u8,
    options: Vec<(String, String)>,
    id: &'static str,
    field: MirrorU8,
) -> Div {
    team_select(
        app,
        cx,
        TeamSelectConfig {
            select,
            current: current.to_string(),
            options,
            id: id.to_owned(),
            width: 250.,
            on_change: Rc::new(move |teams, value| {
                if let Ok(value) = value.parse::<u8>() {
                    teams.set_mirror_u8(field, value);
                }
            }),
        },
    )
}

pub(crate) fn strategy_editor(
    app: &mut AhabApp,
    cx: &mut Context<AhabApp>,
    config: &TeamMirrorConfig,
    language: Language,
) -> Div {
    let reward_options = (0..=3)
        .map(|value| {
            (
                value.to_string(),
                reward_cards_label(value, language).to_owned(),
            )
        })
        .collect();
    let shopping_options = (0..=5)
        .map(|value| {
            (
                value.to_string(),
                shopping_strategy_label_custom(value, language).to_owned(),
            )
        })
        .collect();
    let opening_order_options = (0..=5)
        .map(|value| {
            (
                value.to_string(),
                opening_items_order_label(value, language).to_owned(),
            )
        })
        .collect();
    let opening_system_options = SYSTEM_NAMES
        .iter()
        .enumerate()
        .map(|(value, _)| (value.to_string(), system_label(value, language).to_owned()))
        .collect();

    let reward = editor_card(
        div()
            .flex()
            .flex_col()
            .gap_2()
            .child(control_row(
                text("奖励卡优先度", "Reward Card Priority").get(language),
                mirror_switch(
                    app,
                    cx,
                    MirrorBool::RewardCards,
                    config.reward_cards,
                    "shop-reward-cards",
                ),
            ))
            .child(if config.reward_cards {
                strategy_select(
                    app,
                    cx,
                    TeamSelect::RewardCards,
                    config.reward_cards_select,
                    reward_options,
                    "shop-reward-cards-select",
                    MirrorU8::RewardCardsSelect,
                )
                .into_any_element()
            } else {
                div().into_any_element()
            }),
    );

    let shopping = editor_card(
        div()
            .flex()
            .flex_col()
            .gap_2()
            .child(control_row(
                text("自定义购物策略", "Custom Shopping Strategy").get(language),
                mirror_switch(
                    app,
                    cx,
                    MirrorBool::ShoppingStrategy,
                    config.shopping_strategy,
                    "shop-custom-shopping-strategy",
                ),
            ))
            .child(if config.shopping_strategy {
                strategy_select(
                    app,
                    cx,
                    TeamSelect::ShoppingStrategy,
                    config.shopping_strategy_select,
                    shopping_options,
                    "shop-shopping-strategy-select",
                    MirrorU8::ShoppingStrategySelect,
                )
                .into_any_element()
            } else {
                div().into_any_element()
            }),
    );

    let opening = editor_card(
        div()
            .flex()
            .flex_col()
            .gap_2()
            .child(control_row(
                text("自选开局饰品", "Custom Starting Gifts").get(language),
                mirror_switch(
                    app,
                    cx,
                    MirrorBool::OpeningItems,
                    config.opening_items,
                    "shop-opening-items",
                ),
            ))
            .child(if config.opening_items {
                editor_option_grid(vec![
                    control_row(
                        text("饰品体系", "Gift System").get(language),
                        strategy_select(
                            app,
                            cx,
                            TeamSelect::OpeningItemsSystem,
                            config.opening_items_system,
                            opening_system_options,
                            "shop-opening-items-system",
                            MirrorU8::OpeningItemsSystem,
                        ),
                    ),
                    control_row(
                        text("选择顺序", "Selection Order").get(language),
                        strategy_select(
                            app,
                            cx,
                            TeamSelect::OpeningItemsOrder,
                            config.opening_items_select,
                            opening_order_options,
                            "shop-opening-items-order",
                            MirrorU8::OpeningItemsSelect,
                        ),
                    ),
                ])
                .into_any_element()
            } else {
                div().into_any_element()
            }),
    );

    editor_card(
        div()
            .flex()
            .flex_col()
            .gap_2()
            .child(editor_section_title(
                text("奖励与开局策略", "Rewards & Starting Strategy").get(language),
            ))
            .child(reward)
            .child(shopping)
            .child(opening),
    )
}
