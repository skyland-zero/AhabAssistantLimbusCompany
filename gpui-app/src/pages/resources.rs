//! Resource status and synchronization page backed by the shared IPC state.
//!
//! The page provides a compact update toolbar, a responsive resource-card
//! grid, and explicit synchronization progress, success, warning, loading,
//! and empty states.

use std::time::Duration;

use gpui::{Context, Div, div, prelude::*, px, relative};

use crate::{
    app::{ACCENT, AhabApp, BACKGROUND, BORDER, SURFACE, TEXT, TEXT_MUTED},
    components::style::GREEN,
    components::{
        BadgeTone, ButtonVariant, action_button, badge, card, empty_state, is_activation_key,
        loading, render_rgb as rgb, scroll_area_with_id, svg_icon,
    },
    i18n::paired as text,
    model::{Language, ResourceGroup},
};

const ICON_REFRESH: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="2"><path d="M20 11a8.1 8.1 0 0 0-15.5-2M4 5v4h4"/><path d="M4 13a8.1 8.1 0 0 0 15.5 2M20 19v-4h-4"/></svg>"#;
const ICON_SEARCH_CHECK: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="2"><circle cx="11" cy="11" r="8"/><path d="m21 21-4.3-4.3"/><path d="m8 11 2 2 4-4"/></svg>"#;

pub fn render(app: &mut AhabApp, cx: &mut Context<AhabApp>) -> Div {
    // The mock emits its terminal progress event synchronously. Keep that
    // state alive for one visible frame so the native client shows the same
    // progress feedback as the asynchronous React IPC path.
    if app.resources.sync_progress == Some(100) && !app.resources.sync_finish_scheduled {
        app.resources.sync_finish_scheduled = true;
        let done_message = match app.state.settings.language {
            Language::ZhCn => "资源同步完成",
            Language::EnUs => "Resource sync completed",
        };
        cx.spawn(async move |this, cx| {
            cx.background_executor()
                .timer(Duration::from_millis(300))
                .await;
            let _ = this.update(cx, |view, cx| {
                view.resources.finish_sync();
                view.show_toast(crate::shell::ToastKind::Success, done_message, cx);
                cx.notify();
            });
        })
        .detach();
    }

    let language = app.state.settings.language;
    let progress = app.resources.sync_progress;
    let feedback = app.resources.feedback.clone();
    let groups = app.resources.groups.clone();

    let mut check = action_button(
        text("检查更新", "Check for Updates").get(language),
        ButtonVariant::Outline,
        Some(svg_icon(ICON_SEARCH_CHECK, 14., TEXT)),
        28.,
    )
    .id("resources-check");
    check = check
        .on_click(cx.listener(move |view, _, _, cx| {
            view.resources.check_update();
            view.show_toast(
                crate::shell::ToastKind::Success,
                text("资源更新状态已刷新", "Resource update status refreshed").get(language),
                cx,
            );
            cx.notify();
        }))
        .on_key_down(
            cx.listener(move |view, event: &gpui::KeyDownEvent, window, cx| {
                if is_activation_key(event) {
                    window.prevent_default();
                    view.resources.check_update();
                    view.show_toast(
                        crate::shell::ToastKind::Success,
                        text("资源更新状态已刷新", "Resource update status refreshed")
                            .get(language),
                        cx,
                    );
                    cx.notify();
                }
            }),
        );

    let mut sync = action_button(
        text("立即同步", "Sync Now").get(language),
        ButtonVariant::Default,
        Some(svg_icon(ICON_REFRESH, 14., TEXT)),
        28.,
    )
    .id("resources-sync");
    if progress.is_none() {
        sync = sync
            .on_click(cx.listener(move |view, _, _, cx| {
                view.resources.sync_now();
                view.show_toast(
                    crate::shell::ToastKind::Loading,
                    text("资源同步中…", "Syncing resources…").get(language),
                    cx,
                );
                cx.notify();
            }))
            .on_key_down(
                cx.listener(move |view, event: &gpui::KeyDownEvent, window, cx| {
                    if is_activation_key(event) {
                        window.prevent_default();
                        view.resources.sync_now();
                        view.show_toast(
                            crate::shell::ToastKind::Loading,
                            text("资源同步中…", "Syncing resources…").get(language),
                            cx,
                        );
                        cx.notify();
                    }
                }),
            );
    } else {
        sync = sync.opacity(0.5).cursor_not_allowed();
    }

    let progress_badge = progress.map(|value| {
        let mut status = badge("", BadgeTone::Accent);
        status = status
            .child(svg_icon(ICON_REFRESH, 12., ACCENT))
            .child(format!(
                "{} {}%",
                text("同步中…", "Syncing…").get(language),
                value
            ));
        status
    });
    let toolbar_status = progress_badge.unwrap_or_else(|| div().w(px(1.)).h(px(1.)));

    let toolbar = div()
        .flex()
        .items_center()
        .justify_between()
        .gap_3()
        .border_b_1()
        .border_color(rgb(BORDER))
        .bg(rgb(SURFACE))
        .px_5()
        .py(px(10.))
        .child(toolbar_status)
        .child(
            div()
                .flex()
                .items_center()
                .gap_1p5()
                .child(check)
                .child(sync),
        );

    let cards: Vec<Div> = groups
        .into_iter()
        .map(|group| resource_card(group, progress, language))
        .collect();
    let has_groups = !cards.is_empty();
    // The React page stays one column below the lg=1024 breakpoint. GPUI's
    // minimum window is below that breakpoint, so a flex column keeps the
    // same max-width and avoids an unmeasured grid during the first frame.
    let grid = div()
        .w_full()
        .max_w(px(768.))
        .flex()
        .flex_col()
        .gap(px(12.))
        .children(cards);

    let body = if has_groups {
        div().w_full().child(grid)
    } else if progress.is_some() {
        div().w_full().child(loading(
            text("正在加载资源状态…", "Loading resource status…").get(language),
        ))
    } else {
        div().w_full().child(empty_state(
            text("暂无资源", "No resources").get(language),
            text(
                "资源状态将在后端接入后显示。",
                "Resource status will appear when the backend is connected.",
            )
            .get(language),
        ))
    };

    let mut content = div().w_full().p_6().child(body);
    if let Some(feedback) = feedback {
        content = content.child(
            div()
                .mt_4()
                .text_size(px(12.))
                .text_color(rgb(GREEN))
                .child(localized_feedback(&feedback, language)),
        );
    }

    div()
        .size_full()
        .flex()
        .flex_col()
        .bg(rgb(BACKGROUND))
        .child(toolbar)
        .child(
            scroll_area_with_id("resources-scroll", content)
                .flex_1()
                .min_h_0(),
        )
}

fn resource_card(group: ResourceGroup, progress: Option<u8>, language: Language) -> Div {
    let has_update = group
        .remoteVersion
        .as_ref()
        .is_some_and(|version| version != &group.localVersion);
    let status = if let Some(value) = progress {
        badge(
            format!("{} {}%", text("同步中…", "Syncing…").get(language), value),
            BadgeTone::Accent,
        )
    } else if has_update {
        badge(
            format!(
                "{} {}",
                text("可更新到", "Update available:").get(language),
                group.remoteVersion.clone().unwrap_or_default()
            ),
            BadgeTone::Accent,
        )
    } else {
        badge(
            text("已是最新", "Up to date").get(language),
            BadgeTone::Success,
        )
    };

    let mut body = div()
        .flex()
        .flex_col()
        .gap_3()
        .px_5()
        .py_4()
        .child(
            div()
                .flex()
                .items_center()
                .justify_between()
                .gap_2()
                .child(
                    div()
                        .min_w_0()
                        .text_size(px(14.))
                        .text_color(rgb(TEXT))
                        .child(group.name),
                )
                .child(status),
        )
        .child(version_row(
            text("本地版本", "Local Version").get(language),
            group.localVersion,
        ))
        .child(version_row(
            text("远端版本", "Remote Version").get(language),
            group.remoteVersion.unwrap_or_else(|| "—".to_owned()),
        ))
        .child(version_row(
            text("上次同步", "Last Synced").get(language),
            format_sync_time(group.lastSyncAt, language),
        ));

    if let Some(value) = progress {
        let fraction = f32::from(value.min(100)) / 100.;
        body = body.child(
            div()
                .h(px(4.))
                .w_full()
                .rounded_full()
                .bg(rgb(BORDER))
                .child(
                    div()
                        .h_full()
                        .w(relative(fraction))
                        .rounded_full()
                        .bg(rgb(ACCENT)),
                ),
        );
    }

    card(body).p_0().w_full()
}

fn version_row(label: &'static str, value: String) -> Div {
    div()
        .flex()
        .items_center()
        .justify_between()
        .gap_3()
        .text_size(px(12.))
        .child(div().text_color(rgb(TEXT_MUTED)).child(label))
        .child(
            div()
                .font_family("Consolas")
                .text_color(rgb(TEXT))
                .child(value),
        )
}

fn format_sync_time(timestamp: Option<i64>, language: Language) -> String {
    match timestamp {
        None | Some(0) => text("从未同步", "Never").get(language).to_owned(),
        Some(timestamp) => match language {
            Language::ZhCn => format!("时间戳 {timestamp}"),
            Language::EnUs => format!("Timestamp {timestamp}"),
        },
    }
}

fn localized_feedback(feedback: &str, language: Language) -> String {
    if matches!(language, Language::ZhCn) {
        return feedback.to_owned();
    }
    match feedback {
        "发现资源更新" => "Resource updates found".to_owned(),
        "资源已是最新版本" => "Resources are up to date".to_owned(),
        "资源同步完成" => "Resource sync completed".to_owned(),
        _ => feedback.to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::format_sync_time;
    use crate::model::Language;

    #[test]
    fn zero_timestamp_is_not_reported_as_a_real_sync() {
        assert_eq!(format_sync_time(None, Language::ZhCn), "从未同步");
        assert_eq!(format_sync_time(Some(0), Language::ZhCn), "从未同步");
        assert_eq!(format_sync_time(None, Language::EnUs), "Never");
    }
}
