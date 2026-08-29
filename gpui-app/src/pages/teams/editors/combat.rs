use super::*;

pub(crate) fn combat_editor(
    app: &mut AhabApp,
    cx: &mut Context<AhabApp>,
    config: &TeamMirrorConfig,
    language: Language,
) -> Div {
    let second_system = card(
        div()
            .flex()
            .flex_col()
            .gap_3()
            .child(control_row(
                text("启用第二体系", "Enable Second System").get(language),
                mirror_switch(
                    app,
                    cx,
                    MirrorBool::SecondSystem,
                    config.second_system,
                    "combat-second-system",
                ),
            ))
            .child(if config.second_system {
                div()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .child(control_row(
                        text("第二体系", "Secondary System").get(language),
                        team_select(
                            app,
                            cx,
                            TeamSelectConfig {
                                select: TeamSelect::SecondSystem,
                                current: config.second_system_select.to_string(),
                                options: SYSTEM_NAMES
                                    .iter()
                                    .enumerate()
                                    .map(|(index, _)| {
                                        (
                                            index.to_string(),
                                            system_label(index, language).to_owned(),
                                        )
                                    })
                                    .collect(),
                                id: "combat-second-system-select".to_owned(),
                                width: 180.,
                                on_change: Rc::new(|teams, value| {
                                    if let Ok(value) = value.parse::<u8>() {
                                        teams.set_mirror_u8(MirrorU8::SecondSystemSelect, value);
                                    }
                                }),
                            },
                        ),
                    ))
                    .child(control_row(
                        text("起始楼层", "Start Floor").get(language),
                        team_select(
                            app,
                            cx,
                            TeamSelectConfig {
                                select: TeamSelect::SecondSystemFloor,
                                current: config.second_system_setting.to_string(),
                                options: (0..=1)
                                    .map(|value| {
                                        (
                                            value.to_string(),
                                            second_system_setting_label(value, language).to_owned(),
                                        )
                                    })
                                    .collect(),
                                id: "combat-second-system-floor".to_owned(),
                                width: 180.,
                                on_change: Rc::new(|teams, value| {
                                    if let Ok(value) = value.parse::<u8>() {
                                        teams
                                            .set_mirror_u8(MirrorU8::SecondSystemStartFloor, value);
                                    }
                                }),
                            },
                        ),
                    ))
                    .child(control_row(
                        text("四级合成", "Fuse Tier 4").get(language),
                        mirror_switch(
                            app,
                            cx,
                            MirrorBool::SecondSystemFuseIv,
                            config.second_system_fuse_IV,
                            "combat-fuse-iv",
                        ),
                    ))
                    .child(control_row(
                        text("购买饰品", "Buy Gifts").get(language),
                        mirror_switch(
                            app,
                            cx,
                            MirrorBool::SecondSystemBuy,
                            config.second_system_buy,
                            "combat-buy",
                        ),
                    ))
                    .child(control_row(
                        text("选择奖励", "Select Rewards").get(language),
                        mirror_switch(
                            app,
                            cx,
                            MirrorBool::SecondSystemSelectReward,
                            config.second_system_select_reward,
                            "combat-reward",
                        ),
                    ))
                    .child(control_row(
                        text("升级饰品", "Upgrade Gifts").get(language),
                        mirror_switch(
                            app,
                            cx,
                            MirrorBool::SecondSystemPowerUp,
                            config.second_system_power_up,
                            "combat-power-up",
                        ),
                    ))
            } else {
                div().text_size(px(11.)).text_color(rgb(TEXT_MUTED)).child(
                    text(
                        "开启后可配置第二体系和起始楼层。",
                        "Enable this to configure the secondary system and start floor.",
                    )
                    .get(language),
                )
            }),
    );

    let preferences = card(
        div()
            .flex()
            .flex_col()
            .gap_2()
            .child(
                div()
                    .text_size(px(13.))
                    .text_color(rgb(TEXT))
                    .child(text("战斗与技能偏好", "Combat & Skill Preferences").get(language)),
            )
            .child(control_row(
                text("避免三技能", "Avoid Skill 3").get(language),
                mirror_switch(
                    app,
                    cx,
                    MirrorBool::AvoidSkill3,
                    config.avoid_skill_3,
                    "combat-avoid-skill-3",
                ),
            ))
            .child(control_row(
                text("优先三技能", "Prioritize Skill 3").get(language),
                mirror_switch(
                    app,
                    cx,
                    MirrorBool::PrioritizeSkill3,
                    config.prioritize_skill_3,
                    "combat-prioritize-skill-3",
                ),
            ))
            .child(control_row(
                text("每层重新编队", "Re-form Team Each Floor").get(language),
                mirror_switch(
                    app,
                    cx,
                    MirrorBool::ReformationEachFloor,
                    config.re_formation_each_floor,
                    "combat-reformation",
                ),
            )),
    );

    let defense = card(
        div()
            .flex()
            .flex_col()
            .gap_2()
            .child(
                div().text_size(px(13.)).text_color(rgb(TEXT)).child(
                    text(
                        "防御策略（互斥）",
                        "Defense Strategies (Mutually Exclusive)",
                    )
                    .get(language),
                ),
            )
            .child(control_row(
                text("首回合防御", "Defend in Round 1").get(language),
                mirror_switch(
                    app,
                    cx,
                    MirrorBool::DefenseFirstRound,
                    config.defense_first_round,
                    "combat-defense-first",
                ),
            ))
            .child(control_row(
                text("良秀单通防御", "Solo Ryoshu Defense").get(language),
                mirror_switch(
                    app,
                    cx,
                    MirrorBool::DefenseForSolo,
                    config.defense_for_solo,
                    "combat-defense-solo",
                ),
            ))
            .child(if config.defense_for_solo {
                control_row(
                    text("防御回合", "Defense Turns").get(language),
                    team_select(
                        app,
                        cx,
                        TeamSelectConfig {
                            select: TeamSelect::DefenseTurns,
                            current: config.defense_for_solo_turns.to_string(),
                            options: (1..=5)
                                .map(|turns| (turns.to_string(), turns_label(turns, language)))
                                .collect(),
                            id: "combat-defense-turns".to_owned(),
                            width: 180.,
                            on_change: Rc::new(|teams, value| {
                                if let Ok(value) = value.parse::<u8>() {
                                    teams.set_mirror_u8(MirrorU8::DefenseTurns, value);
                                }
                            }),
                        },
                    ),
                )
                .into_any_element()
            } else {
                div().into_any_element()
            }),
    );

    let replacement = card(
        div()
            .flex()
            .flex_col()
            .gap_2()
            .child(control_row(
                text("启用技能替换", "Enable Skill Replacement").get(language),
                mirror_switch(
                    app,
                    cx,
                    MirrorBool::SkillReplacement,
                    config.skill_replacement,
                    "combat-skill-replacement",
                ),
            ))
            .child(if config.skill_replacement {
                div().flex().flex_col().gap_2().child(control_row(
                    text("替换模式", "Replacement Mode").get(language),
                    team_select(
                        app,
                        cx,
                        TeamSelectConfig {
                            select: TeamSelect::SkillReplacementMode,
                            current: config.skill_replacement_mode.to_string(),
                            options: vec![
                                ("0".to_owned(), "1 → 2".to_owned()),
                                ("1".to_owned(), "1 → 3".to_owned()),
                            ],
                            id: "combat-skill-replacement-mode".to_owned(),
                            width: 180.,
                            on_change: Rc::new(|teams, value| {
                                if let Ok(value) = value.parse::<u8>() {
                                    teams.set_mirror_u8(MirrorU8::SkillReplacementMode, value);
                                }
                            }),
                        },
                    ),
                ))
            } else {
                div()
            }),
    );

    div()
        .flex()
        .flex_col()
        .gap_4()
        .child(second_system)
        .child(settings_grid(vec![preferences, defense, replacement], 220.))
}
