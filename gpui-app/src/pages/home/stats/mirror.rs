use super::*;

pub(crate) fn mirror_history_row(
    index: usize,
    record: &MirrorCompletionStats,
    language: Language,
) -> Div {
    let failed = record.failed.unwrap_or(false);
    let mode_label = if record.hardMode {
        text("困难", "Hard").get(language)
    } else {
        text("普通", "Normal").get(language)
    };
    let status_label = if failed {
        text("未完成", "Incomplete").get(language)
    } else {
        text("已完成", "Completed").get(language)
    };
    let status_tone = if failed {
        BadgeTone::Danger
    } else {
        BadgeTone::Success
    };
    let team = record.team.as_ref();
    let team_name = mirror_team_name(team, language);
    let team_number = team
        .map(|team| team.number)
        .filter(|number| *number > 0)
        .map(|number| format!(" #{number}"))
        .unwrap_or_default();
    let sinners = mirror_team_sinners(team, language);
    let route = mirror_route_name(record, language);
    let completed_at = record.completedAt.replace('T', " ");

    let heading = div()
        .flex()
        .items_center()
        .justify_between()
        .gap_2()
        .child(
            div()
                .min_w_0()
                .flex()
                .items_center()
                .gap_2()
                .child(
                    div()
                        .text_size(px(10.0))
                        .text_color(rgb(TEXT_MUTED))
                        .child(format!("#{index}")),
                )
                .child(
                    div()
                        .min_w_0()
                        .truncate()
                        .text_size(px(11.0))
                        .text_color(rgb(TEXT))
                        .child(completed_at),
                ),
        )
        .child(
            div()
                .flex()
                .items_center()
                .gap_1p5()
                .child(badge(
                    mode_label,
                    if record.hardMode {
                        BadgeTone::Warning
                    } else {
                        BadgeTone::Neutral
                    },
                ))
                .child(badge(status_label, status_tone))
                .child(
                    div()
                        .text_size(px(12.0))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(rgb(ACCENT))
                        .child(format_duration(record.totalSeconds)),
                ),
        );

    let metadata = div()
        .min_w_0()
        .flex()
        .items_center()
        .gap_1p5()
        .text_size(px(10.5))
        .text_color(rgb(TEXT_MUTED))
        .child(
            div()
                .min_w_0()
                .truncate()
                .text_color(rgb(TEXT))
                .child(format!("{}{}", team_name, team_number)),
        )
        .child(
            div()
                .text_color(rgb(ACCENT))
                .child(mirror_system_label(team, language)),
        )
        .child(
            div()
                .min_w_0()
                .truncate()
                .child(format!("{} · {}", route, sinners)),
        );
    let metadata = if let Some(reason) = record.failureReason.as_deref() {
        metadata.child(
            div()
                .min_w_0()
                .truncate()
                .text_color(palette_rgb(current_render_palette().danger))
                .child(format!(
                    "{}: {}",
                    text("原因", "Reason").get(language),
                    reason
                )),
        )
    } else {
        metadata
    };

    let mut timing = div().w_full().grid().grid_cols(3).gap_1();
    for (label, seconds) in [
        (
            text("战斗", "Battle").get(language).to_owned(),
            record.battleSeconds,
        ),
        (
            format!(
                "{} {}次",
                text("事件", "Events").get(language),
                record.eventCount
            ),
            record.eventSeconds,
        ),
        (
            text("商店", "Shop").get(language).to_owned(),
            record.shopSeconds,
        ),
        (
            text("寻路", "Path").get(language).to_owned(),
            record.findRoadSeconds,
        ),
        (
            text("主题包", "Theme").get(language).to_owned(),
            record.themePackSeconds,
        ),
        (
            text("奖励卡", "Reward").get(language).to_owned(),
            record.rewardCardSeconds,
        ),
        (
            text("饰品", "Ego").get(language).to_owned(),
            record.egoGiftSeconds,
        ),
        (
            text("结算", "Claim").get(language).to_owned(),
            record.settlementSeconds,
        ),
        (
            text("其他", "Other").get(language).to_owned(),
            record.otherSeconds,
        ),
    ] {
        timing = timing.child(mirror_history_timing(label, seconds));
    }

    card(
        div()
            .flex()
            .flex_col()
            .gap(px(6.0))
            .child(heading)
            .child(metadata)
            .child(timing),
    )
    .w_full()
    .p_3()
}

pub(crate) fn mirror_history_timing(label: String, seconds: f64) -> Div {
    div()
        .min_w_0()
        .flex()
        .items_center()
        .justify_between()
        .gap_1()
        .rounded_sm()
        .px(px(6.0))
        .py(px(3.0))
        .bg(rgba((SURFACE_HOVER << 8) | 0x35))
        .text_size(px(9.5))
        .child(
            div()
                .min_w_0()
                .truncate()
                .text_color(rgb(TEXT_MUTED))
                .child(label),
        )
        .child(
            div()
                .text_color(rgb(TEXT))
                .font_weight(FontWeight::MEDIUM)
                .child(format_duration(seconds)),
        )
}

pub(crate) fn mirror_context_line(record: &MirrorCompletionStats, language: Language) -> Div {
    let team = record.team.as_ref();
    let mode = if record.hardMode {
        text("困难", "Hard").get(language)
    } else {
        text("普通", "Normal").get(language)
    };
    div()
        .min_w_0()
        .flex()
        .items_center()
        .gap_1p5()
        .text_size(px(9.5))
        .text_color(rgb(TEXT_MUTED))
        .child(
            div()
                .min_w_0()
                .truncate()
                .text_color(rgb(TEXT))
                .child(mirror_team_name(team, language)),
        )
        .child(
            div()
                .text_color(rgb(ACCENT))
                .child(mirror_system_label(team, language)),
        )
        .child(div().text_color(rgb(TEXT_MUTED)).child(mode))
}

pub(crate) fn mirror_team_name(team: Option<&MirrorTeamStats>, language: Language) -> String {
    team.and_then(|team| (!team.name.is_empty()).then_some(team.name.clone()))
        .unwrap_or_else(|| text("未知队伍", "Unknown team").get(language).to_owned())
}

pub(crate) fn mirror_team_sinners(team: Option<&MirrorTeamStats>, language: Language) -> String {
    let Some(team) = team else {
        return text("未记录人格", "Sinners unavailable")
            .get(language)
            .to_owned();
    };
    let names = if matches!(language, Language::ZhCn) {
        &team.sinnerNames
    } else {
        &team.sinnerNamesEn
    };
    let values = if names.is_empty() {
        &team.sinners
    } else {
        names
    };
    if values.is_empty() {
        text("未记录人格", "Sinners unavailable")
            .get(language)
            .to_owned()
    } else {
        values.join(if matches!(language, Language::ZhCn) {
            "、"
        } else {
            ", "
        })
    }
}

pub(crate) fn mirror_system_label(team: Option<&MirrorTeamStats>, language: Language) -> String {
    let system = team
        .map(|team| {
            if !team.system.is_empty() {
                team.system.as_str()
            } else {
                team.accessoryScheme.as_str()
            }
        })
        .unwrap_or_default();
    let label = match system {
        "burn" => text("烧伤", "Burn"),
        "bleed" => text("流血", "Bleed"),
        "tremor" => text("震颤", "Tremor"),
        "rupture" => text("破裂", "Rupture"),
        "poise" => text("呼吸", "Poise"),
        "sinking" => text("沉沦", "Sinking"),
        "charge" => text("充能", "Charge"),
        "slash" => text("斩击", "Slash"),
        "pierce" => text("突刺", "Pierce"),
        "blunt" => text("打击", "Blunt"),
        _ if system.is_empty() => text("未知体系", "Unknown system"),
        _ => return system.to_owned(),
    };
    label.get(language).to_owned()
}

pub(crate) fn mirror_route_name(record: &MirrorCompletionStats, language: Language) -> String {
    let route = if matches!(language, Language::ZhCn) {
        &record.routeName
    } else {
        &record.routeNameEn
    };
    if !route.is_empty() {
        route.clone()
    } else if !record.routeId.is_empty() {
        record.routeId.clone()
    } else {
        text("默认路线", "Default route").get(language).to_owned()
    }
}
