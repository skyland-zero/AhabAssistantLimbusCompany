use super::*;

#[allow(dead_code)]
pub(super) fn task_header(
    state: ExecutionState,
    current_task: Option<FixedTaskId>,
    language: crate::model::Language,
) -> Div {
    let (label, tone) = match state {
        ExecutionState::Starting => (
            text("正在启动", "Starting").get(language).to_owned(),
            BadgeTone::Warning,
        ),
        ExecutionState::Running => (
            current_task
                .map(|task| {
                    format!(
                        "{}: {}",
                        i18n::text(language, I18nKey::HomeRunning),
                        task_title(task, language)
                    )
                })
                .unwrap_or_else(|| i18n::text(language, I18nKey::HomeRunning).to_owned()),
            BadgeTone::Accent,
        ),
        ExecutionState::Paused => (
            i18n::text(language, I18nKey::HomePaused).to_owned(),
            BadgeTone::Accent,
        ),
        ExecutionState::Stopping => (
            text("正在停止", "Stopping").get(language).to_owned(),
            BadgeTone::Warning,
        ),
        ExecutionState::Restoring => (
            text("正在恢复设备", "Restoring device")
                .get(language)
                .to_owned(),
            BadgeTone::Warning,
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
        .gap(px(10.0))
        .px(px(14.0))
        .h(px(36.0))
        .py_0()
        .child(
            div()
                .text_size(px(14.0))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(rgb(TEXT))
                .child(i18n::text(language, I18nKey::HomeTitle)),
        )
        .child(badge(label, tone))
}

pub(super) fn options_tabs(
    task: FixedTaskId,
    selected: TaskOptionsTab,
    language: Language,
    cx: &mut Context<AhabApp>,
) -> Div {
    let palette = current_render_palette();
    let mut tabs = div()
        .flex()
        .items_center()
        .gap_1()
        .p(px(2.))
        .rounded_md()
        .bg(palette_rgb(palette.muted));

    for (tab, label) in [
        (TaskOptionsTab::General, text("常规设置", "General")),
        (TaskOptionsTab::Advanced, text("高级设置", "Advanced")),
    ] {
        let active = selected == tab;
        let hover = palette_rgb(palette.accent_surface);
        let foreground = palette_rgb(palette.foreground);
        let mut control = tab_surface_with_palette(active, &palette)
            .id(format!("home-options-{task:?}-{tab:?}"))
            .tab_index(0)
            .cursor_pointer()
            .hover(move |style| style.bg(hover).text_color(foreground))
            .child(label.get(language));

        control = control.on_click(cx.listener(move |view, _, _, cx| {
            view.home.set_options_tab(task, tab);
            cx.stop_propagation();
            cx.notify();
        }));
        control =
            control.on_key_down(cx.listener(move |view, event: &KeyDownEvent, window, cx| {
                if is_activation_key(event) {
                    window.prevent_default();
                    view.home.set_options_tab(task, tab);
                    cx.notify();
                }
            }));
        tabs = tabs.child(control);
    }
    tabs
}

pub(super) fn daily_counter(
    current: u8,
    min: u8,
    max: u8,
    field: DailyCounter,
    id: &'static str,
    busy: bool,
    cx: &mut Context<AhabApp>,
) -> Div {
    task_counter(
        current,
        min,
        max,
        id,
        busy,
        cx,
        Rc::new(move |home, delta| home.adjust_daily_counter(field, delta)),
    )
}

pub(super) type HomeCounterAction = Rc<dyn Fn(&mut HomeState, i8)>;
pub(super) type HomeSelectChange = Rc<dyn Fn(&mut HomeState, String)>;

pub(super) struct HomeSelectConfig {
    pub(super) select: HomeSelect,
    pub(super) current: String,
    pub(super) options: Vec<(String, String)>,
    pub(super) id: String,
    pub(super) width: f32,
    pub(super) disabled: bool,
    pub(super) on_change: HomeSelectChange,
}

pub(super) fn task_counter(
    current: u8,
    min: u8,
    max: u8,
    id: &'static str,
    busy: bool,
    cx: &mut Context<AhabApp>,
    action: HomeCounterAction,
) -> Div {
    let mut decrement = button("", ButtonVariant::Outline)
        .id(format!("{id}-decrement"))
        .w(px(26.))
        .h(px(26.))
        .p_0()
        .child(action_icon(ICON_MINUS, 13., TEXT));
    if !busy && current > min {
        let click_action = action.clone();
        let key_action = action.clone();
        decrement = decrement
            .on_click(cx.listener(move |view, _, _, cx| {
                click_action(&mut view.home, -1);
                cx.stop_propagation();
                cx.notify();
            }))
            .on_key_down(cx.listener(move |view, event: &KeyDownEvent, window, cx| {
                if is_activation_key(event) {
                    window.prevent_default();
                    key_action(&mut view.home, -1);
                    cx.notify();
                }
            }));
    } else {
        decrement = decrement.opacity(0.45).cursor_not_allowed();
    }

    let mut increment = button("", ButtonVariant::Outline)
        .id(format!("{id}-increment"))
        .w(px(26.))
        .h(px(26.))
        .p_0()
        .child(action_icon(ICON_PLUS, 13., TEXT));
    if !busy && current < max {
        let click_action = action.clone();
        let key_action = action.clone();
        increment = increment
            .on_click(cx.listener(move |view, _, _, cx| {
                click_action(&mut view.home, 1);
                cx.stop_propagation();
                cx.notify();
            }))
            .on_key_down(cx.listener(move |view, event: &KeyDownEvent, window, cx| {
                if is_activation_key(event) {
                    window.prevent_default();
                    key_action(&mut view.home, 1);
                    cx.notify();
                }
            }));
    } else {
        increment = increment.opacity(0.45).cursor_not_allowed();
    }

    div()
        .flex()
        .items_center()
        .gap_1()
        .child(decrement)
        .child(
            div()
                .w(px(30.))
                .text_center()
                .font_family("Consolas")
                .text_size(px(12.))
                .text_color(rgb(TEXT))
                .child(current.to_string()),
        )
        .child(increment)
}

pub(super) fn task_option_switch<F>(
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
        let action = Rc::new(action);
        let click_action = action.clone();
        control = control.on_click(cx.listener(move |view, _, _, cx| {
            click_action(&mut view.home);
            cx.stop_propagation();
            cx.notify();
        }));
        control =
            control.on_key_down(cx.listener(move |view, event: &KeyDownEvent, window, cx| {
                if is_activation_key(event) {
                    window.prevent_default();
                    action(&mut view.home);
                    cx.notify();
                }
            }));
    } else {
        control = control.opacity(0.45).cursor_not_allowed();
    }
    control
}

pub(super) fn home_select(
    app: &AhabApp,
    cx: &mut Context<AhabApp>,
    config: HomeSelectConfig,
) -> Div {
    let HomeSelectConfig {
        select,
        current,
        options,
        id,
        width,
        disabled,
        on_change,
    } = config;
    let selected_index = options
        .iter()
        .position(|(value, _)| value == &current)
        .unwrap_or(0);
    let selected_label = options
        .get(selected_index)
        .map(|(_, label)| label.clone())
        .unwrap_or_else(|| current.clone());
    let values = options
        .iter()
        .map(|(value, _)| value.clone())
        .collect::<Vec<_>>();
    let open = app.home.is_select_open(select);
    let palette = crate::components::style::current_render_palette();
    let mut trigger = select_trigger(selected_label, open, &palette)
        .id(id.clone())
        .w(px(width));

    if disabled {
        trigger = trigger.opacity(0.5).cursor_not_allowed();
    } else {
        trigger = trigger.on_click(cx.listener(move |view, _, _, cx| {
            if open {
                view.home.close_select();
            } else {
                view.home.toggle_select(select);
            }
            cx.stop_propagation();
            cx.notify();
        }));
        let key_change = on_change.clone();
        let key_values = values.clone();
        let key_current = current.clone();
        trigger =
            trigger.on_key_down(cx.listener(move |view, event: &KeyDownEvent, window, cx| {
                let key = event.keystroke.key.to_ascii_lowercase();
                if key == "escape" {
                    window.prevent_default();
                    view.home.close_select();
                    cx.notify();
                    return;
                }
                if matches!(key.as_str(), "enter" | "space") {
                    window.prevent_default();
                    view.home.toggle_select(select);
                    cx.notify();
                    return;
                }

                let current_index = key_values
                    .iter()
                    .position(|candidate| candidate == &key_current)
                    .unwrap_or(0);
                let next_index = match key.as_str() {
                    "left" | "arrowleft" | "up" | "arrowup" => Some(
                        (current_index as isize - 1).rem_euclid(key_values.len().max(1) as isize)
                            as usize,
                    ),
                    "right" | "arrowright" => Some((current_index + 1) % key_values.len().max(1)),
                    "down" | "arrowdown" if open => {
                        Some((current_index + 1) % key_values.len().max(1))
                    }
                    "down" | "arrowdown" => Some(current_index),
                    "home" => Some(0),
                    "end" => key_values.len().checked_sub(1),
                    _ => None,
                };
                if let Some(next_index) = next_index
                    && let Some(value) = key_values.get(next_index)
                {
                    window.prevent_default();
                    if matches!(key.as_str(), "down" | "arrowdown" | "up" | "arrowup") && !open {
                        view.home.toggle_select(select);
                    } else {
                        key_change(&mut view.home, value.clone());
                        if !open {
                            view.home.close_select();
                        }
                    }
                    cx.notify();
                }
            }));
    }

    let mut option_list = div().flex().flex_col().gap_1();
    for (value, option_label) in options {
        let selected = value == current;
        let option_id = format!("{id}-option-{value}");
        let click_change = on_change.clone();
        let click_value = value.clone();
        let mut option = select_option(option_label, selected, &palette)
            .id(option_id)
            .on_click(cx.listener(move |view, _, _, cx| {
                click_change(&mut view.home, click_value.clone());
                view.home.close_select();
                cx.stop_propagation();
                cx.notify();
            }));
        let key_change = on_change.clone();
        let key_value = value;
        option = option.on_key_down(cx.listener(move |view, event: &KeyDownEvent, window, cx| {
            if is_activation_key(event) {
                window.prevent_default();
                key_change(&mut view.home, key_value.clone());
                view.home.close_select();
                cx.notify();
            } else if event.keystroke.key.eq_ignore_ascii_case("escape") {
                window.prevent_default();
                view.home.close_select();
                cx.notify();
            }
        }));
        option_list = option_list.child(option);
    }

    let mut root = div().relative().w(px(width)).child(trigger);
    if open {
        let popup = select_popup(option_list, &palette)
            .shadow_sm()
            .on_mouse_down_out(cx.listener(move |view, _, _, cx| {
                view.home.close_select();
                cx.notify();
            }));
        root = root.child(deferred(popup).priority(10));
    }
    root
}
