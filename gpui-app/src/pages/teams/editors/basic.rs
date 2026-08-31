use super::*;

pub(crate) fn basic_editor(
    app: &mut AhabApp,
    cx: &mut Context<AhabApp>,
    team: &TeamDetail,
    config: &TeamMirrorConfig,
    language: Language,
) -> Div {
    let is_luxcavation = team.purpose == TeamPurpose::Luxcavation;
    let name_input = app.team_inputs.name.clone();
    let code_input = app.team_inputs.code.clone();

    let name_field = if let Some(input) = name_input {
        labeled_field(
            text("队伍名称", "Team name").get(language),
            div().child(input),
        )
    } else {
        labeled_field(
            text("队伍名称", "Team name").get(language),
            div().child(text("编辑器初始化中", "Initializing").get(language)),
        )
    };

    let purpose = team_select(
        app,
        cx,
        TeamSelectConfig {
            select: TeamSelect::Purpose,
            current: purpose_key(team.purpose).to_owned(),
            options: vec![
                (
                    "mirror".to_owned(),
                    purpose_label(TeamPurpose::Mirror, language).to_owned(),
                ),
                (
                    "luxcavation".to_owned(),
                    purpose_label(TeamPurpose::Luxcavation, language).to_owned(),
                ),
                (
                    "general".to_owned(),
                    purpose_label(TeamPurpose::General, language).to_owned(),
                ),
            ],
            id: "team-purpose".to_owned(),
            width: 180.,
            on_change: Rc::new(|teams, value| {
                let purpose = match value.as_str() {
                    "mirror" => TeamPurpose::Mirror,
                    "luxcavation" => TeamPurpose::Luxcavation,
                    "general" => TeamPurpose::General,
                    _ => return,
                };
                teams.set_editor_purpose(purpose);
            }),
        },
    );
    let purpose_field = labeled_field(text("用途", "Purpose").get(language), purpose);

    let mut systems = div().w_full().grid().grid_cols(5).gap_2();
    for (index, name) in SYSTEM_NAMES.iter().enumerate() {
        let selected = team.accessoryScheme == *name;
        let mut control =
            system_choice(index, selected, false, language).id(format!("team-system-{name}"));
        let system_name = (*name).to_owned();
        let system_name_for_key = system_name.clone();
        control = control
            .on_click(cx.listener(move |view, _, _, cx| {
                view.teams
                    .set_editor_scheme(system_name.clone(), index as u8);
                cx.notify();
            }))
            .on_key_down(cx.listener(move |view, event: &KeyDownEvent, window, cx| {
                if team_activation_key(event) {
                    window.prevent_default();
                    view.teams
                        .set_editor_scheme(system_name_for_key.clone(), index as u8);
                    cx.notify();
                }
            }));
        systems = systems.child(control);
    }
    let systems_field = if is_luxcavation {
        div().into_any_element()
    } else {
        field_block(text("饰品体系", "Gift System").get(language), systems).into_any_element()
    };

    let mut sinners = div().w_full().grid().grid_cols(6).gap_2();
    for sinner in app.teams.sinners.clone() {
        let selected = team.sinners.iter().position(|id| id == &sinner.id);
        let selected_state = selected.is_some();
        let id = sinner.id.clone();
        let path = sinner_path(&id);
        let key_id = id.clone();
        let palette = current_render_palette();
        let name_label = div()
            .id(format!("team-sinner-name-{id}"))
            .flex()
            .items_center()
            .justify_center()
            .w_full()
            .h(px(22.))
            .flex_none()
            .px_1()
            .rounded_sm()
            .bg(palette_rgb(if selected_state {
                palette.brand_light
            } else {
                palette.card
            }))
            .text_center()
            .text_size(px(11.))
            .font_weight(gpui::FontWeight::MEDIUM)
            .text_color(palette_rgb(if selected_state {
                palette.brand
            } else {
                palette.foreground
            }))
            .child(sinner.name);
        let mut control = div()
            .id(format!("team-sinner-{id}"))
            .w_full()
            .min_w_0()
            .h(px(124.))
            .flex()
            .flex_col()
            .items_center()
            .justify_between()
            .pt_2()
            .pb_2()
            .rounded_lg()
            .tab_index(0)
            .border_1()
            .border_color(palette_rgb(if selected_state {
                palette.brand
            } else {
                palette.border
            }))
            .bg(palette_rgb(if selected_state {
                palette.brand_light
            } else {
                palette.secondary
            }))
            .cursor_pointer()
            .focus_visible(|style| style.border_color(palette_rgb(current_render_palette().ring)))
            .on_click(cx.listener(move |view, _, _, cx| {
                view.teams.toggle_sinner(&id);
                cx.notify();
            }));
        control =
            control.on_key_down(cx.listener(move |view, event: &KeyDownEvent, window, cx| {
                if team_activation_key(event) {
                    window.prevent_default();
                    view.teams.toggle_sinner(&key_id);
                    cx.notify();
                }
            }));
        control = control.child(
            div()
                .relative()
                .w(px(56.))
                .h(px(56.))
                .flex_none()
                .child(img(path).size_full())
                .child(
                    selected
                        .map(|index| {
                            div()
                                .absolute()
                                .top_0()
                                .right_0()
                                .px_1()
                                .rounded_md()
                                .bg(palette_rgb(current_render_palette().brand))
                                .text_size(px(9.))
                                .text_color(palette_rgb(current_render_palette().brand_foreground))
                                .child(format!("#{}", index + 1))
                        })
                        .unwrap_or_else(div),
                ),
        );
        // Keep the name outside the avatar container so both selected and
        // unselected sinners use the same readable caption below the image.
        control = control.child(name_label);
        sinners = sinners.child(control);
    }

    let code_field = if config.use_team_code {
        code_input
            .map(|input| {
                labeled_field(
                    text("编队码", "Team code").get(language),
                    div().child(input),
                )
            })
            .unwrap_or_else(|| {
                labeled_field(
                    text("编队码", "Team code").get(language),
                    div().child(text("编辑器初始化中", "Initializing").get(language)),
                )
            })
    } else {
        div()
    };

    let team_code_switch = mirror_switch(
        app,
        cx,
        MirrorBool::UseTeamCode,
        config.use_team_code,
        "basic-use-team-code",
    );
    let fixed_switch = mirror_switch(
        app,
        cx,
        MirrorBool::FixedTeamUse,
        config.fixed_team_use,
        "basic-fixed-team",
    );
    let enabled_switch = switch(team.enabled)
        .id("team-enabled")
        .on_click(cx.listener(|view, _, _, cx| {
            let enabled = view
                .teams
                .editor
                .as_ref()
                .map(|editor| !editor.team.enabled)
                .unwrap_or(true);
            view.teams.set_editor_enabled(enabled);
            cx.notify();
        }))
        .on_key_down(cx.listener(|view, event: &KeyDownEvent, window, cx| {
            if team_activation_key(event) {
                window.prevent_default();
                let enabled = view
                    .teams
                    .editor
                    .as_ref()
                    .map(|editor| !editor.team.enabled)
                    .unwrap_or(true);
                view.teams.set_editor_enabled(enabled);
                cx.notify();
            }
        }));

    let mut clear_sinners = button(
        text("清空人格", "Clear Sinners").get(language),
        ButtonVariant::Ghost,
    )
    .id("team-clear-sinners");
    clear_sinners = clear_sinners.on_click(cx.listener(|view, _, _, cx| {
        view.teams.clear_sinners();
        cx.notify();
    }));

    let team_code_card = editor_card(
        div()
            .flex()
            .flex_col()
            .gap_2()
            .child(control_row(
                text("启用编队码", "Use Team Code").get(language),
                team_code_switch,
            ))
            .child(
                div().text_size(px(10.)).text_color(rgb(TEXT_MUTED)).child(
                    text(
                        "启用后保存编队码，运行时由后端解析。",
                        "Save the team code and let the backend parse it at runtime.",
                    )
                    .get(language),
                ),
            )
            .child(code_field),
    );

    let fixed_card = editor_card(
        div()
            .flex()
            .flex_col()
            .gap_2()
            .child(control_row(
                text("固定队伍用途", "Fixed Team Purpose").get(language),
                fixed_switch,
            ))
            .child(if config.fixed_team_use {
                team_select(
                    app,
                    cx,
                    TeamSelectConfig {
                        select: TeamSelect::FixedTeamUse,
                        current: config.fixed_team_use_select.to_string(),
                        options: vec![
                            ("0".to_owned(), fixed_team_use_label(0, language).to_owned()),
                            ("1".to_owned(), fixed_team_use_label(1, language).to_owned()),
                            ("2".to_owned(), fixed_team_use_label(2, language).to_owned()),
                        ],
                        id: "basic-fixed-team-range".to_owned(),
                        width: 180.,
                        on_change: Rc::new(|teams, value| {
                            if let Ok(value) = value.parse::<u8>() {
                                teams.set_mirror_u8(MirrorU8::FixedTeamUseSelect, value);
                            }
                        }),
                    },
                )
                .into_any_element()
            } else {
                div().into_any_element()
            }),
    );

    let mirror_basic_cards = if is_luxcavation {
        div().into_any_element()
    } else {
        div()
            .flex()
            .flex_col()
            .gap_3()
            .child(team_code_card)
            .child(fixed_card)
            .into_any_element()
    };

    let enabled_field = if is_luxcavation {
        div().into_any_element()
    } else {
        editor_card(control_row(
            text("队伍启用", "Team Enabled").get(language),
            enabled_switch,
        ))
        .into_any_element()
    };

    div()
        .flex()
        .flex_col()
        .gap_4()
        .child(
            div()
                .flex()
                .flex_wrap()
                .gap_3()
                .child(name_field.flex_basis(px(220.)).flex_grow(1.))
                .child(purpose_field.flex_basis(px(140.)).flex_grow(1.)),
        )
        .child(systems_field)
        .child(field_block(
            if matches!(language, Language::ZhCn) {
                format!("人格顺序（{} / 12）", team.sinners.len())
            } else {
                format!("Sinner order ({} / 12)", team.sinners.len())
            },
            div().flex().flex_col().gap_2().child(sinners).child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .gap_2()
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .text_size(px(10.))
                            .text_color(rgb(TEXT_MUTED))
                            .child(
                                text(
                                    "按点击顺序分配 #1~#12；再次点击可移除。",
                                    "Click to assign slots #1-#12; click again to remove.",
                                )
                                .get(language),
                            ),
                    )
                    .child(clear_sinners),
            ),
        ))
        .child(mirror_basic_cards)
        .child(enabled_field)
}
