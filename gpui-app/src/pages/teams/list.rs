use super::*;

pub(crate) fn render(app: &mut AhabApp, cx: &mut Context<AhabApp>) -> Div {
    let language = app.state.settings.language;
    let filter = app.teams.filter;

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

    let fixed_slots = app.teams.fixed_slots_for_filter(filter);
    let extra_teams = app.teams.extra_teams_for_filter(filter);
    let has_fixed_slots = !fixed_slots.is_empty();
    let mut cards = div().flex().flex_col().gap_4();
    if fixed_slots.is_empty() && extra_teams.is_empty() {
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
        let mut fixed_cards = div().w_full().grid().grid_cols(2).gap_3().items_stretch();
        for slot in fixed_slots {
            let item = if let Some(team) = slot.team {
                team_card(app, cx, team, Some(slot.number), language)
            } else {
                empty_slot_card(app, cx, slot.number, language)
            };
            fixed_cards = fixed_cards.child(item.w_full());
        }
        if has_fixed_slots {
            cards = cards.child(fixed_cards);
        }

        if !extra_teams.is_empty() {
            if !matches!(filter, TeamFilter::General) {
                cards = cards.child(
                    div()
                        .text_size(px(12.))
                        .font_weight(gpui::FontWeight::MEDIUM)
                        .text_color(palette_rgb(current_render_palette().muted_foreground))
                        .child(text("额外编队", "Extra teams").get(language)),
                );
            }
            let mut extra_cards = div().w_full().grid().grid_cols(2).gap_3().items_stretch();
            for team in extra_teams {
                extra_cards = extra_cards.child(
                    team_card(
                        app,
                        cx,
                        team.clone(),
                        team_number_from_id(&team.id),
                        language,
                    )
                    .w_full(),
                );
            }
            cards = cards.child(extra_cards);
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
    slot_number: Option<u32>,
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
    let overwrite_team = team.clone();
    let overwrite_team_for_key = overwrite_team.clone();
    let toggle_team = team.clone();
    let toggle_team_for_key = toggle_team.clone();
    let toggle_target = !team.enabled;
    let toggle_pending = app.teams.team_toggle_busy();

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

    let mut overwrite = button(
        text("从预设覆盖", "Apply preset").get(language),
        ButtonVariant::Outline,
    )
    .id(format!("overwrite-team-{team_id}"))
    .h(px(30.))
    .px_2()
    .py_0()
    .text_size(px(10.))
    .on_click(cx.listener(move |view, _, _, cx| {
        view.open_team_preset_picker_for_team(&overwrite_team, cx);
    }));
    overwrite =
        overwrite.on_key_down(cx.listener(move |view, event: &KeyDownEvent, window, cx| {
            if team_activation_key(event) {
                window.prevent_default();
                view.open_team_preset_picker_for_team(&overwrite_team_for_key, cx);
            }
        }));

    let mut enabled_switch = switch(team.enabled).id(format!("toggle-team-{team_id}"));
    if toggle_pending {
        enabled_switch = enabled_switch.opacity(0.5);
    } else if !is_luxcavation {
        enabled_switch = enabled_switch.on_click(cx.listener(move |view, _, _, cx| {
            view.set_team_enabled(&toggle_team, toggle_target, cx);
        }));
        enabled_switch = enabled_switch.on_key_down(cx.listener(
            move |view, event: &KeyDownEvent, window, cx| {
                if team_activation_key(event) {
                    window.prevent_default();
                    view.set_team_enabled(&toggle_team_for_key, toggle_target, cx);
                }
            },
        ));
    }
    let enabled_control = if is_luxcavation {
        div()
    } else {
        div()
            .flex()
            .items_center()
            .gap_1()
            .text_size(px(11.))
            .text_color(palette_rgb(current_render_palette().muted_foreground))
            .child(text("启用", "Enabled").get(language))
            .child(enabled_switch)
    };

    let mut sinner_badges = div().w_full().min_w_0().flex().flex_wrap().gap_1();
    for (index, sinner) in team.sinners.iter().enumerate() {
        sinner_badges = sinner_badges.child(badge(
            format!("#{} {}", index + 1, app.teams.sinner_name(sinner)),
            BadgeTone::Neutral,
        ));
    }

    let scheme = normalized_scheme(&team.accessoryScheme);
    let header = div()
        .w_full()
        .min_w_0()
        .text_size(px(14.))
        .text_color(rgb(TEXT))
        .child(team.name.clone());

    let mut details = div()
        .w_full()
        .min_w_0()
        .flex()
        .items_center()
        .flex_wrap()
        .gap_1()
        .text_size(px(11.))
        .text_color(rgb(TEXT_MUTED));
    details = details.child(badge(
        purpose_label(team.purpose, language),
        BadgeTone::Neutral,
    ));
    if !is_luxcavation {
        details = details.child(scheme_badge(scheme, language));
    }
    details = details.child(if matches!(language, Language::ZhCn) {
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
    if !is_luxcavation && !team.enabled {
        details = details.child(badge(
            text("已停用", "Disabled").get(language),
            BadgeTone::Neutral,
        ));
    }

    let actions = div()
        .flex()
        .flex_none()
        .items_center()
        .gap_1()
        .child(enabled_control)
        .child(overwrite)
        .child(edit)
        .child(delete);

    let mut top_row = div().w_full().flex().items_center().gap_2();
    if let Some(number) = slot_number {
        top_row = top_row.child(badge(format!("#{number:02}"), BadgeTone::Accent));
    }
    top_row = top_row.child(div().flex_1()).child(actions);

    card(
        div()
            .w_full()
            .flex()
            .flex_col()
            .gap_1()
            .child(top_row)
            .child(header)
            .child(details)
            .child(sinner_badges),
    )
    .p_3()
    .opacity(if is_luxcavation || team.enabled {
        1.
    } else {
        0.6
    })
    .hover(|style| style.bg(palette_rgb(current_render_palette().secondary)))
}

fn empty_slot_card(
    _app: &mut AhabApp,
    cx: &mut Context<AhabApp>,
    number: u32,
    language: Language,
) -> Div {
    let purpose = TeamSlot::default_purpose(number);
    let mut create = div()
        .id(format!("new-team-slot-{number}"))
        .flex()
        .items_center()
        .justify_center()
        .gap_1()
        .h(px(30.))
        .px_3()
        .rounded_md()
        .tab_index(0)
        .cursor_pointer()
        .bg(palette_rgb(current_render_palette().brand_light))
        .text_size(px(11.))
        .text_color(palette_rgb(current_render_palette().brand))
        .focus_visible(|style| style.border_color(palette_rgb(current_render_palette().ring)))
        .child(icon(ICON_PLUS, 13., current_render_palette().brand))
        .child(text("新建", "Create").get(language))
        .on_click(cx.listener(move |view, _, _, cx| {
            view.open_new_team_for_slot(number, cx);
        }));
    create = create.on_key_down(cx.listener(move |view, event: &KeyDownEvent, window, cx| {
        if team_activation_key(event) {
            window.prevent_default();
            view.open_new_team_for_slot(number, cx);
        }
    }));

    let mut choose_preset = div()
        .id(format!("preset-team-slot-{number}"))
        .flex()
        .items_center()
        .justify_center()
        .gap_1()
        .h(px(30.))
        .px_3()
        .rounded_md()
        .tab_index(0)
        .cursor_pointer()
        .border_1()
        .border_color(palette_rgb(current_render_palette().brand))
        .text_size(px(11.))
        .text_color(palette_rgb(current_render_palette().brand))
        .focus_visible(|style| style.border_color(palette_rgb(current_render_palette().ring)))
        .child(icon(ICON_COPY, 13., current_render_palette().brand))
        .child(text("从预设编队选择", "Choose preset").get(language))
        .on_click(cx.listener(move |view, _, _, cx| {
            view.open_team_preset_picker_for_slot(number, cx);
        }));
    choose_preset =
        choose_preset.on_key_down(cx.listener(move |view, event: &KeyDownEvent, window, cx| {
            if team_activation_key(event) {
                window.prevent_default();
                view.open_team_preset_picker_for_slot(number, cx);
            }
        }));

    card(
        div()
            .flex()
            .items_center()
            .justify_between()
            .gap_3()
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .child(badge(format!("#{number:02}"), BadgeTone::Neutral))
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap_1()
                            .child(
                                div()
                                    .text_size(px(13.))
                                    .text_color(rgb(TEXT_MUTED))
                                    .child(text("未配置", "Not configured").get(language)),
                            )
                            .child(
                                div()
                                    .text_size(px(10.))
                                    .text_color(rgb(TEXT_MUTED))
                                    .child(purpose_label(purpose, language)),
                            ),
                    ),
            )
            .child(
                div()
                    .flex()
                    .flex_wrap()
                    .justify_end()
                    .gap_2()
                    .child(create)
                    .child(choose_preset),
            ),
    )
    .p_3()
    .opacity(0.75)
    .hover(|style| style.bg(palette_rgb(current_render_palette().secondary)))
}
