//! Home page surface: task configuration, execution controls, and the
//! connection/log side panel. All mutable page state lives in HomeState.

use std::time::Duration;

use gpui::{Animation, AnimationExt, Context, Div, Render, Window, div, prelude::*, px, rgb, rgba};

use crate::{
    app::{ACCENT, AhabApp, BACKGROUND, BORDER, SURFACE, SURFACE_HOVER, TEXT, TEXT_MUTED},
    components::{BadgeTone, ButtonVariant, badge, button, card, scroll_area_with_id, switch},
    i18n::{self, Key as I18nKey},
    model::{AfterExitAction, AfterPowerAction, ConnectionStatus, ExecutionState, FixedTaskId},
    state::{HomeState, MirrorOption},
};

const RIGHT_PANEL_DEFAULT_WIDTH: f32 = 280.0;
const RIGHT_PANEL_MIN_WIDTH: f32 = 280.0;
const RIGHT_PANEL_MAX_WIDTH: f32 = 800.0;
const SPLITTER_WIDTH: f32 = 4.0;
const SPLITTER_COLLAPSED_WIDTH: f32 = 16.0;
const SPLITTER_COLLAPSE_THRESHOLD: f32 = 160.0;
const MIN_LEFT_PANEL_WIDTH: f32 = 460.0;
struct SplitterDragGhost;

impl Render for SplitterDragGhost {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div().w(px(1.0)).h(px(1.0)).bg(rgb(ACCENT))
    }
}

pub fn render(app: &mut AhabApp, cx: &mut Context<AhabApp>) -> Div {
    let busy = app.home.is_busy();
    let execution_state = app.home.execution.state;
    // The mock start call intentionally leaves currentTaskId optional. Use
    // the first executable selection for the presentation only so the
    // running sweep still exercises the same visual state as the React page.
    let current_task = app.home.execution.currentTaskId.or_else(|| {
        (execution_state == ExecutionState::Running)
            .then(|| first_executable_task(&app.home))
            .flatten()
    });

    let task_cards = vec![
        set_windows_card(app, cx, busy, current_task == Some(FixedTaskId::SetWindows)),
        daily_card(app, cx, busy, current_task == Some(FixedTaskId::DailyTask)),
        reward_card(app, cx, busy, current_task == Some(FixedTaskId::GetReward)),
        enkephalin_card(
            app,
            cx,
            busy,
            current_task == Some(FixedTaskId::BuyEnkephalin),
        ),
        mirror_card(app, cx, busy, current_task == Some(FixedTaskId::Mirror)),
        ahab_card(
            app,
            cx,
            busy,
            current_task == Some(FixedTaskId::ResonateWithAhab),
        ),
    ];

    let task_list = div()
        .id("home-task-scroll")
        .overflow_y_scroll()
        .scrollbar_width(px(6.))
        .flex_1()
        .min_h_0()
        .px(px(14.0))
        .py(px(10.0))
        .child(div().flex().flex_col().gap_2().pb_2().children(task_cards));

    let left_panel = div()
        .flex_1()
        .min_w_0()
        .min_h_0()
        .flex()
        .flex_col()
        .border_r_1()
        .border_color(rgb(BORDER))
        .child(task_header(
            execution_state,
            current_task,
            app.state.settings.language,
        ))
        .child(task_list)
        .child(execution_toolbar(app, cx, busy, execution_state));

    let splitter = splitter(app, cx);
    let right = if app.home.right_panel_collapsed {
        div()
    } else {
        right_panel(app, cx)
    };

    div()
        .relative()
        .w_full()
        .flex_1()
        .h_full()
        .min_w_0()
        .min_h_0()
        .flex()
        .overflow_hidden()
        .bg(rgb(BACKGROUND))
        .child(left_panel)
        .child(splitter)
        .child(right)
        .child(after_completion_editor(app, cx, busy))
}

fn task_header(
    state: ExecutionState,
    current_task: Option<FixedTaskId>,
    language: crate::model::Language,
) -> Div {
    let (label, tone) = match state {
        ExecutionState::Running => (
            current_task
                .map(|task| {
                    format!(
                        "{}: {}",
                        i18n::text(language, I18nKey::HomeRunning),
                        task_title(task)
                    )
                })
                .unwrap_or_else(|| i18n::text(language, I18nKey::HomeRunning).to_owned()),
            BadgeTone::Accent,
        ),
        ExecutionState::Paused => (
            i18n::text(language, I18nKey::HomePaused).to_owned(),
            BadgeTone::Accent,
        ),
        ExecutionState::Idle => (
            i18n::text(language, I18nKey::HomeIdle).to_owned(),
            BadgeTone::Neutral,
        ),
    };

    div()
        .flex_none()
        .flex()
        .items_center()
        .justify_between()
        .px(px(14.0))
        .py(px(8.0))
        .bg(rgba((SURFACE << 8) | 0x4d))
        .child(
            div()
                .text_size(px(14.0))
                .text_color(rgb(TEXT))
                .child(i18n::text(language, I18nKey::HomeTitle)),
        )
        .child(badge(label, tone))
}

fn set_windows_card(
    app: &mut AhabApp,
    cx: &mut Context<AhabApp>,
    busy: bool,
    executing: bool,
) -> Div {
    let size = app.home.tasks.set_windows.set_win_size;
    let use_post_message = app.home.tasks.set_windows.use_post_message;
    let expanded = app.home.is_expanded(FixedTaskId::SetWindows);
    let mut size_button = button(format!("{size}P"), ButtonVariant::Outline).id("window-size");
    let reduced = app.home.tasks.set_windows.set_reduce_miscontact;
    let mut reduced_switch = switch(reduced).id("reduce-miscontact");
    if !busy {
        size_button = size_button.on_click(cx.listener(|view, _, _, cx| {
            view.home.cycle_number(FixedTaskId::SetWindows);
            cx.stop_propagation();
            cx.notify();
        }));
        reduced_switch = reduced_switch.on_click(cx.listener(|view, _, _, cx| {
            view.home.toggle_detail(FixedTaskId::SetWindows);
            cx.stop_propagation();
            cx.notify();
        }));
    }
    let body = div()
        .flex()
        .flex_col()
        .gap_2()
        .child(control_row("窗口分辨率", size_button))
        .child(control_row("结束后恢复窗口", reduced_switch))
        .child(set_windows_details(app, cx, busy));
    task_card(
        cx,
        FixedTaskId::SetWindows,
        "窗口设置",
        "SET",
        true,
        expanded,
        executing,
        vec![
            preview_tag("分辨率", format!("{size}P"), false),
            preview_tag(
                "异步输入",
                if use_post_message { "开" } else { "关" },
                use_post_message,
            ),
        ],
        Some(body),
        None,
    )
}

fn set_windows_details(app: &mut AhabApp, cx: &mut Context<AhabApp>, busy: bool) -> Div {
    let config = &app.home.tasks.set_windows;
    let position = match config.set_win_position.as_str() {
        "0" => "屏幕居中",
        "1" => "靠左对齐",
        "2" => "靠右对齐",
        _ => "保持原位",
    };
    let next_position = match config.set_win_position.as_str() {
        "0" => "1",
        "1" => "2",
        "2" => "3",
        _ => "0",
    }
    .to_owned();
    let mut position_button = button(position, ButtonVariant::Outline).id("window-position");
    let screenshot = config.screenshot_interval;
    let next_screenshot = next_interval(screenshot, &[0.2, 0.5, 1.0]);
    let mut screenshot_button =
        button(format!("{screenshot:.1}s"), ButtonVariant::Outline).id("screenshot-interval");
    let mouse = config.mouse_action_interval;
    let next_mouse = next_interval(mouse, &[0.1, 0.3, 0.5]);
    let mut mouse_button =
        button(format!("{mouse:.1}s"), ButtonVariant::Outline).id("mouse-interval");
    let mut post_message = switch(config.use_post_message).id("post-message");
    if !busy {
        position_button = position_button.on_click(cx.listener(move |view, _, _, cx| {
            view.home
                .update_tasks(|tasks| tasks.set_windows.set_win_position = next_position.clone());
            cx.stop_propagation();
            cx.notify();
        }));
        screenshot_button = screenshot_button.on_click(cx.listener(move |view, _, _, cx| {
            view.home
                .update_tasks(|tasks| tasks.set_windows.screenshot_interval = next_screenshot);
            cx.stop_propagation();
            cx.notify();
        }));
        mouse_button = mouse_button.on_click(cx.listener(move |view, _, _, cx| {
            view.home
                .update_tasks(|tasks| tasks.set_windows.mouse_action_interval = next_mouse);
            cx.stop_propagation();
            cx.notify();
        }));
        post_message = post_message.on_click(cx.listener(|view, _, _, cx| {
            view.home.update_tasks(|tasks| {
                tasks.set_windows.use_post_message = !tasks.set_windows.use_post_message
            });
            cx.stop_propagation();
            cx.notify();
        }));
    }
    div()
        .flex()
        .flex_col()
        .gap_2()
        .pt_2()
        .border_t_1()
        .border_color(rgb(BORDER))
        .child(control_row("窗口位置", position_button))
        .child(control_row("截图间隔", screenshot_button))
        .child(control_row("鼠标操作间隔", mouse_button))
        .child(control_row("异步 PostMessage 输入", post_message))
}

fn daily_card(app: &mut AhabApp, cx: &mut Context<AhabApp>, busy: bool, executing: bool) -> Div {
    let enabled = app.home.tasks.enabledTasks.daily_task;
    let expanded = app.home.is_expanded(FixedTaskId::DailyTask);
    let config = &app.home.tasks.daily_task;
    let previews = vec![
        preview_tag("经验本", format!("×{}", config.set_EXP_count), false),
        preview_tag("纽本", format!("×{}", config.set_thread_count), false),
        preview_tag(
            "连战",
            if config.use_continuous_combat {
                format!("×{}", config.use_continuous_combat_select)
            } else {
                "关".to_owned()
            },
            config.use_continuous_combat,
        ),
    ];
    let body = daily_details(app, cx, busy);
    task_card_with_toggle(
        cx,
        busy,
        FixedTaskId::DailyTask,
        "日常任务",
        "CAL",
        enabled,
        expanded,
        executing,
        previews,
        Some(body),
    )
}

fn daily_details(app: &mut AhabApp, cx: &mut Context<AhabApp>, busy: bool) -> Div {
    let config = &app.home.tasks.daily_task;
    let exp_count = action_button(
        format!("×{}", config.set_EXP_count),
        "daily-exp-count",
        ButtonVariant::Outline,
        busy,
        cx,
        |home| {
            home.update_tasks(|tasks| {
                tasks.daily_task.set_EXP_count = (tasks.daily_task.set_EXP_count + 1).min(99)
            });
        },
    );
    let thread_count = action_button(
        format!("×{}", config.set_thread_count),
        "daily-thread-count",
        ButtonVariant::Outline,
        busy,
        cx,
        |home| {
            home.update_tasks(|tasks| {
                tasks.daily_task.set_thread_count = (tasks.daily_task.set_thread_count + 1).min(99)
            });
        },
    );
    let continuous_count = action_button(
        format!("{} 连战", config.use_continuous_combat_select),
        "daily-continuous-count",
        ButtonVariant::Outline,
        busy,
        cx,
        |home| {
            home.update_tasks(|tasks| {
                tasks.daily_task.use_continuous_combat_select =
                    if tasks.daily_task.use_continuous_combat_select >= 10 {
                        1
                    } else {
                        tasks.daily_task.use_continuous_combat_select + 1
                    };
            });
        },
    );
    let targeted_exp = task_option_switch(
        "经验本针对性配队",
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
        "纽本针对性配队",
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
    let mut body = div()
        .flex()
        .flex_col()
        .gap_1()
        .child(control_row("经验本次数（0~99）", exp_count))
        .child(control_row("纽本次数（0~99）", thread_count))
        .child(control_row("连续次数（1~10）", continuous_count))
        .child(control_row("经验本按属性配队", targeted_exp))
        .child(control_row("纽本按星期配队", targeted_thread));
    if config.targeted_teaming_EXP || config.targeted_teaming_thread {
        body = body.child(
            div()
                .text_size(px(10.))
                .text_color(rgb(TEXT_MUTED))
                .child("周属性队伍：点击下方数字循环 1~3（Mock 队伍）"),
        );
        let mut selectors = div().flex().flex_wrap().gap_1();
        let exp_fields = [
            ("经验斩击", 0_u8),
            ("经验突刺", 1),
            ("经验打击", 2),
            ("经验全属性", 3),
        ];
        if config.targeted_teaming_EXP {
            for (label, index) in exp_fields {
                selectors = selectors.child(daily_team_cycle(label, index, config, busy, cx));
            }
        }
        let thread_fields = [
            ("周一", 0_u8),
            ("周二", 1),
            ("周三", 2),
            ("周四", 3),
            ("周五", 4),
            ("周六", 5),
            ("周日", 6),
        ];
        if config.targeted_teaming_thread {
            for (label, index) in thread_fields {
                selectors = selectors.child(daily_team_cycle(label, index + 4, config, busy, cx));
            }
        }
        body = body.child(selectors);
    }
    body
}

fn daily_team_cycle(
    label: &'static str,
    index: u8,
    config: &crate::model::DailyTaskConfig,
    busy: bool,
    cx: &mut Context<AhabApp>,
) -> gpui::Stateful<Div> {
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
        _ => config.thread_day_7,
    };
    action_button(
        format!("{label} · 队伍 {value}"),
        format!("daily-team-{index}"),
        ButtonVariant::Ghost,
        busy,
        cx,
        move |home| {
            home.update_tasks(|tasks| {
                let next = if value >= 3 { 1 } else { value + 1 };
                match index {
                    0 => tasks.daily_task.EXP_day_1_2 = next,
                    1 => tasks.daily_task.EXP_day_3_4 = next,
                    2 => tasks.daily_task.EXP_day_5_6 = next,
                    3 => tasks.daily_task.EXP_day_7 = next,
                    4 => tasks.daily_task.thread_day_1 = next,
                    5 => tasks.daily_task.thread_day_2 = next,
                    6 => tasks.daily_task.thread_day_3 = next,
                    7 => tasks.daily_task.thread_day_4 = next,
                    8 => tasks.daily_task.thread_day_5 = next,
                    9 => tasks.daily_task.thread_day_6 = next,
                    _ => tasks.daily_task.thread_day_7 = next,
                }
            });
        },
    )
}

fn reward_card(app: &mut AhabApp, cx: &mut Context<AhabApp>, busy: bool, executing: bool) -> Div {
    let enabled = app.home.tasks.enabledTasks.get_reward;
    let mode = app.home.tasks.get_reward.set_get_prize;
    let expanded = app.home.is_expanded(FixedTaskId::GetReward);
    let number = number_button(
        reward_mode_label(mode).to_owned(),
        FixedTaskId::GetReward,
        cx,
        busy,
    );
    let body = div()
        .flex()
        .flex_col()
        .gap_2()
        .child(control_row("领取模式", number));
    task_card_with_toggle(
        cx,
        busy,
        FixedTaskId::GetReward,
        "领取奖励",
        "GFT",
        enabled,
        expanded,
        executing,
        vec![preview_tag("模式", reward_mode_label(mode), false)],
        Some(body),
    )
}

fn enkephalin_card(
    app: &mut AhabApp,
    cx: &mut Context<AhabApp>,
    busy: bool,
    executing: bool,
) -> Div {
    let enabled = app.home.tasks.enabledTasks.buy_enkephalin;
    let expanded = app.home.is_expanded(FixedTaskId::BuyEnkephalin);
    let config = &app.home.tasks.buy_enkephalin;
    let number = number_button(
        format!("{} 次", config.set_lunacy_to_enkephalin),
        FixedTaskId::BuyEnkephalin,
        cx,
        busy,
    );
    let detail = detail_switch(
        config.Dr_Grandet_mode,
        FixedTaskId::BuyEnkephalin,
        cx,
        busy,
        "grandet-mode",
    );
    let mut skip = switch(config.skip_enkephalin).id("skip-enkephalin");
    if !busy {
        skip = skip.on_click(cx.listener(|view, _, _, cx| {
            view.home.update_tasks(|tasks| {
                tasks.buy_enkephalin.skip_enkephalin = !tasks.buy_enkephalin.skip_enkephalin
            });
            cx.stop_propagation();
            cx.notify();
        }));
    }
    let body = div()
        .flex()
        .flex_col()
        .gap_2()
        .child(control_row("换体次数", number))
        .child(control_row("葛朗台模式", detail))
        .child(
            div()
                .pt_2()
                .border_t_1()
                .border_color(rgb(BORDER))
                .child(control_row("跳过模块合成", skip))
                .child(
                    div()
                        .text_size(px(10.0))
                        .text_color(rgb(TEXT_MUTED))
                        .child("除狂气换体外，不自动将多余体力合成为脑啡肽模块。"),
                ),
        );
    task_card_with_toggle(
        cx,
        busy,
        FixedTaskId::BuyEnkephalin,
        "狂气换体",
        "ZAP",
        enabled,
        expanded,
        executing,
        vec![
            preview_tag(
                "换体",
                format!("{}次", config.set_lunacy_to_enkephalin),
                false,
            ),
            preview_tag(
                "葛朗台",
                if config.Dr_Grandet_mode { "开" } else { "关" },
                config.Dr_Grandet_mode,
            ),
        ],
        Some(body),
    )
}

fn mirror_card(app: &mut AhabApp, cx: &mut Context<AhabApp>, busy: bool, executing: bool) -> Div {
    let enabled = app.home.tasks.enabledTasks.mirror;
    let expanded = app.home.is_expanded(FixedTaskId::Mirror);
    let config = &app.home.tasks.mirror;
    let number = number_button(
        format!("{} 次", config.set_mirror_count),
        FixedTaskId::Mirror,
        cx,
        busy,
    );
    let infinite = task_option_switch(
        "无限模式",
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
    let options = [
        (
            "不使用每周加成",
            config.no_weekly_bonuses,
            "mirror-no-weekly",
            MirrorOption::NoWeeklyBonuses,
        ),
        (
            "只打三层",
            config.floor_3_exit,
            "mirror-floor-three",
            MirrorOption::FloorThreeExit,
        ),
        (
            "保存困牢奖励",
            config.save_rewards,
            "mirror-save-rewards",
            MirrorOption::SaveRewards,
        ),
        (
            "困牢单次加成",
            config.hard_mirror_single_bonuses,
            "mirror-single-bonus",
            MirrorOption::HardSingleBonuses,
        ),
        (
            "第 5 层选活动包",
            config.select_event_pack,
            "mirror-select-event",
            MirrorOption::SelectEventPack,
        ),
        (
            "第 5 层跳过活动包",
            config.skip_event_pack,
            "mirror-skip-event",
            MirrorOption::SkipEventPack,
        ),
        (
            "再次领取奖励",
            config.re_claim_rewards,
            "mirror-reclaim",
            MirrorOption::ReclaimRewards,
        ),
        (
            "不跳过白棉花",
            config.not_skip_whitegossypium,
            "mirror-cotton",
            MirrorOption::NotSkipCotton,
        ),
        (
            "战斗直到全灭",
            config.fight_to_last_man,
            "mirror-last-man",
            MirrorOption::FightToLast,
        ),
        (
            "键盘寻路",
            config.mirror_keyboard_navigation,
            "mirror-keyboard",
            MirrorOption::KeyboardNavigation,
        ),
        (
            "简易键盘寻路",
            config.mirror_keyboard_simple_pathfinding,
            "mirror-simple-keyboard",
            MirrorOption::SimplePathfinding,
        ),
    ];
    let mut options_view = div().flex().flex_col().gap_1();
    for (label, value, id, field) in options {
        let toggle = task_option_switch(label, value, id, busy, cx, move |home| {
            home.toggle_mirror_option(field);
        });
        options_view = options_view.child(control_row(label, toggle));
    }
    let progress = app.home.mirror_progress.as_ref().map(|progress| {
        div()
            .flex()
            .items_center()
            .justify_between()
            .px_2()
            .py_1()
            .rounded_md()
            .bg(rgba((ACCENT << 8) | 0x22))
            .child(div().text_size(px(11.0)).text_color(rgb(ACCENT)).child(
                if progress.isInfinite {
                    format!("镜牢进度 {} / ∞", progress.current)
                } else {
                    format!("镜牢进度 {} / {}", progress.current, progress.total)
                },
            ))
            .child(badge(
                if progress.isHard { "困难" } else { "普通" },
                BadgeTone::Accent,
            ))
    });
    let advanced = div()
        .pt_2()
        .border_t_1()
        .border_color(rgb(BORDER))
        .child(options_view);
    let body = div()
        .flex()
        .flex_col()
        .gap_2()
        .children(progress)
        .child(control_row("运行次数", number))
        .child(control_row("无限模式", infinite))
        .child(control_row("困难镜牢", hard))
        .child(advanced);
    task_card_with_toggle(
        cx,
        busy,
        FixedTaskId::Mirror,
        "坐牢设置 (镜牢)",
        "CMP",
        enabled,
        expanded,
        executing,
        vec![
            preview_tag(
                "坐牢",
                if config.infinite_dungeons {
                    "∞".to_owned()
                } else {
                    format!("{}次", config.set_mirror_count)
                },
                config.infinite_dungeons,
            ),
            preview_tag(
                "难度",
                if config.hard_mirror {
                    "困难"
                } else {
                    "普通"
                },
                config.hard_mirror,
            ),
        ],
        Some(body),
    )
}

fn ahab_card(app: &mut AhabApp, cx: &mut Context<AhabApp>, busy: bool, executing: bool) -> Div {
    let enabled = app.home.tasks.enabledTasks.resonate_with_Ahab;
    task_card_with_toggle(
        cx,
        busy,
        FixedTaskId::ResonateWithAhab,
        "亚哈共鸣",
        "RAD",
        enabled,
        false,
        executing,
        vec![preview_tag(
            "语录",
            if enabled { "开启" } else { "关闭" },
            enabled,
        )],
        None,
    )
}

fn action_button<F>(
    label: impl Into<String>,
    id: impl Into<String>,
    variant: ButtonVariant,
    busy: bool,
    cx: &mut Context<AhabApp>,
    action: F,
) -> gpui::Stateful<Div>
where
    F: Fn(&mut HomeState) + 'static,
{
    let mut control = button(label, variant).id(id.into());
    if !busy {
        control = control.on_click(cx.listener(move |view, _, _, cx| {
            action(&mut view.home);
            cx.stop_propagation();
            cx.notify();
        }));
    }
    control
}

fn task_option_switch<F>(
    _label: &'static str,
    value: bool,
    id: &'static str,
    busy: bool,
    cx: &mut Context<AhabApp>,
    action: F,
) -> gpui::Stateful<Div>
where
    F: Fn(&mut HomeState) + 'static,
{
    let mut control = switch(value).id(id);
    if !busy {
        control = control.on_click(cx.listener(move |view, _, _, cx| {
            action(&mut view.home);
            cx.stop_propagation();
            cx.notify();
        }));
    }
    control
}

fn next_interval(value: f32, values: &[f32]) -> f32 {
    values
        .iter()
        .position(|candidate| (*candidate - value).abs() < f32::EPSILON)
        .map(|index| values[(index + 1) % values.len()])
        .unwrap_or(values[0])
}

fn preview_tag(label: &'static str, value: impl Into<String>, highlight: bool) -> Div {
    let tone = if highlight {
        BadgeTone::Accent
    } else {
        BadgeTone::Neutral
    };
    badge(format!("{label} {}", value.into()), tone)
}

fn task_card_with_toggle(
    cx: &mut Context<AhabApp>,
    busy: bool,
    task: FixedTaskId,
    title: &'static str,
    icon: &'static str,
    enabled: bool,
    expanded: bool,
    executing: bool,
    preview_tags: Vec<Div>,
    body: Option<Div>,
) -> Div {
    let mut toggle = switch(enabled).id(task_id(task));
    if !busy {
        toggle = toggle.on_click(cx.listener(move |view, _, _, cx| {
            view.home.toggle_task(task);
            cx.stop_propagation();
            cx.notify();
        }));
    }
    task_card(
        cx,
        task,
        title,
        icon,
        enabled,
        expanded,
        executing,
        preview_tags,
        body,
        Some(toggle),
    )
}

fn task_card(
    cx: &mut Context<AhabApp>,
    task: FixedTaskId,
    title: &'static str,
    icon: &'static str,
    enabled: bool,
    expanded: bool,
    executing: bool,
    preview_tags: Vec<Div>,
    body: Option<Div>,
    toggle: Option<gpui::Stateful<Div>>,
) -> Div {
    let has_options = body.is_some();
    let mut header = div()
        .id(format!("task-header-{}", task_id(task)))
        .flex()
        .items_center()
        .gap_2()
        .px_3()
        .py(px(6.0));
    if has_options {
        header = header
            .cursor_pointer()
            .hover(|style| style.bg(rgba((SURFACE_HOVER << 8) | 0x35)))
            .on_click(cx.listener(move |view, _, _, cx| {
                view.home.toggle_expanded(task);
                cx.notify();
            }));
    }

    if let Some(toggle) = toggle {
        header = header.child(div().flex_none().child(toggle));
    }
    header = header.child(
        div()
            .flex()
            .items_center()
            .gap_2()
            .flex_1()
            .min_w_0()
            .child(task_icon(icon, executing))
            .child(
                div()
                    .min_w_0()
                    .truncate()
                    .text_size(px(14.0))
                    .text_color(rgb(if executing {
                        ACCENT
                    } else if !enabled {
                        TEXT_MUTED
                    } else {
                        TEXT
                    }))
                    .child(title),
            ),
    );
    header = header.child(
        div()
            .flex()
            .items_center()
            .justify_end()
            .gap_1()
            .min_w_0()
            .flex_1()
            .overflow_hidden()
            .children(preview_tags),
    );
    if has_options {
        header = header.child(
            div()
                .w(px(16.0))
                .flex_none()
                .text_center()
                .text_size(px(16.0))
                .text_color(rgb(TEXT_MUTED))
                .child(if expanded { "⌃" } else { "⌄" }),
        );
    }

    let mut root = div()
        .relative()
        .min_w_0()
        .overflow_hidden()
        .rounded_lg()
        .border_1()
        .border_color(rgb(if executing { ACCENT } else { BORDER }))
        .bg(rgb(SURFACE));
    if !enabled {
        root = root.opacity(0.75).bg(rgba((SURFACE_HOVER << 8) | 0x30));
    }
    root = root.child(header);
    if expanded && let Some(body) = body {
        root = root.child(
            div()
                .bg(rgba((SURFACE_HOVER << 8) | 0x59))
                .px_3()
                .py(px(10.0))
                .child(body),
        );
    }
    if executing {
        root = root.child(running_sweep());
    }
    root
}

fn task_icon(label: &'static str, executing: bool) -> Div {
    div()
        .w(px(20.0))
        .h(px(20.0))
        .flex()
        .items_center()
        .justify_center()
        .rounded_md()
        .bg(rgb(if executing { ACCENT } else { SURFACE_HOVER }))
        .text_size(px(8.0))
        .text_color(rgb(if executing { BACKGROUND } else { TEXT_MUTED }))
        .child(label)
}

fn running_sweep() -> Div {
    div()
        .absolute()
        .top_0()
        .bottom_0()
        .left_0()
        .w(px(3.0))
        .overflow_hidden()
        .bg(rgb(ACCENT))
        .child(
            div()
                .absolute()
                .left_0()
                .right_0()
                .h(px(28.0))
                .bg(rgb(ACCENT))
                .with_animation(
                    "home-task-sweep",
                    Animation::new(Duration::from_millis(1200)).repeat(),
                    |element, progress| element.top(px(-28.0 + progress * 120.0)),
                ),
        )
}

fn number_button(
    label: String,
    task: FixedTaskId,
    cx: &mut Context<AhabApp>,
    busy: bool,
) -> gpui::Stateful<Div> {
    let mut control = button(label, ButtonVariant::Outline).id(format!("number-{}", task_id(task)));
    if !busy {
        control = control.on_click(cx.listener(move |view, _, _, cx| {
            view.home.cycle_number(task);
            cx.stop_propagation();
            cx.notify();
        }));
    }
    control
}

fn detail_switch(
    checked: bool,
    task: FixedTaskId,
    cx: &mut Context<AhabApp>,
    busy: bool,
    id: &'static str,
) -> gpui::Stateful<Div> {
    let mut control = switch(checked).id(id);
    if !busy {
        control = control.on_click(cx.listener(move |view, _, _, cx| {
            view.home.toggle_detail(task);
            cx.stop_propagation();
            cx.notify();
        }));
    }
    control
}

fn control_row(label: &'static str, control: impl IntoElement) -> Div {
    div()
        .flex()
        .items_center()
        .justify_between()
        .py_1()
        .child(
            div()
                .flex_1()
                .min_w_0()
                .text_size(px(12.))
                .text_color(rgb(TEXT_MUTED))
                .child(label),
        )
        .child(div().flex_none().child(control))
}

fn execution_toolbar(
    app: &mut AhabApp,
    cx: &mut Context<AhabApp>,
    busy: bool,
    state: ExecutionState,
) -> Div {
    let mut select_all = button("☑ 全选", ButtonVariant::Outline)
        .id("select-all")
        .h(px(32.0));
    let mut clear_all = button("↻ 清空", ButtonVariant::Outline)
        .id("clear-all")
        .h(px(32.0));
    if !busy {
        select_all = select_all.on_click(cx.listener(|view, _, _, cx| {
            view.home.set_all_tasks(true);
            cx.stop_propagation();
            cx.notify();
        }));
        clear_all = clear_all.on_click(cx.listener(|view, _, _, cx| {
            view.home.set_all_tasks(false);
            cx.stop_propagation();
            cx.notify();
        }));
    }

    let mut after_button = button(
        format!(
            "设置 · {}",
            after_completion_summary(&app.home.tasks.afterCompletion)
        ),
        ButtonVariant::Ghost,
    )
    .id("after-completion-open")
    .h(px(32.0))
    .flex_1()
    .min_w_0();
    if !busy {
        after_button = after_button.on_click(cx.listener(|view, _, _, cx| {
            view.home.set_after_completion_open(true);
            cx.stop_propagation();
            cx.notify();
        }));
    }

    let mut pause = button(
        if state == ExecutionState::Paused {
            "▶ 继续"
        } else {
            "Ⅱ 暂停"
        },
        ButtonVariant::Outline,
    )
    .id("pause-resume")
    .h(px(36.0));
    if busy {
        pause = pause.on_click(cx.listener(|view, _, _, cx| {
            view.home.pause_or_resume();
            cx.stop_propagation();
            cx.notify();
        }));
    }

    let (run_label, run_variant) = if busy {
        ("■ Stop!", ButtonVariant::Destructive)
    } else {
        ("▶ Link Start!  F10", ButtonVariant::Default)
    };
    let mut run = button(run_label, run_variant).id("start-stop").h(px(36.0));
    if busy {
        run = run.on_click(cx.listener(|view, _, _, cx| {
            view.home.stop();
            cx.stop_propagation();
            cx.notify();
        }));
    } else {
        run = run.on_click(cx.listener(|view, _, _, cx| {
            view.home.start();
            cx.stop_propagation();
            cx.notify();
        }));
    }

    let mut command_group = div().min_w_0().flex().flex_wrap().items_center().gap_2();
    if busy {
        command_group = command_group.child(pause);
    }
    command_group = command_group.child(run);

    div()
        .flex_none()
        .min_w_0()
        .flex()
        .flex_wrap()
        .items_center()
        .justify_between()
        .gap_3()
        .border_t_1()
        .border_color(rgb(BORDER))
        .bg(rgba((SURFACE << 8) | 0xf2))
        .p_3()
        .child(
            div()
                .min_w_0()
                .flex()
                .flex_1()
                .flex_wrap()
                .items_center()
                .gap_1()
                .child(select_all)
                .child(clear_all)
                .child(after_button),
        )
        .child(command_group)
}

fn after_completion_editor(app: &mut AhabApp, cx: &mut Context<AhabApp>, busy: bool) -> Div {
    if !app.home.after_completion_open {
        return div();
    }
    let config = app.home.tasks.afterCompletion.clone();
    let mut exits = div().flex().flex_wrap().gap_2();
    for action in [
        AfterExitAction::ExitGame,
        AfterExitAction::ExitEmulator,
        AfterExitAction::ExitAalc,
    ] {
        let selected = config.actions.contains(&action);
        let mut control = button(
            after_exit_label(action),
            if selected {
                ButtonVariant::Secondary
            } else {
                ButtonVariant::Outline
            },
        )
        .id(format!("after-exit-{action:?}"));
        if !busy {
            control = control.on_click(cx.listener(move |view, _, _, cx| {
                view.home.toggle_after_exit_action(action);
                cx.notify();
            }));
        }
        exits = exits.child(control);
    }

    let mut power = div().flex().flex_wrap().gap_2();
    for action in [
        AfterPowerAction::None,
        AfterPowerAction::Sleep,
        AfterPowerAction::Hibernate,
        AfterPowerAction::Lock,
        AfterPowerAction::Shutdown,
    ] {
        let selected = config.powerAction == action;
        let mut control = button(
            after_power_label(action),
            if selected {
                ButtonVariant::Secondary
            } else {
                ButtonVariant::Outline
            },
        )
        .id(format!("after-power-{action:?}"));
        if !busy {
            control = control.on_click(cx.listener(move |view, _, _, cx| {
                view.home.set_after_power_action(action);
                cx.notify();
            }));
        }
        power = power.child(control);
    }

    let mut keep = switch(config.keepAfterCompletion).id("after-keep-default");
    if !busy {
        keep = keep.on_click(cx.listener(move |view, _, _, cx| {
            let keep = !view.home.tasks.afterCompletion.keepAfterCompletion;
            view.home.set_keep_after_completion(keep);
            cx.notify();
        }));
    }
    let mut close = button("完成", ButtonVariant::Default).id("after-completion-close");
    close = close.on_click(cx.listener(|view, _, _, cx| {
        view.home.set_after_completion_open(false);
        cx.notify();
    }));

    div()
        .absolute()
        .top_0()
        .left_0()
        .right_0()
        .bottom_0()
        .flex()
        .items_center()
        .justify_center()
        .p_4()
        .bg(rgba(0x080c14d9))
        .child(
            card(
                div()
                    .flex()
                    .flex_col()
                    .gap_3()
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .child(
                                div()
                                    .text_size(px(15.))
                                    .text_color(rgb(TEXT))
                                    .child("结束后操作"),
                            )
                            .child(close),
                    )
                    .child(control_row("退出动作（可多选）", exits))
                    .child(control_row("最终电源动作", power))
                    .child(control_row("保存为默认配置", keep)),
            )
            .w(px(460.0))
            .max_w_full(),
        )
}

fn after_completion_summary(config: &crate::model::AfterCompletionConfig) -> String {
    let exits = if config.actions.is_empty() {
        "无".to_owned()
    } else {
        config
            .actions
            .iter()
            .map(|action| after_exit_label(*action))
            .collect::<Vec<_>>()
            .join("、")
    };
    format!(
        "结束后：{} / {}",
        exits,
        after_power_label(config.powerAction)
    )
}

fn after_exit_label(action: AfterExitAction) -> &'static str {
    match action {
        AfterExitAction::ExitGame => "退出游戏",
        AfterExitAction::ExitEmulator => "退出模拟器",
        AfterExitAction::ExitAalc => "退出 AALC",
    }
}

fn after_power_label(action: AfterPowerAction) -> &'static str {
    match action {
        AfterPowerAction::None => "无动作",
        AfterPowerAction::Sleep => "睡眠",
        AfterPowerAction::Hibernate => "休眠",
        AfterPowerAction::Lock => "锁屏",
        AfterPowerAction::Shutdown => "关机",
    }
}

fn splitter(app: &mut AhabApp, cx: &mut Context<AhabApp>) -> gpui::Stateful<Div> {
    let collapsed = app.home.right_panel_collapsed;
    let width = if collapsed {
        SPLITTER_COLLAPSED_WIDTH
    } else {
        SPLITTER_WIDTH
    };
    let mut handle = div()
        .id("home-panel-splitter")
        .w(px(width))
        .h_full()
        .flex_none()
        .flex()
        .items_center()
        .justify_center()
        .cursor(gpui::CursorStyle::ResizeColumn)
        .hover(|style| style.bg(rgba((ACCENT << 8) | 0x35)))
        .child(div().w(px(2.0)).h(px(32.0)).rounded_full().bg(rgb(BORDER)))
        .on_drag(SplitterDragGhost, |_, _, _, cx| {
            cx.new(|_| SplitterDragGhost)
        });
    handle = handle.on_drag_move(cx.listener(
        |view, event: &gpui::DragMoveEvent<SplitterDragGhost>, window, cx| {
            let viewport_width = window.viewport_size().width.as_f32();
            let requested_width = viewport_width - event.event.position.x.as_f32();
            if requested_width < SPLITTER_COLLAPSE_THRESHOLD {
                view.set_right_panel_collapsed(true);
            } else {
                let available_max = (viewport_width - MIN_LEFT_PANEL_WIDTH - SPLITTER_WIDTH)
                    .max(RIGHT_PANEL_MIN_WIDTH)
                    .min(RIGHT_PANEL_MAX_WIDTH);
                view.set_right_panel_width(
                    requested_width
                        .clamp(RIGHT_PANEL_MIN_WIDTH, available_max)
                        .round() as u32,
                );
                view.set_right_panel_collapsed(false);
            }
            cx.notify();
        },
    ));
    handle
}

fn right_panel(app: &mut AhabApp, cx: &mut Context<AhabApp>) -> Div {
    let width = bounded_right_panel_width(app.home.right_panel_width);
    let connection_status = match app.home.device_status {
        ConnectionStatus::Connected => ("已连接", BadgeTone::Success),
        ConnectionStatus::Connecting => ("连接中", BadgeTone::Accent),
        ConnectionStatus::Disconnected => ("未连接", BadgeTone::Neutral),
    };

    let selected_id = app.home.selected_device.clone();
    let selected_device = selected_id
        .as_deref()
        .and_then(|id| app.home.devices.iter().find(|device| device.id == id));
    let selected_name = selected_device
        .map(|device| device.name.clone())
        .unwrap_or_else(|| "选择游戏窗口".to_owned());
    let next_device = next_device_id(&app.home.devices, selected_id.as_deref());
    let mut device_select = button(selected_name, ButtonVariant::Outline)
        .id("device-select")
        .h(px(30.0))
        .flex_1()
        .min_w_0();
    if let Some(next_device) = next_device {
        device_select = device_select.on_click(cx.listener(move |view, _, _, cx| {
            view.home.select_device(next_device.clone());
            cx.stop_propagation();
            cx.notify();
        }));
    }

    let mut refresh = button("刷新", ButtonVariant::Icon)
        .id("device-refresh")
        .h(px(30.0));
    if app.home.device_status != ConnectionStatus::Connecting {
        refresh = refresh.on_click(cx.listener(|view, _, _, cx| {
            let response = view
                .home
                .client
                .call(crate::ipc::contract::method::DEVICE_LIST, None);
            if let Some(value) = response.result
                && let Ok(devices) = serde_json::from_value(value)
            {
                view.home.devices = devices;
            }
            cx.stop_propagation();
            cx.notify();
        }));
    }

    let mut disconnect = button("×", ButtonVariant::Icon)
        .id("disconnect-device")
        .h(px(30.0));
    if app.home.device_status == ConnectionStatus::Connected {
        disconnect = disconnect.on_click(cx.listener(|view, _, _, cx| {
            view.home.disconnect_device();
            cx.stop_propagation();
            cx.notify();
        }));
    }

    let connection_header = div()
        .h(px(36.0))
        .flex_none()
        .flex()
        .items_center()
        .justify_between()
        .border_b_1()
        .border_color(rgb(BORDER))
        .px_3()
        .child(panel_heading("MON", "设备连接"))
        .child(badge(connection_status.0, connection_status.1));
    let connection_body = div()
        .flex()
        .items_center()
        .gap_2()
        .p_3()
        .child(device_select)
        .child(refresh)
        .children((app.home.device_status == ConnectionStatus::Connected).then_some(disconnect));

    let screenshot_detail = app
        .home
        .latest_screenshot
        .as_ref()
        .map(|frame| format!("收到最新画面 · {}×{}", frame.width, frame.height))
        .unwrap_or_else(|| "等待游戏窗口画面接入".to_owned());
    let screenshot_header = div()
        .h(px(32.0))
        .flex_none()
        .flex()
        .items_center()
        .px(px(10.0))
        .child(panel_heading("SCR", "实时画面"));
    let screenshot_body = div()
        .w_full()
        .aspect_ratio(16.0 / 9.0)
        .flex()
        .flex_col()
        .items_center()
        .justify_center()
        .gap_1()
        .rounded_md()
        .border_1()
        .border_color(rgb(SURFACE_HOVER))
        .bg(rgb(BACKGROUND))
        .text_size(px(11.0))
        .text_color(rgb(TEXT_MUTED))
        .child("LIVE")
        .child(screenshot_detail)
        .child(
            div()
                .text_size(px(10.0))
                .text_color(rgb(TEXT_MUTED))
                .child("16:9 · 1280×720"),
        );
    let screenshot_card = panel_card(
        div()
            .flex()
            .flex_col()
            .child(screenshot_header)
            .child(div().p(px(10.0)).child(screenshot_body)),
    )
    .flex_none();

    let visible_logs: Vec<String> = app
        .home
        .logs
        .iter()
        .rev()
        .take(300)
        .cloned()
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect();
    let log_rows: Vec<_> = visible_logs
        .into_iter()
        .map(|line| {
            div()
                .w_full()
                .flex()
                .items_start()
                .gap_2()
                .py(px(2.0))
                .font_family("monospace")
                .text_size(px(11.0))
                .text_color(rgb(TEXT_MUTED))
                .child(
                    div()
                        .mt(px(5.0))
                        .w(px(5.0))
                        .h(px(5.0))
                        .flex_none()
                        .rounded_full()
                        .bg(rgb(TEXT_MUTED)),
                )
                .child(line)
        })
        .collect();
    let mut clear_logs = button("清空", ButtonVariant::Ghost)
        .id("clear-logs")
        .h(px(24.0));
    clear_logs = clear_logs.on_click(cx.listener(|view, _, _, cx| {
        view.home.clear_logs();
        cx.stop_propagation();
        cx.notify();
    }));
    let logs_header = div()
        .h(px(32.0))
        .flex_none()
        .flex()
        .items_center()
        .justify_between()
        .px(px(10.0))
        .child(
            div()
                .flex()
                .items_center()
                .gap_2()
                .child(panel_heading("LOG", "运行日志"))
                .child(badge(
                    visible_logs_count(app).to_string(),
                    BadgeTone::Neutral,
                )),
        )
        .child(clear_logs);
    let logs_card = panel_card(
        div()
            .flex()
            .flex_col()
            .min_h_0()
            .h_full()
            .child(logs_header)
            .child(
                scroll_area_with_id("home-log-scroll", div().children(log_rows))
                    .flex_1()
                    .min_h_0()
                    .px_3()
                    .py_2(),
            ),
    )
    .flex_1()
    .min_h_0();

    div()
        .w(px(width))
        .min_w(px(240.0))
        .h_full()
        .min_h_0()
        .flex_shrink_1()
        .flex()
        .flex_col()
        .gap_2()
        .overflow_x_hidden()
        .p(px(10.0))
        .child(
            panel_card(
                div()
                    .flex()
                    .flex_col()
                    .child(connection_header)
                    .child(connection_body),
            )
            .flex_none(),
        )
        .child(screenshot_card)
        .child(logs_card)
}

fn panel_card(child: impl IntoElement) -> Div {
    div()
        .min_w_0()
        .overflow_hidden()
        .rounded_lg()
        .border_1()
        .border_color(rgb(BORDER))
        .bg(rgb(SURFACE))
        .child(child)
}

fn panel_heading(icon: &'static str, title: &'static str) -> Div {
    div()
        .flex()
        .items_center()
        .gap_2()
        .text_size(px(12.0))
        .text_color(rgb(TEXT_MUTED))
        .child(
            div()
                .w(px(20.0))
                .text_size(px(8.0))
                .text_color(rgb(ACCENT))
                .child(icon),
        )
        .child(title)
}

fn next_device_id(
    devices: &[crate::model::DeviceInfo],
    selected_id: Option<&str>,
) -> Option<String> {
    if devices.is_empty() {
        return None;
    }
    let next_index = selected_id
        .and_then(|selected| devices.iter().position(|device| device.id == selected))
        .map(|index| (index + 1) % devices.len())
        .unwrap_or(0);
    Some(devices[next_index].id.clone())
}

fn visible_logs_count(app: &AhabApp) -> usize {
    app.home.logs.len().min(300)
}

fn first_executable_task(home: &HomeState) -> Option<FixedTaskId> {
    let enabled = &home.tasks.enabledTasks;
    if enabled.daily_task {
        Some(FixedTaskId::DailyTask)
    } else if enabled.get_reward {
        Some(FixedTaskId::GetReward)
    } else if enabled.buy_enkephalin {
        Some(FixedTaskId::BuyEnkephalin)
    } else if enabled.mirror {
        Some(FixedTaskId::Mirror)
    } else {
        None
    }
}

fn bounded_right_panel_width(width: f32) -> f32 {
    if !width.is_finite() {
        RIGHT_PANEL_DEFAULT_WIDTH
    } else {
        width.clamp(RIGHT_PANEL_MIN_WIDTH, RIGHT_PANEL_MAX_WIDTH)
    }
}

fn reward_mode_label(mode: u8) -> &'static str {
    match mode {
        0 => "全部",
        1 => "狂气/通行证",
        2 => "邮件",
        _ => "全部",
    }
}

#[cfg(test)]
mod tests {
    use super::{
        RIGHT_PANEL_DEFAULT_WIDTH, RIGHT_PANEL_MAX_WIDTH, RIGHT_PANEL_MIN_WIDTH,
        SPLITTER_COLLAPSED_WIDTH, SPLITTER_WIDTH, bounded_right_panel_width, reward_mode_label,
    };

    fn available_home_left_width(
        window_width: u32,
        right_panel_width: u32,
        splitter_width: u32,
    ) -> u32 {
        window_width.saturating_sub(right_panel_width + splitter_width)
    }

    #[test]
    fn minimum_window_keeps_home_columns_reachable() {
        assert_eq!(
            available_home_left_width(800, 280, SPLITTER_WIDTH as u32),
            516
        );
        assert!(available_home_left_width(800, 280, SPLITTER_WIDTH as u32) > 0);
        assert_eq!(
            available_home_left_width(800, 0, SPLITTER_COLLAPSED_WIDTH as u32),
            784
        );
    }

    #[test]
    fn right_panel_width_is_bounded_to_the_visual_contract() {
        assert_eq!(
            bounded_right_panel_width(f32::NAN),
            RIGHT_PANEL_DEFAULT_WIDTH
        );
        assert_eq!(bounded_right_panel_width(100.0), RIGHT_PANEL_MIN_WIDTH);
        assert_eq!(bounded_right_panel_width(280.0), RIGHT_PANEL_MIN_WIDTH);
        assert_eq!(bounded_right_panel_width(900.0), RIGHT_PANEL_MAX_WIDTH);
    }

    #[test]
    fn reward_modes_use_ui_names() {
        assert_eq!(reward_mode_label(0), "全部");
        assert_eq!(reward_mode_label(1), "狂气/通行证");
        assert_eq!(reward_mode_label(2), "邮件");
    }
}

fn task_title(task: FixedTaskId) -> &'static str {
    match task {
        FixedTaskId::SetWindows => "窗口设置",
        FixedTaskId::DailyTask => "日常任务",
        FixedTaskId::GetReward => "领取奖励",
        FixedTaskId::BuyEnkephalin => "狂气换体",
        FixedTaskId::Mirror => "坐牢设置 (镜牢)",
        FixedTaskId::ResonateWithAhab => "亚哈共鸣",
    }
}

fn task_id(task: FixedTaskId) -> &'static str {
    match task {
        FixedTaskId::SetWindows => "set-windows",
        FixedTaskId::DailyTask => "daily-task",
        FixedTaskId::GetReward => "get-reward",
        FixedTaskId::BuyEnkephalin => "buy-enkephalin",
        FixedTaskId::Mirror => "mirror",
        FixedTaskId::ResonateWithAhab => "resonate-ahab",
    }
}
