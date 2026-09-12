use super::*;

pub(super) fn set_windows_card(
    app: &mut AhabApp,
    cx: &mut Context<AhabApp>,
    busy: bool,
    executing: bool,
) -> Div {
    let config = &app.home.tasks.set_windows;
    let size = config.set_win_size;
    let use_post_message = config.use_post_message;
    let language = app.state.settings.language;
    let expanded = app.home.is_expanded(FixedTaskId::SetWindows);
    let body = super::task_details::set_windows_details(app, cx, busy);
    task_card(
        cx,
        TaskCardSpec {
            task: FixedTaskId::SetWindows,
            title: text("窗口设置", "Window Settings").get(language).to_owned(),
            icon: "SET",
            enabled: true,
            expanded,
            executing,
            preview_tags: vec![
                preview_tag(
                    text("分辨率", "Resolution").get(language),
                    format!("{size}P"),
                    false,
                ),
                preview_tag(
                    text("异步输入", "Async Input").get(language),
                    if use_post_message {
                        text("开", "On").get(language)
                    } else {
                        text("关", "Off").get(language)
                    },
                    use_post_message,
                ),
            ],
            body: Some(body),
        },
        None,
    )
}

pub(super) fn daily_card(
    app: &mut AhabApp,
    cx: &mut Context<AhabApp>,
    busy: bool,
    executing: bool,
) -> Div {
    let enabled = app.home.tasks.enabledTasks.daily_task;
    let expanded = app.home.is_expanded(FixedTaskId::DailyTask);
    let language = app.state.settings.language;
    let config = &app.home.tasks.daily_task;
    let previews = vec![
        preview_tag(
            text("经验本", "EXP").get(language),
            format!("×{}", config.set_EXP_count),
            false,
        ),
        preview_tag(
            text("纽本", "Thread").get(language),
            format!("×{}", config.set_thread_count),
            false,
        ),
        preview_tag(
            text("连战", "Chain").get(language),
            if config.use_continuous_combat {
                format!("×{}", config.use_continuous_combat_select)
            } else {
                text("关", "Off").get(language).to_owned()
            },
            config.use_continuous_combat,
        ),
    ];
    let body = super::task_details::daily_details(app, cx, busy);
    task_card_with_toggle(
        cx,
        busy,
        TaskCardSpec {
            task: FixedTaskId::DailyTask,
            title: text("日常任务", "Daily Tasks").get(language).to_owned(),
            icon: "CAL",
            enabled,
            expanded,
            executing,
            preview_tags: previews,
            body: Some(body),
        },
    )
}

pub(super) fn reward_card(
    app: &mut AhabApp,
    cx: &mut Context<AhabApp>,
    busy: bool,
    executing: bool,
) -> Div {
    let enabled = app.home.tasks.enabledTasks.get_reward;
    let language = app.state.settings.language;
    let mode = app.home.tasks.get_reward.set_get_prize;
    let expanded = app.home.is_expanded(FixedTaskId::GetReward);
    let mode_select = home_select(
        app,
        cx,
        HomeSelectConfig {
            select: HomeSelect::RewardMode,
            current: mode.to_string(),
            options: vec![
                (
                    "0".to_owned(),
                    text(
                        "全部领取 (狂气 + 通行证 + 邮件)",
                        "Claim All (Lunacy + Pass + Mail)",
                    )
                    .get(language)
                    .to_owned(),
                ),
                (
                    "1".to_owned(),
                    text("狂气与通行证奖励", "Lunacy & Pass Rewards")
                        .get(language)
                        .to_owned(),
                ),
                (
                    "2".to_owned(),
                    text("仅领取邮件奖励", "Mail Rewards Only")
                        .get(language)
                        .to_owned(),
                ),
            ],
            id: "reward-mode".to_owned(),
            width: 176.,
            disabled: busy,
            on_change: Rc::new(|home, value| {
                if let Ok(value) = value.parse::<u8>() {
                    home.set_reward_mode(value);
                }
            }),
        },
    );
    let body = div().flex().flex_col().gap_2().child(control_row(
        text("领取模式", "Claim Mode").get(language),
        mode_select,
    ));
    task_card_with_toggle(
        cx,
        busy,
        TaskCardSpec {
            task: FixedTaskId::GetReward,
            title: text("领取奖励", "Claim Rewards").get(language).to_owned(),
            icon: "GFT",
            enabled,
            expanded,
            executing,
            preview_tags: vec![preview_tag(
                text("模式", "Mode").get(language),
                panel::reward_mode_label(mode, language),
                false,
            )],
            body: Some(body),
        },
    )
}

pub(super) fn enkephalin_card(
    app: &mut AhabApp,
    cx: &mut Context<AhabApp>,
    busy: bool,
    executing: bool,
) -> Div {
    let enabled = app.home.tasks.enabledTasks.buy_enkephalin;
    let language = app.state.settings.language;
    let expanded = app.home.is_expanded(FixedTaskId::BuyEnkephalin);
    let config = app.home.tasks.buy_enkephalin.clone();
    let body = super::task_details::enkephalin_details(app, cx, busy);
    task_card_with_toggle(
        cx,
        busy,
        TaskCardSpec {
            task: FixedTaskId::BuyEnkephalin,
            title: text("狂气换体", "Refill Enkephalin")
                .get(language)
                .to_owned(),
            icon: "ZAP",
            enabled,
            expanded,
            executing,
            preview_tags: vec![
                preview_tag(
                    text("换体", "Refills").get(language),
                    config.set_lunacy_to_enkephalin.to_string(),
                    false,
                ),
                preview_tag(
                    text("葛朗台", "Grandet").get(language),
                    if config.Dr_Grandet_mode {
                        text("开", "On").get(language)
                    } else {
                        text("关", "Off").get(language)
                    },
                    config.Dr_Grandet_mode,
                ),
            ],
            body: Some(body),
        },
    )
}

pub(super) fn ahab_card(
    app: &mut AhabApp,
    cx: &mut Context<AhabApp>,
    busy: bool,
    executing: bool,
) -> Div {
    let enabled = app.home.tasks.enabledTasks.resonate_with_Ahab;
    let language = app.state.settings.language;
    task_card_with_toggle(
        cx,
        busy,
        TaskCardSpec {
            task: FixedTaskId::ResonateWithAhab,
            title: text("亚哈共鸣", "Ahab Resonance").get(language).to_owned(),
            icon: "RAD",
            enabled,
            expanded: false,
            executing,
            preview_tags: vec![preview_tag(
                text("语录", "Quote").get(language),
                if enabled {
                    text("开启", "Enabled").get(language)
                } else {
                    text("关闭", "Disabled").get(language)
                },
                enabled,
            )],
            body: None,
        },
    )
}

mod mirror;

pub(super) use mirror::mirror_card;
