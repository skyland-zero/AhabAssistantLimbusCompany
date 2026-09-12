use super::*;

pub(crate) fn daily_details_overlay(
    app: &mut AhabApp,
    cx: &mut Context<AhabApp>,
) -> gpui::AnyElement {
    if !app.home.stats_details_open {
        return div().into_any_element();
    }

    let language = app.state.settings.language;
    let palette = current_render_palette();
    let selected_date = app.home.stats_selected_date.clone();
    let selected_entry = app
        .home
        .daily_stats
        .as_ref()
        .and_then(|data| {
            selected_date
                .as_deref()
                .and_then(|date| data.days.iter().find(|day| day.date == date))
                .or_else(|| data.days.first())
        })
        .cloned();

    let mut close = button("", ButtonVariant::Icon)
        .id("stats-daily-close")
        .w(px(30.0))
        .h(px(30.0))
        .p_0()
        .child(action_icon(ICON_X, 15., TEXT_MUTED));
    close = close.on_click(cx.listener(|view, _, _, cx| {
        view.close_stats_details(cx);
        cx.stop_propagation();
    }));
    close = close.on_key_down(cx.listener(|view, event: &KeyDownEvent, window, cx| {
        if is_activation_key(event) {
            window.prevent_default();
            view.close_stats_details(cx);
        }
    }));

    let header = div()
        .flex()
        .items_center()
        .justify_between()
        .gap_3()
        .px(px(18.0))
        .py(px(12.0))
        .border_b_1()
        .border_color(rgb(BORDER))
        .child(
            div()
                .flex()
                .items_center()
                .gap_2()
                .child(action_icon(ICON_CALENDAR_CHECK, 17., ACCENT))
                .child(
                    div()
                        .text_size(px(16.))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(rgb(TEXT))
                        .child(text("每日刷本明细", "Daily Run Details").get(language)),
                ),
        )
        .child(close);

    let body = if app.home.stats_details_loading {
        div()
            .flex_1()
            .flex()
            .items_center()
            .justify_center()
            .text_size(px(12.))
            .text_color(rgb(TEXT_MUTED))
            .child(text("正在加载每日统计…", "Loading daily statistics…").get(language))
    } else if let Some(error) = app.home.stats_details_error.clone() {
        div()
            .flex_1()
            .flex()
            .items_center()
            .justify_center()
            .text_size(px(12.))
            .text_color(palette_rgb(palette.danger))
            .child(error)
    } else if let Some(data) = app.home.daily_stats.clone() {
        daily_details_body(app, cx, &data, selected_entry.as_ref())
    } else {
        div()
            .flex_1()
            .flex()
            .items_center()
            .justify_center()
            .text_size(px(12.))
            .text_color(rgb(TEXT_MUTED))
            .child(text("暂无每日统计", "No daily statistics yet").get(language))
    };

    let dialog = div()
        .id("stats-daily-dialog")
        .w(px(640.0))
        .h(px(520.0))
        .max_w_full()
        .max_h(relative(0.94))
        .min_h_0()
        .overflow_hidden()
        .flex()
        .flex_col()
        .rounded_lg()
        .border_1()
        .border_color(rgb(BORDER))
        .bg(rgb(SURFACE))
        .on_click(cx.listener(|_, _, _, cx| cx.stop_propagation()))
        .child(header)
        .child(body);

    let mut surface = div()
        .id("stats-daily-overlay")
        .relative()
        .size_full()
        .flex()
        .items_center()
        .justify_center()
        .p_4()
        .bg(rgba(0x00000080))
        .on_click(cx.listener(|view, _, _, cx| {
            view.close_stats_details(cx);
        }));
    surface = surface.capture_key_down(cx.listener(|view, event: &KeyDownEvent, window, cx| {
        if event.keystroke.key.eq_ignore_ascii_case("escape") {
            window.prevent_default();
            cx.stop_propagation();
            view.close_stats_details(cx);
        }
    }));

    div()
        .absolute()
        .top_0()
        .left_0()
        .right_0()
        .bottom_0()
        .child(surface.child(dialog))
        .into_any_element()
}

pub(crate) fn mirror_details_overlay(
    app: &mut AhabApp,
    cx: &mut Context<AhabApp>,
) -> gpui::AnyElement {
    if !app.home.mirror_details_open {
        return div().into_any_element();
    }

    let language = app.state.settings.language;
    let records = if app.home.stats.mirrorHistory.is_empty() {
        app.home.stats.lastMirror.clone().into_iter().collect()
    } else {
        app.home.stats.mirrorHistory.clone()
    };
    let mut close = button("", ButtonVariant::Icon)
        .id("stats-mirror-close")
        .w(px(30.0))
        .h(px(30.0))
        .p_0()
        .child(action_icon(ICON_X, 15., TEXT_MUTED));
    close = close.on_click(cx.listener(|view, _, _, cx| {
        view.close_mirror_details(cx);
        cx.stop_propagation();
    }));
    close = close.on_key_down(cx.listener(|view, event: &KeyDownEvent, window, cx| {
        if is_activation_key(event) {
            window.prevent_default();
            view.close_mirror_details(cx);
        }
    }));

    let header = div()
        .flex()
        .items_center()
        .justify_between()
        .gap_3()
        .px(px(18.0))
        .py(px(12.0))
        .border_b_1()
        .border_color(rgb(BORDER))
        .child(
            div()
                .flex()
                .items_center()
                .gap_2()
                .child(action_icon(ICON_SCROLL_TEXT, 17., ACCENT))
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap(px(1.0))
                        .child(
                            div()
                                .text_size(px(16.))
                                .font_weight(FontWeight::SEMIBOLD)
                                .text_color(rgb(TEXT))
                                .child(text("镜牢明细", "Mirror Details").get(language)),
                        )
                        .child(div().text_size(px(10.)).text_color(rgb(TEXT_MUTED)).child(
                            format!(
                                "{} {}/30",
                                text("最近", "Latest").get(language),
                                records.len()
                            ),
                        )),
                ),
        )
        .child(close);

    let body = mirror_history_body(app, &records, language);
    let dialog = div()
        .id("stats-mirror-dialog")
        .w(px(820.0))
        .h(px(650.0))
        .max_w_full()
        .max_h(relative(0.94))
        .min_h_0()
        .overflow_hidden()
        .flex()
        .flex_col()
        .rounded_lg()
        .border_1()
        .border_color(rgb(BORDER))
        .bg(rgb(SURFACE))
        .on_click(cx.listener(|_, _, _, cx| cx.stop_propagation()))
        .child(header)
        .child(body);

    let mut surface = div()
        .id("stats-mirror-overlay")
        .relative()
        .size_full()
        .flex()
        .items_center()
        .justify_center()
        .p_4()
        .bg(rgba(0x00000080))
        .on_click(cx.listener(|view, _, _, cx| {
            view.close_mirror_details(cx);
        }));
    surface = surface.capture_key_down(cx.listener(|view, event: &KeyDownEvent, window, cx| {
        if event.keystroke.key.eq_ignore_ascii_case("escape") {
            window.prevent_default();
            cx.stop_propagation();
            view.close_mirror_details(cx);
        }
    }));

    div()
        .absolute()
        .top_0()
        .left_0()
        .right_0()
        .bottom_0()
        .child(surface.child(dialog))
        .into_any_element()
}

pub(crate) fn mirror_history_body(
    app: &mut AhabApp,
    records: &[MirrorCompletionStats],
    language: Language,
) -> gpui::AnyElement {
    if records.is_empty() {
        return div()
            .flex_1()
            .flex()
            .items_center()
            .justify_center()
            .text_size(px(12.))
            .text_color(rgb(TEXT_MUTED))
            .child(text("暂无镜牢完成记录", "No completed mirror runs").get(language))
            .into_any_element();
    }

    let mut list = div()
        .flex()
        .flex_col()
        .gap(px(8.0))
        .px(px(18.0))
        .pt(px(14.0))
        .pb(px(16.0));
    for (index, record) in records.iter().enumerate() {
        list = list.child(mirror_history_row(index + 1, record, language));
    }
    scroll_area_with_id(app, "stats-mirror-scroll", list)
        .flex_1()
        .min_h_0()
        .into_any_element()
}

pub(crate) fn daily_details_body(
    app: &mut AhabApp,
    cx: &mut Context<AhabApp>,
    data: &crate::model::DailyStatsPayload,
    selected: Option<&DailyStatEntry>,
) -> Div {
    let language = app.state.settings.language;
    let selected = selected.cloned().unwrap_or_default();
    let summary = div()
        .flex_none()
        .mx(px(18.0))
        .mt(px(14.0))
        .p_3()
        .rounded_md()
        .bg(rgba((ACCENT << 8) | 0x18))
        .child(
            div()
                .text_size(px(12.))
                .text_color(rgb(TEXT_MUTED))
                .child(format!(
                    "{}  ·  {}",
                    selected.date,
                    text("当日完成", "Completed").get(language)
                )),
        )
        .child(
            div()
                .flex()
                .items_center()
                .gap_3()
                .mt_1()
                .text_size(px(12.))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(rgb(TEXT))
                .child(format!(
                    "{} {}",
                    text("经验本", "EXP").get(language),
                    selected.exp
                ))
                .child(format!(
                    "{} {}",
                    text("纽本", "Thread").get(language),
                    selected.thread
                ))
                .child(format!(
                    "{} {}",
                    text("镜牢", "Mirror").get(language),
                    selected.mirror
                )),
        );

    let table_header = daily_table_row(
        text("日期", "Date").get(language),
        text("经验本", "EXP").get(language),
        text("纽本", "Thread").get(language),
        text("镜牢", "Mirror").get(language),
        text("合计", "Total").get(language),
        true,
    );
    let mut table = div().flex().flex_col().gap_1().px(px(18.0)).pb(px(14.0));
    for day in &data.days {
        let date = day.date.clone();
        let active = selected.date == day.date;
        let mut row = daily_table_row(
            &day.date,
            day.exp.to_string(),
            day.thread.to_string(),
            day.mirror.to_string(),
            day.total.to_string(),
            false,
        )
        .id(format!("stats-day-{}", day.date));
        if active {
            row = row.bg(rgba((ACCENT << 8) | 0x28));
        } else {
            row = row.hover(|style| style.bg(rgba((SURFACE_HOVER << 8) | 0x45)));
        }
        row = row.on_click(cx.listener(move |view, _, _, cx| {
            view.home.select_stats_date(date.clone());
            cx.stop_propagation();
            cx.notify();
        }));
        table = table.child(row);
    }
    let table_body: gpui::AnyElement = if data.days.is_empty() {
        div()
            .flex()
            .items_center()
            .justify_center()
            .h(px(100.0))
            .text_size(px(12.))
            .text_color(rgb(TEXT_MUTED))
            .child(text("暂无每日数据", "No daily data").get(language))
            .into_any_element()
    } else {
        scroll_area_with_id(app, "stats-daily-scroll", table)
            .flex_1()
            .min_h_0()
            .into_any_element()
    };

    div()
        .flex()
        .flex_col()
        .min_h_0()
        .flex_1()
        .child(summary)
        .child(table_header)
        .child(table_body)
}

pub(crate) fn daily_table_row(
    date: &str,
    exp: impl Into<String>,
    thread: impl Into<String>,
    mirror: impl Into<String>,
    total: impl Into<String>,
    header: bool,
) -> Div {
    div()
        .h(px(28.0))
        .flex_none()
        .items_center()
        .flex()
        .rounded_sm()
        .px_2()
        .text_size(px(if header { 10.0 } else { 11.0 }))
        .text_color(rgb(if header { TEXT_MUTED } else { TEXT }))
        .child(div().w(px(112.0)).flex_none().child(date.to_owned()))
        .child(daily_value(exp))
        .child(daily_value(thread))
        .child(daily_value(mirror))
        .child(daily_value(total))
}

pub(crate) fn daily_value(value: impl Into<String>) -> Div {
    div().flex_1().text_center().child(value.into())
}
