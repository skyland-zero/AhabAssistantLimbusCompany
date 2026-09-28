use super::*;

/// The daily-stats sheet body, rendered from a live handle on the app.
///
/// A `Root` sheet's builder runs *inside* `AhabApp::render`, so it can neither
/// read nor update the app (that panics with "cannot update `AhabApp` while it
/// is already being updated"). `preset_picker_body` works around that with a
/// snapshot taken when the sheet opens, which only covers data that is already
/// loaded.
///
/// This is the alternative for data that arrives later: a child view. GPUI
/// calls its `render` while laying out the elements `AhabApp::render` returned,
/// which is after that call released its borrow of the app - so the app can be
/// read here, and `_app_events` repaints the body once the fetch lands.
pub(crate) struct DailyDetailsView {
    root: WeakEntity<AhabApp>,
    /// Never dropped: the subscription is what brings the asynchronously
    /// fetched rows into a sheet that is already on screen.
    _app_events: gpui::Subscription,
}

impl DailyDetailsView {
    pub(crate) fn new(root: gpui::Entity<AhabApp>, cx: &mut Context<Self>) -> Self {
        let app_events = cx.observe(&root, |_, _, cx| cx.notify());
        Self {
            root: root.downgrade(),
            _app_events: app_events,
        }
    }
}

impl Render for DailyDetailsView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let Some(root) = self.root.upgrade() else {
            return div().into_any_element();
        };

        // Copied out in one borrow so the rows below are built while holding
        // nothing: `daily_details_body` needs the handle, not the app.
        let (language, loading, error, data, selected) = {
            let app = root.read(cx);
            let language = app.state.settings.language;
            let loading = app.home.stats_details_loading;
            let error = app.home.stats_details_error.clone();
            let data = app.home.daily_stats.clone();
            let selected = data.as_ref().and_then(|data| {
                app.home
                    .stats_selected_date
                    .as_deref()
                    .and_then(|date| data.days.iter().find(|day| day.date == date))
                    .or_else(|| data.days.first())
                    .cloned()
            });
            (language, loading, error, data, selected)
        };

        let body = if loading {
            div()
                .flex_1()
                .flex()
                .items_center()
                .justify_center()
                .text_size(px(12.))
                .text_color(rgb(TEXT_MUTED))
                .child(text("正在加载每日统计…", "Loading daily statistics…").get(language))
        } else if let Some(error) = error {
            div()
                .flex_1()
                .flex()
                .items_center()
                .justify_center()
                .text_size(px(12.))
                .text_color(palette_rgb(current_render_palette().danger))
                .child(error)
        } else if let Some(data) = data {
            daily_details_body(&self.root, &data, selected.as_ref(), language)
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

        div()
            .flex()
            .flex_col()
            .w_full()
            .flex_1()
            .min_h_0()
            .child(body)
            .into_any_element()
    }
}

pub(crate) fn mirror_history_body(
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
    scroll_area_with_id("stats-mirror-scroll", list)
        .flex_1()
        .min_h_0()
        .into_any_element()
}

pub(crate) fn daily_details_body(
    root: &WeakEntity<AhabApp>,
    data: &crate::model::DailyStatsPayload,
    selected: Option<&DailyStatEntry>,
    language: Language,
) -> Div {
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
        // A plain closure, not `cx.listener`: this body is built from a view
        // that only holds a weak handle on the app.
        let row_root = root.clone();
        row = row.on_click(move |_, _, cx| {
            let date = date.clone();
            if let Some(root) = row_root.upgrade() {
                root.update(cx, |view, cx| {
                    view.home.select_stats_date(date);
                    cx.stop_propagation();
                    cx.notify();
                });
            }
        });
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
        scroll_area_with_id("stats-daily-scroll", table)
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
