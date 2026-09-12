use super::*;

pub(super) fn preset_target_label(target: &TeamPresetTarget, language: Language) -> String {
    match target {
        TeamPresetTarget::EmptySlot(number) => match language {
            Language::ZhCn => format!("空槽位 #{number:02}"),
            Language::EnUs => format!("Empty slot #{number:02}"),
        },
        TeamPresetTarget::Existing(team) => match language {
            Language::ZhCn => format!(
                "{}{}",
                team.name,
                team_number_from_id(&team.id)
                    .map(|number| format!("（槽位 #{number:02}）"))
                    .unwrap_or_default()
            ),
            Language::EnUs => format!(
                "{}{}",
                team.name,
                team_number_from_id(&team.id)
                    .map(|number| format!(" (slot #{number:02})"))
                    .unwrap_or_default()
            ),
        },
    }
}

pub(super) fn preset_card(
    preset: TeamPreset,
    app: &mut AhabApp,
    cx: &mut Context<AhabApp>,
    language: Language,
) -> Div {
    let preset_id = preset.presetId.clone();
    let preset_id_for_key = preset_id.clone();
    let palette = current_render_palette();
    let mut sinner_list = div().flex().flex_wrap().gap_1();
    for sinner in preset.team.sinners.iter().take(12) {
        sinner_list = sinner_list.child(badge(app.teams.sinner_name(sinner), BadgeTone::Neutral));
    }
    let mut control = card(
        div()
            .flex()
            .flex_col()
            .gap_2()
            .child(
                div()
                    .text_size(px(14.))
                    .font_weight(gpui::FontWeight::MEDIUM)
                    .text_color(palette_rgb(palette.foreground))
                    .child(preset.name.get(language).to_owned()),
            )
            .child(
                div()
                    .text_size(px(11.))
                    .text_color(palette_rgb(palette.muted_foreground))
                    .child(preset.description.get(language).to_owned()),
            )
            .child(
                div()
                    .text_size(px(10.))
                    .text_color(palette_rgb(palette.muted_foreground))
                    .child(preset.floorHint.get(language).to_owned()),
            )
            .child(
                div()
                    .text_size(px(10.))
                    .text_color(palette_rgb(palette.brand))
                    .child(preset.routeName.get(language).to_owned()),
            )
            .child(sinner_list),
    )
    .id(format!("team-preset-{}", preset_id))
    .w_full()
    .min_h(px(170.))
    .p_3()
    .bg(palette_rgb(palette.secondary))
    .border_1()
    .border_color(palette_rgb(palette.input))
    .tab_index(0)
    .cursor_pointer()
    .hover(|style| {
        style
            .bg(palette_rgb(current_render_palette().accent_surface))
            .border_color(palette_rgb(current_render_palette().brand))
    })
    .on_click(cx.listener(move |view, _, _, cx| {
        view.select_team_preset(&preset_id, cx);
    }));
    control = control.on_key_down(cx.listener(move |view, event: &KeyDownEvent, window, cx| {
        if team_activation_key(event) {
            window.prevent_default();
            view.select_team_preset(&preset_id_for_key, cx);
        }
    }));
    div().w_full().child(control)
}

pub(super) fn preset_picker_overlay(
    app: &mut AhabApp,
    cx: &mut Context<AhabApp>,
    language: Language,
) -> Div {
    let Some(picker) = app.teams.preset_picker.as_ref() else {
        return div();
    };
    let target_label = preset_target_label(&picker.target, language);
    let palette = current_render_palette();
    let presets = app.teams.presets.clone();
    let mut preset_grid = div().w_full().grid().grid_cols(2).gap_4();
    for preset in presets {
        preset_grid = preset_grid.child(preset_card(preset, app, cx, language));
    }
    let content: gpui::Stateful<Div> = if app.teams.presets.is_empty() {
        scroll_area_with_id(
            app,
            "team-preset-picker-scroll",
            empty_state(
                text("暂无内置预设", "No built-in presets").get(language),
                text(
                    "请确认后端已完成数据加载。",
                    "Wait for the backend catalog to load.",
                )
                .get(language),
            )
            .w_full()
            .min_h(px(220.)),
        )
    } else {
        scroll_area_with_id(app, "team-preset-picker-scroll", preset_grid)
    };
    let mut close = button(text("取消", "Cancel").get(language), ButtonVariant::Ghost)
        .id("team-preset-cancel")
        .h(px(32.))
        .px_3()
        .py_0()
        .on_click(cx.listener(|view, _, _, cx| {
            view.cancel_team_preset_flow(cx);
        }));
    close = close.on_key_down(cx.listener(|view, event: &KeyDownEvent, window, cx| {
        if team_activation_key(event) {
            window.prevent_default();
            view.cancel_team_preset_flow(cx);
        }
    }));
    let dialog = card(
        div()
            .flex()
            .flex_col()
            .gap_3()
            .child(
                div().flex().items_center().justify_between().gap_3().child(
                    div()
                        .flex()
                        .flex_col()
                        .gap_1()
                        .child(
                            div()
                                .text_size(px(16.))
                                .text_color(palette_rgb(palette.foreground))
                                .child(text("选择预设编队", "Choose a team preset").get(language)),
                        )
                        .child(
                            div()
                                .text_size(px(11.))
                                .text_color(palette_rgb(palette.muted_foreground))
                                .child(target_label),
                        ),
                ),
            )
            .child(content.flex_1().min_h_0())
            .child(div().flex().justify_end().child(close)),
    )
    .id("team-preset-dialog")
    .w(px(760.))
    .h(px(620.))
    .max_w_full()
    .max_h(relative(0.94))
    .min_h_0()
    .overflow_hidden()
    .p_5()
    .on_click(cx.listener(|_, _, _, cx| cx.stop_propagation()));

    let mut surface = div()
        .id("team-preset-overlay")
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
        .child(dialog)
        .on_click(cx.listener(|view, _, _, cx| {
            view.cancel_team_preset_flow(cx);
        }));
    surface = surface.capture_key_down(cx.listener(|view, event: &KeyDownEvent, window, cx| {
        if event.keystroke.key.eq_ignore_ascii_case("escape") {
            window.prevent_default();
            cx.stop_propagation();
            view.cancel_team_preset_flow(cx);
        }
    }));
    div()
        .absolute()
        .top_0()
        .left_0()
        .right_0()
        .bottom_0()
        .child(surface)
}

pub(super) fn preset_overwrite_overlay(
    app: &mut AhabApp,
    cx: &mut Context<AhabApp>,
    language: Language,
) -> Div {
    let Some(overwrite) = app.teams.preset_overwrite.as_ref() else {
        return div();
    };
    let target = overwrite.target.clone();
    let preset = overwrite.preset.clone();
    let palette = current_render_palette();
    let target_label = preset_target_label(
        &TeamPresetTarget::Existing(Box::new(target.clone())),
        language,
    );
    let mut cancel = button(text("取消", "Cancel").get(language), ButtonVariant::Ghost)
        .id("team-preset-overwrite-cancel")
        .h(px(32.))
        .px_3()
        .py_0()
        .on_click(cx.listener(|view, _, _, cx| {
            view.cancel_team_preset_flow(cx);
        }));
    cancel = cancel.on_key_down(cx.listener(|view, event: &KeyDownEvent, window, cx| {
        if team_activation_key(event) {
            window.prevent_default();
            view.cancel_team_preset_flow(cx);
        }
    }));
    let mut confirm = button(
        text("确认覆盖", "Confirm overwrite").get(language),
        ButtonVariant::Destructive,
    )
    .id("team-preset-overwrite-confirm")
    .h(px(32.))
    .px_3()
    .py_0()
    .on_click(cx.listener(|view, _, _, cx| {
        view.confirm_team_preset_overwrite(cx);
    }));
    confirm = confirm.on_key_down(cx.listener(|view, event: &KeyDownEvent, window, cx| {
        if team_activation_key(event) {
            window.prevent_default();
            view.confirm_team_preset_overwrite(cx);
        }
    }));
    let dialog = card(
        div()
            .flex()
            .flex_col()
            .gap_3()
            .child(
                div()
                    .text_size(px(16.))
                    .text_color(palette_rgb(palette.foreground))
                    .child(text("确认覆盖编队？", "Overwrite this team?").get(language)),
            )
            .child(
                div()
                    .text_size(px(12.))
                    .text_color(palette_rgb(palette.danger))
                    .child(format!(
                        "{}{}",
                        target_label,
                        match language {
                            Language::ZhCn => format!(" 将被预设“{}”完整覆盖。", preset.name.get(language)),
                            Language::EnUs => format!(" will be fully replaced by “{}”.", preset.name.get(language)),
                        }
                    )),
            )
            .child(
                div()
                    .text_size(px(11.))
                    .text_color(palette_rgb(palette.muted_foreground))
                    .child(text(
                        "名称、人格顺序、队伍码、镜牢配置和饰品路线都会替换；启用/停用状态保留。",
                        "The name, sinner order, team code, mirror settings, and gift route will be replaced; enabled state is preserved.",
                    ).get(language)),
            )
            .child(
                div()
                    .flex()
                    .justify_end()
                    .gap_2()
                    .child(cancel)
                    .child(confirm),
            ),
    )
    .id("team-preset-overwrite-dialog")
    .w(px(460.))
    .max_w_full()
    .p_5()
    .border_1()
    .border_color(palette_rgb(palette.danger))
    .on_click(cx.listener(|_, _, _, cx| cx.stop_propagation()));

    let mut surface = div()
        .id("team-preset-overwrite-overlay")
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
        .child(dialog)
        .on_click(cx.listener(|view, _, _, cx| {
            view.cancel_team_preset_flow(cx);
        }));
    surface = surface.capture_key_down(cx.listener(|view, event: &KeyDownEvent, window, cx| {
        if event.keystroke.key.eq_ignore_ascii_case("escape") {
            window.prevent_default();
            cx.stop_propagation();
            view.cancel_team_preset_flow(cx);
        }
    }));
    div()
        .absolute()
        .top_0()
        .left_0()
        .right_0()
        .bottom_0()
        .child(surface)
}
