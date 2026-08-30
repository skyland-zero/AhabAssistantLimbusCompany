use super::*;

pub(crate) fn render(app: &mut AhabApp, cx: &mut Context<AhabApp>) -> Div {
    let language = app.state.settings.language;
    let filter = app.teams.filter;
    let teams: Vec<TeamDetail> = app.teams.filtered_teams().cloned().collect();

    // The React TabsList is a compact, muted strip. Keeping the count as a
    // separate pill makes zero-count purpose tabs match the source behavior.
    let mut filter_bar = div()
        .flex()
        .items_center()
        .gap_1()
        .flex_wrap()
        .p(px(2.))
        .rounded_md()
        .bg(palette_rgb(current_render_palette().muted));
    for candidate in TeamFilter::ALL {
        let active = candidate == filter;
        let count = app.teams.count_for(candidate);
        let mut control = div()
            .id(format!("team-filter-{candidate:?}"))
            .flex()
            .items_center()
            .gap_1()
            .h(px(28.))
            .px_3()
            .rounded_md()
            .tab_index(0)
            .cursor_pointer()
            .text_size(px(12.))
            .text_color(palette_rgb(if active {
                current_render_palette().foreground
            } else {
                current_render_palette().muted_foreground
            }))
            .bg(palette_rgb(if active {
                current_render_palette().card
            } else {
                current_render_palette().muted
            }));
        control = control.child(filter_label(candidate, language));
        if candidate == TeamFilter::All || count > 0 {
            control = control.child(
                div()
                    .px_1()
                    .rounded_md()
                    .bg(palette_rgb(current_render_palette().secondary))
                    .text_size(px(10.))
                    .text_color(palette_rgb(current_render_palette().muted_foreground))
                    .child(count.to_string()),
            );
        }
        control = control
            .on_click(cx.listener(move |view, _, _, cx| {
                view.teams.set_filter(candidate);
                cx.notify();
            }))
            .on_key_down(cx.listener(move |view, event: &KeyDownEvent, window, cx| {
                if team_activation_key(event) {
                    window.prevent_default();
                    view.teams.set_filter(candidate);
                    cx.notify();
                }
            }));
        filter_bar = filter_bar.child(control);
    }

    let mut new_team = div()
        .id("new-team")
        .flex()
        .items_center()
        .justify_center()
        .gap_1()
        .h(px(32.))
        .px_3()
        .rounded_md()
        .tab_index(0)
        .cursor_pointer()
        .bg(rgb(crate::app::ACCENT))
        .text_size(px(12.))
        .text_color(palette_rgb(current_render_palette().brand_foreground))
        .focus_visible(|style| style.border_color(palette_rgb(current_render_palette().ring)))
        .child(icon(
            ICON_PLUS,
            14.,
            current_render_palette().brand_foreground,
        ))
        .child(text("新建队伍", "New Team").get(language))
        .on_click(cx.listener(|view, _, _, cx| view.open_new_team(cx)));
    new_team = new_team.on_key_down(cx.listener(|view, event: &KeyDownEvent, window, cx| {
        if team_activation_key(event) {
            window.prevent_default();
            view.open_new_team(cx);
        }
    }));

    let mut cards = div().flex().flex_wrap().gap_3().items_stretch();
    if app.teams.teams.is_empty() {
        cards = cards.child(
            empty_state(
                text("还没有队伍", "No teams yet").get(language),
                text(
                    "创建一支队伍开始配置镜牢策略。",
                    "Create a team to configure Mirror Dungeon strategies.",
                )
                .get(language),
            )
            .w_full()
            .min_h(px(240.)),
        );
    } else if teams.is_empty() {
        cards = cards.child(
            empty_state(
                text("该分类没有队伍", "No teams in this category").get(language),
                text(
                    "切换分类或创建一支新队伍。",
                    "Switch category or create a new team.",
                )
                .get(language),
            )
            .w_full()
            .min_h(px(240.)),
        );
    } else {
        // Keep cards readable in both languages while allowing two columns in
        // the default window. The row falls back to one column when the
        // available width cannot satisfy the 360px minimum.
        for team in teams {
            cards = cards.child(
                team_card(app, cx, team, language)
                    .flex_basis(px(360.))
                    .flex_grow(1.)
                    .flex_shrink(1.),
            );
        }
    }

    let mut root = page_root().flex_1().min_h_0();
    root = root.child(page_toolbar(
        div()
            .flex()
            .items_center()
            .justify_between()
            .gap_3()
            .flex_wrap()
            .flex_none()
            .child(filter_bar)
            .child(new_team),
    ));
    if let Some(feedback) = app.teams.feedback.clone() {
        root = root.child(
            card(
                div()
                    .flex_none()
                    .text_size(px(11.))
                    .text_color(palette_rgb(current_render_palette().success))
                    .child(localized_feedback(&feedback, language)),
            )
            .w_full()
            .p_3()
            .bg(palette_rgb(current_render_palette().success_light)),
        );
    }
    root.child(
        scroll_area_with_id(app, "teams-list-scroll", div().w_full().child(cards))
            .flex_1()
            .min_h_0(),
    )
}

fn team_card(
    app: &mut AhabApp,
    cx: &mut Context<AhabApp>,
    team: TeamDetail,
    language: Language,
) -> Div {
    let is_luxcavation = team.purpose == TeamPurpose::Luxcavation;
    let config = team.mirrorConfig.clone().unwrap_or_default();
    let discarded = discard_count(&config);
    let has_starlight = !is_luxcavation && config.opening_bonus.iter().any(|level| *level > 0);
    let team_id = team.id.clone();
    let edit_team = team.clone();
    let edit_team_for_key = edit_team.clone();
    let delete_team = team.clone();
    let delete_team_for_key = delete_team.clone();

    let mut edit = div()
        .id(format!("edit-team-{team_id}"))
        .flex()
        .items_center()
        .justify_center()
        .size(px(32.))
        .rounded_md()
        .tab_index(0)
        .cursor_pointer()
        .focus_visible(|style| style.border_color(palette_rgb(current_render_palette().ring)))
        .child(icon(
            ICON_EDIT,
            16.,
            current_render_palette().muted_foreground,
        ))
        .on_click(cx.listener(move |view, _, _, cx| {
            view.open_existing_team(&edit_team, cx);
        }));
    edit = edit.on_key_down(cx.listener(move |view, event: &KeyDownEvent, window, cx| {
        if team_activation_key(event) {
            window.prevent_default();
            view.open_existing_team(&edit_team_for_key, cx);
        }
    }));
    let mut delete = div()
        .id(format!("delete-team-{team_id}"))
        .flex()
        .items_center()
        .justify_center()
        .size(px(32.))
        .rounded_md()
        .tab_index(0)
        .cursor_pointer()
        .focus_visible(|style| style.border_color(palette_rgb(current_render_palette().ring)))
        .child(icon(ICON_TRASH, 16., current_render_palette().danger))
        .on_click(cx.listener(move |view, _, _, cx| {
            view.teams.request_delete(delete_team.clone());
            cx.notify();
        }));
    delete = delete.on_key_down(cx.listener(move |view, event: &KeyDownEvent, window, cx| {
        if team_activation_key(event) {
            window.prevent_default();
            view.teams.request_delete(delete_team_for_key.clone());
            cx.notify();
        }
    }));

    let mut sinner_badges = div().flex().flex_wrap().gap_1();
    for (index, sinner) in team.sinners.iter().enumerate() {
        sinner_badges = sinner_badges.child(badge(
            format!("#{} {}", index + 1, app.teams.sinner_name(sinner)),
            BadgeTone::Neutral,
        ));
    }

    let scheme = normalized_scheme(&team.accessoryScheme);
    let mut header = div()
        .flex()
        .items_center()
        .gap_2()
        .flex_wrap()
        .child(
            div()
                .min_w_0()
                .text_size(px(14.))
                .text_color(rgb(TEXT))
                .child(team.name.clone()),
        )
        .child(badge(
            purpose_label(team.purpose, language),
            BadgeTone::Neutral,
        ));
    if !is_luxcavation {
        header = header.child(scheme_badge(scheme, language));
        if !team.enabled {
            header = header.child(badge(
                text("已停用", "Disabled").get(language),
                BadgeTone::Neutral,
            ));
        }
    }

    let mut details = div()
        .flex()
        .items_center()
        .flex_wrap()
        .gap_2()
        .text_size(px(11.))
        .text_color(rgb(TEXT_MUTED))
        .child(if matches!(language, Language::ZhCn) {
            format!("{} 人格", team.sinners.len())
        } else {
            format!("{} sinners", team.sinners.len())
        });
    if has_starlight {
        details = details.child(
            div()
                .flex()
                .items_center()
                .gap_1()
                .text_color(rgb(crate::app::ACCENT))
                .child(icon(ICON_SPARKLES, 13., current_render_palette().brand))
                .child(text("已配星光", "Starlight ready").get(language)),
        );
    }
    if !is_luxcavation && config.second_system {
        details = details.child(badge(
            text("第二体系", "2nd system").get(language),
            BadgeTone::Neutral,
        ));
    }
    if !is_luxcavation && discarded > 0 {
        details = details.child(badge(
            if matches!(language, Language::ZhCn) {
                format!("舍弃 {} 项", discarded)
            } else {
                format!("Discard ×{}", discarded)
            },
            BadgeTone::Danger,
        ));
    }
    if !is_luxcavation && config.defense_for_solo {
        details = details.child(badge(
            text("良秀单通", "Solo pass").get(language),
            BadgeTone::Accent,
        ));
    }
    if !is_luxcavation && config.use_team_code {
        details = details.child(badge(
            text("编队码", "Team code").get(language),
            BadgeTone::Neutral,
        ));
    }

    card(
        div()
            .flex()
            .items_start()
            .gap_3()
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .child(header)
                    .child(details)
                    .child(sinner_badges),
            )
            .child(div().flex().flex_none().gap_1().child(edit).child(delete)),
    )
    .p_3()
    .opacity(if is_luxcavation || team.enabled {
        1.
    } else {
        0.6
    })
    .hover(|style| style.bg(palette_rgb(current_render_palette().secondary)))
}
