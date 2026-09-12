use super::*;

pub(crate) fn current_run_card(snapshot: &StatsSnapshot) -> Div {
    let language = snapshot.language;
    let current = &snapshot.stats.currentRun;
    let (targets, infinite) = if current.runId.is_some() {
        (current.targets.clone(), current.isMirrorInfinite)
    } else {
        (
            StatCounts {
                exp: if snapshot.tasks.enabledTasks.daily_task {
                    u32::from(snapshot.tasks.daily_task.set_EXP_count)
                } else {
                    0
                },
                thread: if snapshot.tasks.enabledTasks.daily_task {
                    u32::from(snapshot.tasks.daily_task.set_thread_count)
                } else {
                    0
                },
                mirror: if snapshot.tasks.enabledTasks.mirror
                    && !snapshot.tasks.mirror.infinite_dungeons
                {
                    u32::from(snapshot.tasks.mirror.set_mirror_count)
                } else {
                    0
                },
            },
            snapshot.tasks.enabledTasks.mirror && snapshot.tasks.mirror.infinite_dungeons,
        )
    };
    let completed = if current.runId.is_some() {
        current.completed.clone()
    } else {
        StatCounts::default()
    };
    // 显示状态以 execution.status 为准（带 stateRevision 的权威快照），
    // 状态与当前任务必须来自同一份快照，避免混读两份 payload 拼出矛盾显示。
    let (state, current_task) = display_run_state(snapshot);
    // 用时只计算一次：秒边界多次调用 SystemTime::now() 会让同一卡片出现两个差 1 秒的用时。
    let elapsed_str = format_duration(live_elapsed_secs(current, snapshot.stats.updatedAt, state));
    let state_text = match state {
        ExecutionState::Starting => text("启动中", "Starting").get(language),
        ExecutionState::Running => text("运行中", "Running").get(language),
        ExecutionState::Paused => text("已暂停", "Paused").get(language),
        ExecutionState::Stopping => text("停止中", "Stopping").get(language),
        ExecutionState::Restoring => text("恢复设备中", "Restoring device").get(language),
        ExecutionState::Idle => text("待机", "Idle").get(language),
    };
    let state_tone = match state {
        ExecutionState::Running => BadgeTone::Success,
        ExecutionState::Paused => BadgeTone::Warning,
        ExecutionState::Starting | ExecutionState::Stopping | ExecutionState::Restoring => {
            BadgeTone::Warning
        }
        ExecutionState::Idle => BadgeTone::Neutral,
    };
    let task_text = current_task
        .map(|task| task_title(task, language).to_owned())
        .unwrap_or_else(|| {
            text("等待开始", "Waiting to start")
                .get(language)
                .to_owned()
        });

    let header = div()
        .h(px(24.0))
        .flex_none()
        .flex()
        .items_center()
        .justify_between()
        .gap_2()
        .child(
            div()
                .flex()
                .items_center()
                .gap_2()
                .child(action_icon(ICON_PLAY, 14., ACCENT))
                .child(
                    div()
                        .text_size(px(12.0))
                        .text_color(rgb(TEXT_MUTED))
                        .child(text("本次运行", "Current Run").get(language)),
                ),
        )
        .child(badge(state_text, state_tone));

    let task_line = div()
        .h(px(20.0))
        .flex_none()
        .flex()
        .items_center()
        .gap_2()
        .text_size(px(11.0))
        .text_color(rgb(TEXT_MUTED))
        .child(text("当前任务", "Task").get(language))
        .child(
            div()
                .min_w_0()
                .truncate()
                .text_color(rgb(TEXT))
                .child(task_text),
        );

    let info_row = {
        let started_hm = current.startedAt.and_then(|ms| {
            // The payload carries epoch milliseconds (UTC); format it in the
            // user's actual local timezone instead of a hardcoded offset.
            chrono::DateTime::from_timestamp_millis(ms).map(|utc| {
                utc.with_timezone(&chrono::Local)
                    .format("%H:%M")
                    .to_string()
            })
        });
        let is_running = current.runId.is_some() && state == ExecutionState::Running;
        if is_running && current.startedAt.is_some() {
            let mut row = div()
                .h(px(16.0))
                .flex_none()
                .flex()
                .items_center()
                .gap_2()
                .text_size(px(10.0))
                .text_color(rgb(TEXT_MUTED));
            row = row.child(match language {
                crate::model::Language::ZhCn => format!(
                    "已运行 {} · 开始 {}",
                    elapsed_str,
                    started_hm.unwrap_or_else(|| "--:--".into())
                ),
                crate::model::Language::EnUs => format!(
                    "Elapsed {} · Started {}",
                    elapsed_str,
                    started_hm.unwrap_or_else(|| "--:--".into())
                ),
            });
            if current_task_is_mirror(current_task) {
                // 楼层只显示一次：总层数已知时带上（第3层/共5层），与任务区共用同一格式化入口。
                if let Some(floor_label) =
                    mirror_floor_label(snapshot.mirror_floor.as_ref(), language)
                {
                    row = row.child(
                        div()
                            .ml_auto()
                            .px(px(6.0))
                            .py(px(1.0))
                            .rounded_sm()
                            .bg(rgba((ACCENT << 8) | 0x18))
                            .text_size(px(9.0))
                            .text_color(rgb(ACCENT))
                            .child(floor_label),
                    );
                }
            }
            row
        } else {
            div().h(px(16.0)).flex_none()
        }
    };

    let mirror_block: gpui::AnyElement =
        if current_task_is_mirror(current_task) && current.runId.is_some() {
            let is_running = state == ExecutionState::Running;
            let display_completed = if is_running {
                if infinite {
                    completed.mirror + 1
                } else if targets.mirror > 0 {
                    (completed.mirror + 1).min(targets.mirror)
                } else {
                    completed.mirror + 1
                }
            } else {
                completed.mirror
            };
            div()
                .flex_none()
                .flex()
                .flex_col()
                .gap(px(4.0))
                .child(
                    div()
                        .h(px(3.0))
                        .w_full()
                        .rounded_full()
                        .bg(rgba((TEXT_MUTED << 8) | 0x24))
                        .child(
                            div()
                                .h_full()
                                .w(relative(mirror_progress_ratio(
                                    display_completed,
                                    targets.mirror,
                                    infinite,
                                )))
                                .rounded_full()
                                .bg(rgb(ACCENT)),
                        ),
                )
                .child(
                    div()
                        .flex()
                        .items_center()
                        .text_size(px(9.5))
                        .text_color(rgb(TEXT_MUTED))
                        // 楼层后缀与右侧用时已删除：楼层只在上方 chip 显示，用时只在信息行显示。
                        .child(match language {
                            crate::model::Language::ZhCn => format!(
                                "镜牢进度 {}/{}{}",
                                display_completed,
                                if infinite {
                                    "∞".to_string()
                                } else {
                                    targets.mirror.to_string()
                                },
                                if is_running { " · 进行中" } else { "" }
                            ),
                            crate::model::Language::EnUs => format!(
                                "Mirror progress {}/{}{}",
                                display_completed,
                                if infinite {
                                    "∞".to_string()
                                } else {
                                    targets.mirror.to_string()
                                },
                                if is_running { " · running" } else { "" }
                            ),
                        }),
                )
                .into_any_element()
        } else {
            // 不在镜牢：用 flex 占位把 metrics 压底，不显 “镜牢未运行” 文案
            div().flex_1().min_h(px(8.0)).into_any_element()
        };

    let metrics = div()
        .flex_none()
        .flex()
        .items_stretch()
        .gap_2()
        .child(run_metric(
            text("经验本", "EXP").get(language),
            completed.exp,
            targets.exp,
            false,
        ))
        .child(run_metric(
            text("纽本", "Thread").get(language),
            completed.thread,
            targets.thread,
            false,
        ))
        .child(run_metric(
            text("镜牢", "Mirror").get(language),
            completed.mirror,
            targets.mirror,
            infinite,
        ));

    card(
        div()
            .h_full()
            .flex()
            .flex_col()
            .gap(px(4.0))
            .child(header)
            .child(task_line)
            .child(info_row)
            .child(mirror_block)
            .child(metrics),
    )
    .p(px(10.0))
    .h(px(STATS_CARD_HEIGHT))
    .min_w_0()
    .overflow_hidden()
}

pub(crate) fn current_task_is_mirror(task: Option<FixedTaskId>) -> bool {
    matches!(task, Some(FixedTaskId::Mirror))
}

pub(crate) fn display_run_state(snapshot: &StatsSnapshot) -> (ExecutionState, Option<FixedTaskId>) {
    let current = &snapshot.stats.currentRun;
    let execution = &snapshot.execution;
    if execution.runId.is_some() {
        // execution.status（带 stateRevision）是权威快照：点了停止后这里会立刻
        // 变成 Stopping，而 stats 事件可能还滞后为 Running。缺失字段回落到 currentRun。
        (
            execution.state,
            execution.currentTaskId.or(current.currentTaskId),
        )
    } else if current.runId.is_some() {
        // 过渡期：execution 快照还没跟上新 run，沿用 currentRun 避免闪烁。
        (current.state, current.currentTaskId)
    } else {
        (ExecutionState::Idle, None)
    }
}

/// 楼层显示的唯一格式化入口：本次运行 chip 与任务配置区共用，避免中英双语分支散落重复。
/// floor 为 0 或缺失时返回 None，调用方直接不渲染。
pub(crate) fn mirror_floor_label(
    floor: Option<&MirrorFloorPayload>,
    language: Language,
) -> Option<String> {
    let floor = floor?;
    if floor.floor == 0 {
        return None;
    }
    Some(if floor.floorTotal > 0 {
        match language {
            crate::model::Language::ZhCn => {
                format!("第{}层/共{}层", floor.floor, floor.floorTotal)
            }
            _ => format!("Floor {}/{}", floor.floor, floor.floorTotal),
        }
    } else {
        match language {
            crate::model::Language::ZhCn => format!("第{}层", floor.floor),
            _ => format!("Floor {}", floor.floor),
        }
    })
}

pub(crate) fn mirror_progress_ratio(completed: u32, target: u32, infinite: bool) -> f32 {
    if infinite || target == 0 {
        if completed == 0 {
            0.0
        } else {
            (completed as f32 * 0.15).clamp(0.0, 1.0)
        }
    } else {
        (completed as f32 / target as f32).clamp(0.0, 1.0)
    }
}

pub(crate) fn run_metric(label: &'static str, completed: u32, target: u32, infinite: bool) -> Div {
    // 三格只保留数字：镜牢进度已有主进度条，迷你进度条属于重复指标。
    let value = if infinite {
        format!("{completed} / ∞")
    } else {
        format!("{completed} / {target}")
    };
    div()
        .flex_1()
        .min_w_0()
        .flex()
        .flex_col()
        .justify_between()
        .gap_1()
        .px_2()
        .py_1p5()
        .rounded_md()
        .bg(rgba((SURFACE_HOVER << 8) | 0x45))
        .child(
            div()
                .text_size(px(10.0))
                .text_color(rgb(TEXT_MUTED))
                .truncate()
                .child(label),
        )
        .child(
            div()
                .text_size(px(16.0))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(rgb(TEXT))
                .child(value),
        )
}
