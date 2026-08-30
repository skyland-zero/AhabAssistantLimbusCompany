use super::*;

pub(super) struct TaskCardSpec {
    pub(super) task: FixedTaskId,
    pub(super) title: String,
    pub(super) icon: &'static str,
    pub(super) enabled: bool,
    pub(super) expanded: bool,
    pub(super) executing: bool,
    pub(super) preview_tags: Vec<Div>,
    pub(super) body: Option<Div>,
}

pub(super) fn preview_tag(
    label: impl Into<String>,
    value: impl Into<String>,
    highlight: bool,
) -> Div {
    let label = label.into();
    let tone = if highlight {
        BadgeTone::Accent
    } else {
        BadgeTone::Neutral
    };
    badge(format!("{label} {}", value.into()), tone)
}

pub(super) fn task_card_with_toggle(
    cx: &mut Context<AhabApp>,
    busy: bool,
    spec: TaskCardSpec,
) -> Div {
    let task = spec.task;
    let enabled = spec.enabled;
    let mut toggle = switch_accent(enabled).id(task_id(task));
    if !busy {
        toggle = toggle.on_click(cx.listener(move |view, _, _, cx| {
            view.home.toggle_task(task);
            cx.stop_propagation();
            cx.notify();
        }));
        toggle = toggle.on_key_down(cx.listener(move |view, event: &KeyDownEvent, window, cx| {
            if is_activation_key(event) {
                window.prevent_default();
                view.home.toggle_task(task);
                cx.notify();
            }
        }));
    } else {
        toggle = toggle.opacity(0.45).cursor_not_allowed();
    }
    task_card(cx, spec, Some(toggle))
}

pub(super) fn task_card(
    cx: &mut Context<AhabApp>,
    spec: TaskCardSpec,
    toggle: Option<gpui::Stateful<Div>>,
) -> Div {
    let TaskCardSpec {
        task,
        title,
        icon,
        enabled,
        expanded,
        executing,
        preview_tags,
        body,
    } = spec;
    let has_options = body.is_some();
    let mut header = div()
        .id(format!("task-header-{}", task_id(task)))
        .flex()
        .items_center()
        .gap_2()
        .px_3()
        .h(px(36.0))
        .py_0();
    if has_options {
        header = header
            .cursor_pointer()
            .tab_index(0)
            .hover(|style| style.bg(rgba((SURFACE_HOVER << 8) | 0x35)))
            .on_click(cx.listener(move |view, _, _, cx| {
                view.home.toggle_expanded(task);
                cx.notify();
            }))
            .on_key_down(cx.listener(move |view, event: &KeyDownEvent, window, cx| {
                if is_activation_key(event) {
                    window.prevent_default();
                    view.home.toggle_expanded(task);
                    cx.notify();
                }
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
    if !expanded {
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
    }
    if has_options {
        header = header.child(
            div()
                .w(px(16.0))
                .flex_none()
                .text_center()
                .text_size(px(16.0))
                .text_color(rgb(TEXT_MUTED))
                .child(action_icon(
                    if expanded {
                        ICON_CHEVRON_UP
                    } else {
                        ICON_CHEVRON_DOWN
                    },
                    14.,
                    TEXT_MUTED,
                )),
        );
    }

    let mut root = div()
        .relative()
        .min_w_0()
        .overflow_hidden()
        .rounded_lg()
        .border_1()
        .border_color(if executing { rgb(ACCENT) } else { rgba(0) })
        .bg(rgb(SURFACE));
    root = root.child(header);
    if expanded && let Some(body) = body {
        let body = div()
            .relative()
            .bg(rgba((SURFACE_HOVER << 8) | 0x59))
            .px_3()
            .py(px(10.0))
            .child(body)
            .with_animation(
                format!("task-details-{}", task_id(task)),
                Animation::new(Duration::from_millis(150)).with_easing(gpui::ease_out_quint()),
                |body, progress| body.opacity(progress).top(px(-4.0 * (1.0 - progress))),
            );
        root = root.child(body);
    }
    if executing {
        root = root.child(running_sweep(task));
    }
    root
}

pub(super) fn task_icon(label: &'static str, executing: bool) -> Div {
    let color = if executing { BACKGROUND } else { TEXT_MUTED };
    if label == "SET" {
        return div()
            .flex()
            .items_center()
            .gap_2()
            .child(
                div()
                    .w(px(20.0))
                    .h(px(20.0))
                    .flex()
                    .items_center()
                    .justify_center()
                    .rounded_md()
                    .bg(rgb(if executing { ACCENT } else { SURFACE_HOVER }))
                    .text_color(rgb(color))
                    .font_family("monospace")
                    .text_size(px(9.0))
                    .child(label),
            )
            .child(action_icon(ICON_SLIDERS, 16., color));
    }

    div()
        .w(px(16.0))
        .h(px(16.0))
        .flex()
        .items_center()
        .justify_center()
        .text_color(rgb(color))
        .child(action_icon(task_icon_data(label), 16., color))
}

pub(super) fn running_sweep(task: FixedTaskId) -> Div {
    let palette = current_render_palette();
    let is_dark = matches!(palette.scheme, crate::components::style::ColorScheme::Dark);
    let track_color = rgba((palette.brand.rgb_hex() << 8) | if is_dark { 0x38 } else { 0x28 });
    let core_color = palette_rgb(palette.brand);

    div()
        .absolute()
        .top(px(6.0))
        .bottom(px(6.0))
        .left(px(4.0))
        .w(px(3.0))
        .rounded_full()
        .overflow_hidden()
        .bg(track_color)
        .child(
            div()
                .absolute()
                .left_0()
                .right_0()
                .h(relative(0.45))
                .rounded_full()
                .bg(core_color)
                .with_animation(
                    format!("home-task-sweep-{}", task_id(task)),
                    Animation::new(Duration::from_millis(1300)).repeat(),
                    |element, progress| element.top(relative(progress * 1.45 - 0.45)),
                ),
        )
}

pub(super) fn detail_switch(
    checked: bool,
    task: FixedTaskId,
    cx: &mut Context<AhabApp>,
    busy: bool,
    id: &'static str,
) -> gpui::Stateful<Div> {
    task_option_switch("", checked, id, busy, cx, move |home| {
        home.toggle_detail(task);
    })
}

pub(super) fn control_row(label: impl Into<String>, control: impl IntoElement) -> Div {
    div()
        .flex()
        .items_center()
        .justify_between()
        .py_1()
        .child(
            div()
                .flex_1()
                .min_w_0()
                .text_size(px(13.))
                .text_color(rgb(TEXT))
                .child(label.into()),
        )
        .child(div().flex_none().child(control))
}

pub(super) fn adaptive_settings_grid(
    language: Language,
    children: impl IntoIterator<Item = Div>,
) -> Div {
    let min_width = if matches!(language, Language::ZhCn) {
        160.
    } else {
        200.
    };
    settings_grid(children, min_width)
}

#[allow(dead_code)]
pub(super) fn task_title(task: FixedTaskId, language: Language) -> &'static str {
    match task {
        FixedTaskId::SetWindows => text("窗口设置", "Window Settings").get(language),
        FixedTaskId::DailyTask => text("日常任务", "Daily Tasks").get(language),
        FixedTaskId::GetReward => text("领取奖励", "Claim Rewards").get(language),
        FixedTaskId::BuyEnkephalin => text("狂气换体", "Refill Enkephalin").get(language),
        FixedTaskId::Mirror => text("坐牢设置 (镜牢)", "Mirror Dungeon").get(language),
        FixedTaskId::ResonateWithAhab => text("亚哈共鸣", "Ahab Resonance").get(language),
    }
}

pub(super) fn task_id(task: FixedTaskId) -> &'static str {
    match task {
        FixedTaskId::SetWindows => "set-windows",
        FixedTaskId::DailyTask => "daily-task",
        FixedTaskId::GetReward => "get-reward",
        FixedTaskId::BuyEnkephalin => "buy-enkephalin",
        FixedTaskId::Mirror => "mirror",
        FixedTaskId::ResonateWithAhab => "resonate-ahab",
    }
}
