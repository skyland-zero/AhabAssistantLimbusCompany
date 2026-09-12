use super::*;

pub(crate) fn mirror_card(
    app: &mut AhabApp,
    cx: &mut Context<AhabApp>,
    busy: bool,
    executing: bool,
) -> Div {
    let enabled = app.home.tasks.enabledTasks.mirror;
    let language = app.state.settings.language;
    let expanded = app.home.is_expanded(FixedTaskId::Mirror);
    let config = &app.home.tasks.mirror;
    let number = if config.infinite_dungeons {
        badge(
            text("∞ 无限", "∞ Infinite").get(language),
            BadgeTone::Neutral,
        )
    } else {
        task_counter(
            config.set_mirror_count,
            1,
            99,
            "mirror-count",
            busy,
            cx,
            Rc::new(|home, delta| home.adjust_mirror_count(delta)),
        )
    };
    let infinite = task_option_switch(
        text("无限模式", "Infinite Mode").get(language),
        config.infinite_dungeons,
        "mirror-infinite",
        busy,
        cx,
        |home| {
            home.toggle_mirror_option(MirrorOption::Infinite);
        },
    );
    let hard = detail_switch(
        config.hard_mirror,
        FixedTaskId::Mirror,
        cx,
        busy,
        "hard-mirror",
    );
    let hard_floors = {
        let current_floors = config.hard_mirror_target_floors;
        let mut group = div()
            .flex()
            .items_center()
            .gap_1()
            .rounded_lg()
            .bg(rgb(SURFACE))
            .p_1();
        for (target, label) in [
            (5, text("5层速刷", "5F Fast")),
            (15, text("15层叠加", "15F Infinity")),
        ] {
            let is_selected = current_floors == target || (target == 5 && current_floors != 15);
            let mut btn = button(
                label.get(language),
                if is_selected {
                    ButtonVariant::Secondary
                } else {
                    ButtonVariant::Ghost
                },
            )
            .id(format!("mirror-hard-floors-{target}"))
            .px_2()
            .py_0p5()
            .text_size(px(11.0));
            if !busy {
                btn = btn.on_click(cx.listener(move |view, _, _, cx| {
                    view.home.set_hard_mirror_target_floors(target);
                    cx.notify();
                }));
            }
            group = group.child(btn);
        }
        group
    };
    let options = [
        (
            text("不使用每周加成", "Do Not Use Weekly Bonuses").get(language),
            config.no_weekly_bonuses,
            "mirror-no-weekly",
            MirrorOption::NoWeeklyBonuses,
        ),
        (
            text("只打三层", "Exit at Floor 3").get(language),
            config.floor_3_exit,
            "mirror-floor-three",
            MirrorOption::FloorThreeExit,
        ),
        (
            text("保存困牢奖励", "Save Hard Rewards").get(language),
            config.save_rewards,
            "mirror-save-rewards",
            MirrorOption::SaveRewards,
        ),
        (
            text("困牢单次加成", "Single Bonus per Run").get(language),
            config.hard_mirror_single_bonuses,
            "mirror-single-bonus",
            MirrorOption::HardSingleBonuses,
        ),
        (
            text("第 5 层选活动包", "Pick Event Pack on F5").get(language),
            config.select_event_pack,
            "mirror-select-event",
            MirrorOption::SelectEventPack,
        ),
        (
            text("第 5 层跳过活动包", "Skip Event Pack on F5").get(language),
            config.skip_event_pack,
            "mirror-skip-event",
            MirrorOption::SkipEventPack,
        ),
        (
            text("再次领取奖励", "Re-claim Rewards").get(language),
            config.re_claim_rewards,
            "mirror-reclaim",
            MirrorOption::ReclaimRewards,
        ),
        (
            text("不跳过白棉花", "Do Not Skip Gossypium").get(language),
            config.not_skip_whitegossypium,
            "mirror-cotton",
            MirrorOption::NotSkipCotton,
        ),
        (
            text("战斗直到全灭", "Fight to the Last Sinner").get(language),
            config.fight_to_last_man,
            "mirror-last-man",
            MirrorOption::FightToLast,
        ),
        (
            text("键盘寻路", "Keyboard Pathfinding").get(language),
            config.mirror_keyboard_navigation,
            "mirror-keyboard",
            MirrorOption::KeyboardNavigation,
        ),
        (
            text("简易键盘寻路", "Simple Keyboard Pathfinding").get(language),
            config.mirror_keyboard_simple_pathfinding,
            "mirror-simple-keyboard",
            MirrorOption::SimplePathfinding,
        ),
        (
            text(
                "F3/F4 优先 Hatred and Despair",
                "Prefer Hatred and Despair on F3/F4",
            )
            .get(language),
            config.mirror_prefer_hatred_and_despair,
            "mirror-hatred-and-despair",
            MirrorOption::PreferHatredAndDespair,
        ),
        (
            text("寻路优先最少非 Boss 战斗", "Minimize non-Boss combat").get(language),
            config.mirror_minimize_non_boss_combat,
            "mirror-minimize-non-boss-combat",
            MirrorOption::MinimizeNonBossCombat,
        ),
    ];
    let mut option_items = Vec::new();
    for (label, value, id, field) in options {
        let toggle = task_option_switch(label, value, id, busy, cx, move |home| {
            home.toggle_mirror_option(field);
        });
        option_items.push(control_row(label, toggle));
    }
    // 楼层后缀与卡片 chip 共用同一格式化入口，避免三处各写一遍双语分支。
    let floor_suffix = super::stats::mirror_floor_label(app.home.mirror_floor.as_ref(), language)
        .map(|label| format!(" · {label}"));
    let progress = app.home.mirror_progress.as_ref().map(|progress| {
        div()
            .flex()
            .items_center()
            .justify_between()
            .px_2()
            .py_1()
            .rounded_md()
            .bg(rgba((ACCENT << 8) | 0x22))
            .child(div().text_size(px(11.0)).text_color(rgb(ACCENT)).child({
                let base = if progress.isInfinite {
                    format!(
                        "{} {} / ∞",
                        text("镜牢进度", "Mirror Progress").get(language),
                        progress.current
                    )
                } else {
                    format!(
                        "{} {} / {}",
                        text("镜牢进度", "Mirror Progress").get(language),
                        progress.current,
                        progress.total
                    )
                };
                match &floor_suffix {
                    Some(suffix) => format!("{base}{suffix}"),
                    None => base,
                }
            }))
            .child(badge(
                if progress.isHard {
                    if config.hard_mirror_target_floors == 15 {
                        text("困难·15层", "Hard (15F)").get(language)
                    } else {
                        text("困难·5层", "Hard (5F)").get(language)
                    }
                } else {
                    text("普通", "Normal").get(language)
                },
                BadgeTone::Accent,
            ))
    });
    let tab = app.home.options_tab(FixedTaskId::Mirror);
    let mut general_controls = vec![
        control_row(text("运行次数", "Run Count").get(language), number),
        control_row(text("无限模式", "Infinite Mode").get(language), infinite),
        control_row(text("困难镜牢", "Hard Mirror Dungeon").get(language), hard),
    ];
    if config.hard_mirror {
        general_controls.push(control_row(
            text("困牢目标", "Hard Target").get(language),
            hard_floors,
        ));
    }
    let general = div()
        .flex()
        .flex_col()
        .gap_2()
        .children(progress)
        .child(adaptive_settings_grid(language, general_controls));
    let advanced = div()
        .pt_2()
        .border_t_1()
        .border_color(rgb(BORDER))
        .child(adaptive_settings_grid(language, option_items));
    let body = div()
        .flex()
        .flex_col()
        .gap_2()
        .child(options_tabs(
            FixedTaskId::Mirror,
            tab,
            app.state.settings.language,
            cx,
        ))
        .child(match tab {
            TaskOptionsTab::General => general,
            TaskOptionsTab::Advanced => advanced,
        });
    task_card_with_toggle(
        cx,
        busy,
        TaskCardSpec {
            task: FixedTaskId::Mirror,
            title: task_title(FixedTaskId::Mirror, language).to_owned(),
            icon: "CMP",
            enabled,
            expanded,
            executing,
            preview_tags: vec![
                preview_tag(
                    text("轮次", "Runs").get(language),
                    if config.infinite_dungeons {
                        "∞".to_owned()
                    } else {
                        format!("{}", config.set_mirror_count)
                    },
                    config.infinite_dungeons,
                ),
                preview_tag(
                    text("难度", "Difficulty").get(language),
                    if config.hard_mirror {
                        if config.hard_mirror_target_floors == 15 {
                            text("困难·15层", "Hard (15F)").get(language)
                        } else {
                            text("困难·5层", "Hard (5F)").get(language)
                        }
                    } else {
                        text("普通", "Normal").get(language)
                    },
                    config.hard_mirror,
                ),
            ],
            body: Some(body),
        },
    )
}
