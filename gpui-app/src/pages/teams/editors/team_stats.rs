use super::*;

use crate::model::{TeamStats, TeamStatsBucket};

use gpui::WeakEntity;

fn duration_label(seconds: f64, language: Language) -> String {
    if seconds <= 0.0 {
        return text("暂无", "—").get(language).to_owned();
    }
    let centiseconds = (seconds * 100.0).round() as u64;
    let minutes = centiseconds / 6_000;
    let seconds = (centiseconds % 6_000) / 100;
    let fraction = centiseconds % 100;
    format!("{minutes}:{seconds:02}.{fraction:02}")
}

fn stats_value(value: impl Into<String>) -> Div {
    div()
        .text_size(px(12.))
        .text_color(palette_rgb(current_render_palette().foreground))
        .child(value.into())
}

fn stats_bucket_rows(
    bucket: &TeamStatsBucket,
    label: &'static str,
    label_en: &'static str,
    language: Language,
) -> Vec<Div> {
    let runs = match language {
        Language::ZhCn => format!("{label}次数"),
        Language::EnUs => format!("{label_en} Runs"),
    };
    let average = match language {
        Language::ZhCn => format!("{label}平均用时"),
        Language::EnUs => format!("{label_en} Average"),
    };
    let last_five = match language {
        Language::ZhCn => format!("{label}最近 5 次平均用时"),
        Language::EnUs => format!("{label_en} Last 5 Average"),
    };
    let last_ten = match language {
        Language::ZhCn => format!("{label}最近 10 次平均用时"),
        Language::EnUs => format!("{label_en} Last 10 Average"),
    };
    vec![
        control_row(runs, stats_value(bucket.count.to_string())),
        control_row(
            average,
            stats_value(duration_label(bucket.averageSeconds, language)),
        ),
        control_row(
            last_five,
            stats_value(duration_label(bucket.last5AverageSeconds, language)),
        ),
        control_row(
            last_ten,
            stats_value(duration_label(bucket.last10AverageSeconds, language)),
        ),
    ]
}

pub(crate) fn team_stats_editor(
    root: &WeakEntity<AhabApp>,
    app: &AhabApp,
    language: Language,
) -> Div {
    let Some(editor) = app.teams.editor.as_ref() else {
        return div();
    };
    let stats: TeamStats = editor.stats.clone();
    let team_id = editor.team.id.clone();
    let stats_loading = editor.stats_loading;
    let clearing = editor.stats_clearing;
    let error = editor.stats_error.is_some();
    let can_act = !team_id.is_empty() && !stats_loading && !clearing;

    let mut refresh = button(
        "team-stats-refresh",
        text("刷新数据", "Refresh Data").get(language),
        ButtonVariant::Outline,
    )
    .h(px(30.))
    .px_3()
    .py_0();
    if can_act {
        refresh = refresh
            .on_click(app_listener(root, |view, _, _, cx| {
                view.refresh_team_stats(cx)
            }))
            .on_key_down(app_listener(
                root,
                |view, event: &KeyDownEvent, window, cx| {
                    if team_activation_key(event) {
                        window.prevent_default();
                        view.refresh_team_stats(cx);
                    }
                },
            ));
    } else {
        refresh = refresh.opacity(0.5).cursor_default();
    }

    let mut clear = button(
        "team-stats-clear",
        text("清除历史统计数据", "Clear History").get(language),
        ButtonVariant::Destructive,
    )
    .h(px(30.))
    .px_3()
    .py_0();
    if can_act {
        clear = clear
            .on_click(app_listener(root, |view, _, window, cx| {
                view.open_clear_stats_confirmation(window, cx)
            }))
            .on_key_down(app_listener(
                root,
                |view, event: &KeyDownEvent, window, cx| {
                    if team_activation_key(event) {
                        window.prevent_default();
                        view.open_clear_stats_confirmation(window, cx);
                    }
                },
            ));
    } else {
        clear = clear.opacity(0.5).cursor_default();
    }

    let mut rows = stats_bucket_rows(&stats.hard, "困难镜牢", "Hard Mirror", language);
    rows.extend(stats_bucket_rows(
        &stats.normal,
        "普通镜牢",
        "Normal Mirror",
        language,
    ));
    let body = if stats_loading {
        loading(text("统计数据加载中…", "Loading team statistics…").get(language))
            .into_any_element()
    } else if error {
        div()
            .text_size(px(11.))
            .text_color(palette_rgb(current_render_palette().danger))
            .child(
                text(
                    "统计数据加载失败，请点击刷新重试。",
                    "Statistics failed to load. Click Refresh to try again.",
                )
                .get(language),
            )
            .into_any_element()
    } else {
        div()
            .flex()
            .flex_col()
            .gap_2()
            .child(control_row(
                text("总镜牢次数", "Total Mirror Runs").get(language),
                stats_value(stats.totalCount.to_string()),
            ))
            .child(editor_option_grid(rows))
            .into_any_element()
    };

    editor_card(
        div()
            .flex()
            .flex_col()
            .gap_3()
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .gap_2()
                    .child(editor_section_title(
                        text("编队统计数据", "Team Statistics").get(language),
                    ))
                    .child(div().flex().gap_2().child(refresh).child(clear)),
            )
            .child(body)
            .child(if clearing {
                loading(text("统计数据清除中…", "Clearing statistics…").get(language))
                    .into_any_element()
            } else {
                div().into_any_element()
            }),
    )
}
