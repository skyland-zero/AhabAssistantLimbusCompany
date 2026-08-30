use super::*;

pub(crate) fn render_overlay(app: &mut AhabApp, cx: &mut Context<AhabApp>) -> Div {
    let language = app.state.settings.language;
    let Some(editor) = app.teams.editor.as_ref() else {
        if app.teams.delete_target.is_none() {
            return div();
        }

        let mut surface = div()
            .id("team-delete-overlay")
            .relative()
            .size_full()
            .flex()
            .items_center()
            .justify_center()
            .p_4()
            .bg(rgba(0x00000080));
        surface = surface.on_click(cx.listener(|view, _, _, cx| {
            view.teams.cancel_delete();
            cx.notify();
        }));
        surface =
            surface.capture_key_down(cx.listener(|view, event: &KeyDownEvent, window, cx| {
                if event.keystroke.key.eq_ignore_ascii_case("escape") {
                    window.prevent_default();
                    cx.stop_propagation();
                    view.teams.cancel_delete();
                    cx.notify();
                }
            }));
        return div()
            .absolute()
            .top_0()
            .left_0()
            .right_0()
            .bottom_0()
            .child(surface.child(delete_confirmation(app, cx, language)));
    };

    let team = editor.team.clone();
    let tab = if editor.tab.is_available(team.purpose) {
        editor.tab
    } else {
        TeamEditorTab::Basic
    };
    let config = editor.mirror_config();
    let feedback = app.teams.feedback.clone();
    let starlight_cost = app.teams.starlight_cost();

    let mut tabs = div()
        .flex()
        .items_center()
        .gap_1()
        .flex_wrap()
        .p(px(2.))
        .rounded_md()
        .bg(palette_rgb(current_render_palette().muted));
    for candidate in TeamEditorTab::ALL {
        if !candidate.is_available(team.purpose) {
            continue;
        }
        let active = candidate == tab;
        let mut control = div()
            .id(format!("team-editor-tab-{candidate:?}"))
            .flex()
            .items_center()
            .justify_center()
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
        control = control.child(editor_tab_label(candidate, language));
        if candidate == TeamEditorTab::Starlight && starlight_cost > 0 {
            control = control.child(
                div()
                    .px_1()
                    .rounded_md()
                    .bg(palette_rgb(current_render_palette().brand_light))
                    .text_size(px(10.))
                    .text_color(palette_rgb(current_render_palette().brand))
                    .child(starlight_cost.to_string()),
            );
        }
        control = control.on_click(cx.listener(move |view, _, _, cx| {
            view.teams.set_editor_tab(candidate);
            cx.notify();
        }));
        control =
            control.on_key_down(cx.listener(move |view, event: &KeyDownEvent, window, cx| {
                if team_activation_key(event) {
                    window.prevent_default();
                    view.teams.set_editor_tab(candidate);
                    cx.notify();
                }
            }));
        tabs = tabs.child(control);
    }

    let mut copy = div()
        .id("team-copy-json")
        .flex()
        .items_center()
        .justify_center()
        .gap_1()
        .h(px(28.))
        .px_2()
        .rounded_md()
        .tab_index(0)
        .border_1()
        .border_color(rgb(BORDER))
        .cursor_pointer()
        .focus_visible(|style| style.border_color(palette_rgb(current_render_palette().ring)))
        .text_size(px(11.))
        .text_color(rgb(crate::app::ACCENT))
        .child(icon(ICON_COPY, 14., current_render_palette().brand))
        .child(text("复制 JSON", "Copy JSON").get(language))
        .on_click(cx.listener(|view, _, _, cx| view.copy_team_json(cx)));
    copy = copy.on_key_down(cx.listener(|view, event: &KeyDownEvent, window, cx| {
        if team_activation_key(event) {
            window.prevent_default();
            view.copy_team_json(cx);
        }
    }));

    let mut close = button(text("取消", "Cancel").get(language), ButtonVariant::Ghost)
        .id("team-editor-cancel")
        .h(px(32.))
        .px_3()
        .py_0()
        .on_click(cx.listener(|view, _, _, cx| view.close_team_editor(cx)));
    close = close.on_key_down(cx.listener(|view, event: &KeyDownEvent, window, cx| {
        if team_activation_key(event) {
            window.prevent_default();
            view.close_team_editor(cx);
        }
    }));
    let current_name = app
        .team_inputs
        .name
        .as_ref()
        .map(|input| input.read(cx).text())
        .unwrap_or_else(|| team.name.clone());
    let can_save = !current_name.trim().is_empty() && !app.teams.saving;
    let mut save = button(
        text("保存队伍", "Save Team").get(language),
        ButtonVariant::Default,
    )
    .id("team-editor-save")
    .h(px(32.))
    .px_3()
    .py_0();
    if can_save {
        save = save
            .on_click(cx.listener(|view, _, _, cx| view.save_team_editor(cx)))
            .on_key_down(cx.listener(|view, event: &KeyDownEvent, window, cx| {
                if team_activation_key(event) {
                    window.prevent_default();
                    view.save_team_editor(cx);
                }
            }));
    } else {
        save = save.opacity(0.45).cursor_default();
    }

    let content = match tab {
        TeamEditorTab::Basic => editors::basic_editor(app, cx, &team, &config, language),
        TeamEditorTab::Shop => editors::shop_editor(app, cx, &config, language),
        TeamEditorTab::Combat => editors::combat_editor(app, cx, &config, language),
        TeamEditorTab::Starlight => editors::starlight_editor(app, cx, &config, language),
        TeamEditorTab::Advanced => editors::advanced_editor(app, cx, &config, language),
    };

    let dialog_body =
        div()
            .id("team-editor-dialog")
            .w(px(680.))
            .h(px(600.))
            .max_w_full()
            .max_h(relative(0.96))
            .min_h_0()
            .overflow_hidden()
            .flex()
            .flex_col()
            .rounded_lg()
            .border_1()
            .border_color(rgb(BORDER))
            .bg(rgb(SURFACE))
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .gap_3()
                    .px_6()
                    .py_3()
                    .border_b_1()
                    .border_color(rgb(BORDER))
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap_1()
                            .child(div().text_size(px(16.)).text_color(rgb(TEXT)).child(
                                if team.id.is_empty() {
                                    text("新建队伍", "New Team").get(language)
                                } else {
                                    text("编辑队伍", "Edit Team").get(language)
                                },
                            ))
                            .child(
                                div().text_size(px(10.)).text_color(rgb(TEXT_MUTED)).child(
                                    text(
                                        "保存前所有修改只存在于当前编辑器",
                                        "Changes stay in this editor until Save",
                                    )
                                    .get(language),
                                ),
                            ),
                    )
                    .child(copy),
            )
            .child(
                div()
                    .flex_none()
                    .px_6()
                    .py_2()
                    .border_b_1()
                    .border_color(rgb(BORDER))
                    .child(tabs),
            )
            .child(
                scroll_area_with_id(app, "team-editor-scroll", content)
                    .flex_1()
                    .min_h_0()
                    .px_6()
                    .py_4(),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .gap_3()
                    .flex_none()
                    .px_6()
                    .py_3()
                    .border_t_1()
                    .border_color(rgb(BORDER))
                    .bg(palette_rgb(current_render_palette().card))
                    .child(
                        feedback
                            .map(|message| {
                                div()
                                    .flex_1()
                                    .min_w_0()
                                    .text_size(px(11.))
                                    .text_color(palette_rgb(current_render_palette().warning))
                                    .child(localized_feedback(&message, language))
                            })
                            .unwrap_or_else(|| {
                                div()
                                .flex_1()
                                .min_w_0()
                                .text_size(px(11.))
                                .text_color(rgb(TEXT_MUTED))
                                .child(text(
                                    "支持中文输入、剪贴板和 JSON 导入",
                                    "Chinese input, clipboard and JSON import are supported",
                                )
                                .get(language))
                            }),
                    )
                    .child(div().flex().flex_none().gap_2().child(close).child(save)),
            )
            .on_click(cx.listener(|_, _, _, cx| cx.stop_propagation()));

    let mut surface = div()
        .id("team-editor-overlay")
        .relative()
        .size_full()
        .flex()
        .items_center()
        .justify_center()
        .p_4()
        .bg(rgba(0x00000080));
    surface = surface.on_click(cx.listener(|view, _, _, cx| {
        if view.teams.delete_target.is_some() {
            view.teams.cancel_delete();
            cx.notify();
        } else {
            view.close_team_editor(cx);
        }
    }));
    surface = surface.capture_key_down(cx.listener(|view, event: &KeyDownEvent, window, cx| {
        if event.keystroke.key.eq_ignore_ascii_case("escape") {
            window.prevent_default();
            cx.stop_propagation();
            if view.teams.delete_target.is_some() {
                view.teams.cancel_delete();
            } else {
                view.close_team_editor(cx);
            }
            cx.notify();
        }
    }));
    surface = surface.child(dialog_body);

    if app.teams.delete_target.is_some() {
        let delete_layer = div()
            .id("team-delete-layer")
            .absolute()
            .top_0()
            .left_0()
            .right_0()
            .bottom_0()
            .flex()
            .items_center()
            .justify_center()
            .p_4()
            .bg(rgba(0x00000080))
            .on_click(cx.listener(|_, _, _, cx| cx.stop_propagation()));
        surface = surface.child(delete_layer.child(delete_confirmation(app, cx, language)));
    }

    div()
        .absolute()
        .top_0()
        .left_0()
        .right_0()
        .bottom_0()
        .child(surface)
}
