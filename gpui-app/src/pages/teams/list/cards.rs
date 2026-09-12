use super::*;

pub(super) fn team_card(
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
}

pub(super) fn empty_slot_card(
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
                                    .text_size(px(12.))
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
}
