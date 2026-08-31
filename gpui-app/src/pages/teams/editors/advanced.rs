use super::*;

fn observe_gift_label(gift: &str, language: Language) -> String {
    match gift {
        crate::model::SPIDERWEB_ENTANGLED_IN_RED_GIFT_ID => {
            text("赤红纠缠的蜘蛛巢", "Spiderweb Entangled in Red")
                .get(language)
                .to_owned()
        }
        _ => gift.to_owned(),
    }
}

pub(crate) fn advanced_editor(
    app: &mut AhabApp,
    cx: &mut Context<AhabApp>,
    config: &TeamMirrorConfig,
    language: Language,
) -> Div {
    let observe_input = app.team_inputs.observe.clone();
    let json_input = app.team_inputs.json.clone();
    let observe_switch = mirror_switch(
        app,
        cx,
        MirrorBool::ObserveEgoGift,
        config.observe_ego_gift,
        "advanced-observe-ego",
    );
    let custom_weight = mirror_switch(
        app,
        cx,
        MirrorBool::UseCustomThemeWeight,
        config.use_custom_theme_pack_weight,
        "advanced-theme-weight",
    );

    let mut gifts = div().flex().flex_wrap().gap_1();
    for gift in &config.observe_ego_gift_selected {
        let gift_for_remove = gift.clone();
        let remove = div()
            .id(format!("remove-gift-{gift}"))
            .flex()
            .items_center()
            .justify_center()
            .size(px(22.))
            .rounded_md()
            .cursor_pointer()
            .child(icon(
                ICON_CLOSE,
                12.,
                current_render_palette().muted_foreground,
            ))
            .on_click(cx.listener(move |view, _, _, cx| {
                view.teams.remove_observe_gift(&gift_for_remove);
                cx.notify();
            }));
        gifts = gifts.child(
            div()
                .flex()
                .items_center()
                .gap_1()
                .px_2()
                .py_1()
                .rounded_md()
                .bg(palette_rgb(current_render_palette().secondary))
                .text_size(px(11.))
                .text_color(palette_rgb(current_render_palette().foreground))
                .child(observe_gift_label(gift, language))
                .child(remove),
        );
    }

    let has_spiderweb = config
        .observe_ego_gift_selected
        .iter()
        .any(|gift| gift == crate::model::SPIDERWEB_ENTANGLED_IN_RED_GIFT_ID);
    let mut spiderweb_preset = editor_choice_button(
        text("赤红纠缠的蜘蛛巢", "Spiderweb Entangled in Red").get(language),
        has_spiderweb,
    )
    .id("advanced-spiderweb-gift")
    .h(px(30.))
    .px_2()
    .py_0()
    .on_click(cx.listener(|view, _, _, cx| {
        view.teams.add_spiderweb_entangled_in_red();
        cx.notify();
    }));
    spiderweb_preset =
        spiderweb_preset.on_key_down(cx.listener(|view, event: &KeyDownEvent, window, cx| {
            if team_activation_key(event) {
                window.prevent_default();
                view.teams.add_spiderweb_entangled_in_red();
                cx.notify();
            }
        }));

    let mut add_observe = button(text("添加", "Add").get(language), ButtonVariant::Default)
        .id("advanced-add-observe")
        .h(px(34.))
        .px_3()
        .py_0()
        .on_click(cx.listener(|view, _, _, cx| view.add_team_observe_gift(cx)));
    add_observe = add_observe.on_key_down(cx.listener(|view, event: &KeyDownEvent, window, cx| {
        if team_activation_key(event) {
            window.prevent_default();
            view.add_team_observe_gift(cx);
        }
    }));
    let observe_field = observe_input
        .map(|input| {
            div()
                .flex_1()
                .min_w_0()
                .child(input)
                .on_key_down(cx.listener(|view, event: &KeyDownEvent, window, cx| {
                    if event.keystroke.key.eq_ignore_ascii_case("enter") {
                        window.prevent_default();
                        cx.stop_propagation();
                        view.add_team_observe_gift(cx);
                    }
                }))
                .into_any_element()
        })
        .unwrap_or_else(|| div().into_any_element());
    let observe_content = if config.observe_ego_gift {
        div()
            .flex()
            .flex_col()
            .gap_2()
            .pt_1()
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .child(observe_field)
                    .child(add_observe),
            )
            .child(gifts)
    } else {
        div()
    };

    let mut import_toggle = div()
        .id("advanced-json-toggle")
        .flex()
        .items_center()
        .justify_center()
        .gap_1()
        .h(px(28.))
        .px_2()
        .rounded_md()
        .tab_index(0)
        .border_1()
        .border_color(palette_rgb(current_render_palette().input))
        .cursor_pointer()
        .focus_visible(|style| style.border_color(palette_rgb(current_render_palette().ring)))
        .hover(|style| style.bg(palette_rgb(current_render_palette().accent_surface)))
        .text_size(px(11.))
        .text_color(palette_rgb(current_render_palette().brand))
        .child(icon(ICON_PASTE, 14., current_render_palette().brand))
        .child(text("粘贴 / 导入 JSON", "Paste / Import JSON").get(language));
    import_toggle = import_toggle
        .on_click(cx.listener(|view, _, _, cx| {
            if let Some(editor) = view.teams.editor.as_mut() {
                editor.json_import_open = !editor.json_import_open;
            }
            cx.notify();
        }))
        .on_key_down(cx.listener(|view, event: &KeyDownEvent, window, cx| {
            if team_activation_key(event) {
                window.prevent_default();
                if let Some(editor) = view.teams.editor.as_mut() {
                    editor.json_import_open = !editor.json_import_open;
                }
                cx.notify();
            }
        }));
    let json_panel = if app
        .teams
        .editor
        .as_ref()
        .map(|editor| editor.json_import_open)
        .unwrap_or(false)
    {
        div()
            .flex()
            .flex_col()
            .gap_2()
            .child(
                json_input
                    .map(|input| input.into_any_element())
                    .unwrap_or_else(|| div().into_any_element()),
            )
            .child(
                div()
                    .flex()
                    .justify_end()
                    .gap_2()
                    .child({
                        let mut close =
                            button(text("关闭", "Close").get(language), ButtonVariant::Ghost)
                                .id("advanced-json-close")
                                .h(px(28.))
                                .px_3()
                                .py_0()
                                .on_click(cx.listener(|view, _, _, cx| {
                                    if let Some(editor) = view.teams.editor.as_mut() {
                                        editor.json_import_open = false;
                                    }
                                    cx.notify();
                                }));
                        close = close.on_key_down(cx.listener(
                            |view, event: &KeyDownEvent, window, cx| {
                                if team_activation_key(event) {
                                    window.prevent_default();
                                    if let Some(editor) = view.teams.editor.as_mut() {
                                        editor.json_import_open = false;
                                    }
                                    cx.notify();
                                }
                            },
                        ));
                        close
                    })
                    .child({
                        let mut import = button(
                            text("校验并覆盖", "Validate & Apply").get(language),
                            ButtonVariant::Default,
                        )
                        .id("advanced-json-import")
                        .h(px(28.))
                        .px_3()
                        .py_0()
                        .on_click(cx.listener(|view, _, _, cx| view.import_team_json(cx)));
                        import = import.on_key_down(cx.listener(
                            |view, event: &KeyDownEvent, window, cx| {
                                if team_activation_key(event) {
                                    window.prevent_default();
                                    view.import_team_json(cx);
                                }
                            },
                        ));
                        import
                    }),
            )
    } else {
        div()
    };

    div()
        .flex()
        .flex_col()
        .gap_4()
        .child(
            editor_card(
                div()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .child(control_row(
                        text("开局礼物搜索", "Starting Gift Search").get(language),
                        observe_switch,
                    ))
                    .child(
                        div().text_size(px(10.)).text_color(rgb(TEXT_MUTED)).child(
                            text(
                                "推荐直接选择小指良专属礼物；高级配置可填写 system_level_row_col。",
                                "Use the Ryoshu preset, or enter system_level_row_col for advanced configuration.",
                            )
                            .get(language),
                        ),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_2()
                            .flex_wrap()
                            .child(
                                div()
                                    .text_size(px(11.))
                                    .text_color(rgb(TEXT_MUTED))
                                    .child(text("快速选择", "Quick preset").get(language)),
                            )
                            .child(spiderweb_preset),
                    )
                    .child(editor_section_title(
                        text("高级坐标配置", "Advanced coordinate configuration").get(language),
                    ))
                    .child(observe_content),
            )
            .flex_none(),
        )
        .child(
            editor_card(
                div()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .child(control_row(
                        text("使用队伍专属主题包权重", "Use Custom Theme Pack Weight")
                            .get(language),
                        custom_weight,
                    ))
                    .child(
                        div().text_size(px(10.)).text_color(rgb(TEXT_MUTED)).child(
                            text(
                                "保存后由镜牢执行器读取该队伍的主题包权重。",
                                "The Mirror executor reads this team's pack weights after Save.",
                            )
                            .get(language),
                        ),
                    ),
            )
            .flex_none(),
        )
        .child(
            editor_card(
                div()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .gap_2()
                            .child(editor_section_title(
                                text("配置导入导出", "Configuration Import / Export").get(language),
                            ))
                            .child(import_toggle),
                    )
                    .child(json_panel),
            )
            .flex_none(),
        )
}
