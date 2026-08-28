use super::*;

pub(super) fn splitter(app: &mut AhabApp, cx: &mut Context<AhabApp>) -> gpui::Stateful<Div> {
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
        .child(div().w(px(2.0)).h(px(32.0)).rounded_full().bg(rgba(0)))
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
                    .clamp(RIGHT_PANEL_MIN_WIDTH, RIGHT_PANEL_MAX_WIDTH);
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

pub(super) fn right_panel(app: &mut AhabApp, cx: &mut Context<AhabApp>) -> Div {
    let language = app.state.settings.language;
    let width = bounded_right_panel_width(app.home.right_panel_width);
    let screenshot_card = screenshot_card(app, language);

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
        .child(super::device_panel::connection_card(app, cx).flex_none())
        .child(screenshot_card)
        .child(super::log_panel::logs_card(app, cx))
}

fn screenshot_card(app: &AhabApp, language: Language) -> Div {
    let screenshot_detail = app
        .home
        .latest_screenshot
        .as_ref()
        .map(|frame| {
            format!(
                "{} · {}×{}",
                text("收到最新画面", "Latest frame").get(language),
                frame.width,
                frame.height
            )
        })
        .unwrap_or_else(|| {
            text("等待游戏窗口画面接入", "Waiting for game screen connection")
                .get(language)
                .to_owned()
        });
    let screenshot_header = div()
        .h(px(32.0))
        .flex_none()
        .flex()
        .items_center()
        .px(px(10.0))
        .child(panel_heading(
            ICON_MONITOR_PLAY,
            text("实时画面", "Live Screen").get(language),
        ));
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
        .border_dashed()
        .border_color(rgb(SURFACE_HOVER))
        .bg(rgb(BACKGROUND))
        .text_size(px(11.0))
        .text_color(rgb(TEXT_MUTED))
        .child(
            div()
                .opacity(0.25)
                .child(action_icon(ICON_MONITOR_PLAY, 32., TEXT_MUTED)),
        )
        .child(screenshot_detail)
        .child(
            div()
                .text_size(px(10.0))
                .text_color(rgb(TEXT_MUTED))
                .child(text("16:9 · 1280×720", "16:9 · 1280×720").get(language)),
        );
    panel_card(
        div()
            .flex()
            .flex_col()
            .child(screenshot_header)
            .child(div().p(px(10.0)).child(screenshot_body)),
    )
    .flex_none()
}

pub(super) fn panel_card(child: impl IntoElement) -> Div {
    div()
        .min_w_0()
        .overflow_hidden()
        .rounded_lg()
        .border_1()
        .border_color(rgba(0))
        .bg(rgb(SURFACE))
        .child(child)
}

pub(super) fn panel_heading(icon_data: &'static [u8], title: &'static str) -> Div {
    div()
        .flex()
        .items_center()
        .gap_2()
        .text_size(px(12.0))
        .text_color(rgb(TEXT_MUTED))
        .child(action_icon(icon_data, 14., TEXT_MUTED))
        .child(title)
}

pub(super) fn first_executable_task(home: &HomeState) -> Option<FixedTaskId> {
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

pub(super) fn bounded_right_panel_width(width: f32) -> f32 {
    if !width.is_finite() {
        RIGHT_PANEL_DEFAULT_WIDTH
    } else {
        width.clamp(RIGHT_PANEL_MIN_WIDTH, RIGHT_PANEL_MAX_WIDTH)
    }
}

pub(super) fn reward_mode_label(mode: u8, language: Language) -> &'static str {
    match mode {
        0 => text("全部", "All").get(language),
        1 => text("狂气/通行证", "Lunacy/Pass").get(language),
        2 => text("邮件", "Mail").get(language),
        _ => text("全部", "All").get(language),
    }
}
