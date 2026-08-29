use super::super::*;

pub fn daily_details(app: &mut AhabApp, cx: &mut Context<AhabApp>, busy: bool) -> Div {
    let config = &app.home.tasks.daily_task;
    let language = app.state.settings.language;
    let tab = app.home.options_tab(FixedTaskId::DailyTask);
    let exp_count = daily_counter(
        config.set_EXP_count,
        0,
        99,
        DailyCounter::Exp,
        "daily-exp-count",
        busy,
        cx,
    );
    let thread_count = daily_counter(
        config.set_thread_count,
        0,
        99,
        DailyCounter::Thread,
        "daily-thread-count",
        busy,
        cx,
    );
    let continuous_count = daily_counter(
        config.use_continuous_combat_select,
        1,
        10,
        DailyCounter::Continuous,
        "daily-continuous-count",
        busy,
        cx,
    );
    let daily_team = daily_team_select(
        app,
        text("默认日常编队", "Default Daily Team").get(language),
        11,
        config,
        busy,
        cx,
    );
    let targeted_exp = task_option_switch(
        text("经验本针对性配队", "Targeted EXP Lineups").get(language),
        config.targeted_teaming_EXP,
        "daily-targeted-exp",
        busy,
        cx,
        |home| {
            home.update_tasks(|tasks| {
                tasks.daily_task.targeted_teaming_EXP = !tasks.daily_task.targeted_teaming_EXP
            })
        },
    );
    let targeted_thread = task_option_switch(
        text("纽本针对性配队", "Targeted Thread Lineups").get(language),
        config.targeted_teaming_thread,
        "daily-targeted-thread",
        busy,
        cx,
        |home| {
            home.update_tasks(|tasks| {
                tasks.daily_task.targeted_teaming_thread = !tasks.daily_task.targeted_teaming_thread
            })
        },
    );

    let mut continuous = div().flex().items_center().gap_2();
    if config.use_continuous_combat {
        continuous = continuous.child(continuous_count);
    }
    continuous = continuous.child(task_option_switch(
        "",
        config.use_continuous_combat,
        "daily-continuous-enabled",
        busy,
        cx,
        |home: &mut HomeState| {
            home.update_tasks(|tasks| {
                tasks.daily_task.use_continuous_combat = !tasks.daily_task.use_continuous_combat
            })
        },
    ));

    let general = div()
        .flex()
        .flex_col()
        .gap_1()
        .child(control_row(
            text("经验本次数（0~99）", "EXP Dungeon Runs (0-99)").get(language),
            exp_count,
        ))
        .child(control_row(
            text("纽本次数（0~99）", "Thread Dungeon Runs (0-99)").get(language),
            thread_count,
        ))
        .child(control_row(
            text("默认日常编队", "Default Daily Team").get(language),
            daily_team,
        ))
        .child(control_row(
            text("连续作战", "Continuous Combat").get(language),
            continuous,
        ));

    let mut advanced = div().flex().flex_col().gap_2();
    advanced = advanced
        .child(control_row(
            text("经验本按属性配队", "Targeted EXP Lineups").get(language),
            targeted_exp,
        ))
        .child(control_row(
            text("纽本按星期配队", "Targeted Thread Lineups").get(language),
            targeted_thread,
        ));
    if config.targeted_teaming_EXP || config.targeted_teaming_thread {
        advanced = advanced.child(
            div().text_size(px(11.)).text_color(rgb(TEXT_MUTED)).child(
                text(
                    "选择对应日期使用的队伍（点击按钮循环队伍）",
                    "Choose the team for each day (click to cycle teams)",
                )
                .get(language),
            ),
        );
        let mut selectors = div().flex().flex_wrap().gap_1();
        if config.targeted_teaming_EXP {
            for (label, index) in [
                (text("经验斩击", "Mon/Tue (Slash)"), 0_u8),
                (text("经验突刺", "Wed/Thu (Pierce)"), 1),
                (text("经验打击", "Fri/Sat (Blunt)"), 2),
                (text("经验全属性", "Sun (All)"), 3),
            ] {
                selectors = selectors.child(daily_team_select(
                    app,
                    label.get(language),
                    index,
                    config,
                    busy,
                    cx,
                ));
            }
        }
        if config.targeted_teaming_thread {
            for (label, index) in [
                (text("周一", "Mon (Lust)"), 4_u8),
                (text("周二", "Tue (Sloth)"), 5),
                (text("周三", "Wed (Gluttony)"), 6),
                (text("周四", "Thu (Gloom)"), 7),
                (text("周五", "Fri (Pride)"), 8),
                (text("周六", "Sat (Envy)"), 9),
                (text("周日", "Sun (Wrath)"), 10),
            ] {
                selectors = selectors.child(daily_team_select(
                    app,
                    label.get(language),
                    index,
                    config,
                    busy,
                    cx,
                ));
            }
        }
        advanced = advanced.child(selectors);
    }

    div()
        .flex()
        .flex_col()
        .gap_2()
        .child(options_tabs(FixedTaskId::DailyTask, tab, language, cx))
        .child(match tab {
            TaskOptionsTab::General => general,
            TaskOptionsTab::Advanced => advanced,
        })
}

fn daily_team_select(
    app: &AhabApp,
    label: &'static str,
    index: u8,
    config: &crate::model::DailyTaskConfig,
    busy: bool,
    cx: &mut Context<AhabApp>,
) -> Div {
    let value = match index {
        0 => config.EXP_day_1_2,
        1 => config.EXP_day_3_4,
        2 => config.EXP_day_5_6,
        3 => config.EXP_day_7,
        4 => config.thread_day_1,
        5 => config.thread_day_2,
        6 => config.thread_day_3,
        7 => config.thread_day_4,
        8 => config.thread_day_5,
        9 => config.thread_day_6,
        10 => config.thread_day_7,
        _ => config.daily_teams,
    };
    let language = app.state.settings.language;
    let mut options = app
        .teams
        .teams
        .iter()
        .filter_map(|team| {
            let number = team_number_from_id(&team.id)?;
            Some((number.to_string(), team.name.clone()))
        })
        .take(99)
        .collect::<Vec<_>>();
    if options.is_empty() {
        options.push((
            "1".to_owned(),
            text("编队 1", "Team 1").get(language).to_owned(),
        ));
    }
    let on_change = Rc::new(move |home: &mut HomeState, value: String| {
        if let Ok(value) = value.parse::<u8>() {
            home.set_daily_team(index, value);
        }
    });
    let select = home_select(
        app,
        cx,
        HomeSelectConfig {
            select: HomeSelect::DailyTeam(index),
            current: value.to_string(),
            options,
            id: format!("daily-team-{index}"),
            width: if index == 11 { 144. } else { 120. },
            disabled: busy,
            on_change,
        },
    );
    if index == 11 {
        select
    } else {
        div()
            .flex()
            .items_center()
            .justify_between()
            .gap_1()
            .child(
                div()
                    .min_w_0()
                    .flex_1()
                    .truncate()
                    .text_size(px(11.))
                    .text_color(rgb(TEXT_MUTED))
                    .child(label),
            )
            .child(select)
    }
}

fn team_number_from_id(id: &str) -> Option<u8> {
    let number = id.strip_prefix("team-")?.parse::<u8>().ok()?;
    (1..=99).contains(&number).then_some(number)
}

#[cfg(test)]
mod tests {
    use super::team_number_from_id;

    #[test]
    fn daily_team_ids_use_backend_numbers() {
        assert_eq!(team_number_from_id("team-7"), Some(7));
        assert_eq!(team_number_from_id("team-100"), None);
        assert_eq!(team_number_from_id("team-invalid"), None);
    }
}
