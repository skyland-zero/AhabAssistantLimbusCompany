use super::*;

pub(super) fn after_completion_editor(
    app: &mut AhabApp,
    cx: &mut Context<AhabApp>,
    busy: bool,
) -> gpui::AnyElement {
    if !app.home.after_completion_open {
        return div().into_any_element();
    }
    let config = app
        .home
        .after_completion_draft
        .clone()
        .unwrap_or_else(|| app.home.tasks.afterCompletion.clone());
    let language = app.state.settings.language;
    let mut exits = div().flex().flex_col().gap_2();
    for action in [
        AfterExitAction::ExitGame,
        AfterExitAction::ExitEmulator,
        AfterExitAction::ExitAalc,
    ] {
        let control = task_option_switch(
            "",
            config.actions.contains(&action),
            match action {
                AfterExitAction::ExitGame => "after-exit-game",
                AfterExitAction::ExitEmulator => "after-exit-emulator",
                AfterExitAction::ExitAalc => "after-exit-aalc",
            },
            busy,
            cx,
            move |home| home.toggle_after_completion_draft(action),
        );
        exits = exits.child(
            div()
                .flex()
                .items_center()
                .justify_between()
                .gap_3()
                .py_0()
                .child(
                    div()
                        .text_size(px(12.))
                        .text_color(rgb(TEXT))
                        .child(super::completion::after_exit_label(action, language)),
                )
                .child(control),
        );
    }

    let power = home_select(
        app,
        cx,
        HomeSelectConfig {
            select: HomeSelect::AfterPowerAction,
            current: super::completion::after_power_key(config.powerAction).to_owned(),
            options: vec![
                (
                    "none".to_owned(),
                    super::completion::after_power_label(AfterPowerAction::None, language)
                        .to_owned(),
                ),
                (
                    "sleep".to_owned(),
                    super::completion::after_power_label(AfterPowerAction::Sleep, language)
                        .to_owned(),
                ),
                (
                    "hibernate".to_owned(),
                    super::completion::after_power_label(AfterPowerAction::Hibernate, language)
                        .to_owned(),
                ),
                (
                    "lock".to_owned(),
                    super::completion::after_power_label(AfterPowerAction::Lock, language)
                        .to_owned(),
                ),
                (
                    "shutdown".to_owned(),
                    super::completion::after_power_label(AfterPowerAction::Shutdown, language)
                        .to_owned(),
                ),
            ],
            id: "after-power-action".to_owned(),
            width: 464.,
            disabled: busy,
            on_change: Rc::new(|home, value| {
                if let Some(action) = super::completion::parse_after_power_action(&value) {
                    home.set_after_completion_draft_power(action);
                }
            }),
        },
    );

    let mut close = button("", ButtonVariant::Ghost)
        .id("after-completion-close")
        .w(px(32.))
        .h(px(32.))
        .px_0()
        .child(action_icon(ICON_X, 16., TEXT_MUTED));
    close = close.on_click(cx.listener(|view, _, _, cx| {
        view.home.set_after_completion_open(false);
        cx.notify();
    }));
    close = close.on_key_down(cx.listener(|view, event: &KeyDownEvent, window, cx| {
        if is_activation_key(event) {
            window.prevent_default();
            view.home.set_after_completion_open(false);
            cx.notify();
        }
    }));

    let mut apply_once = button(
        text("仅本次生效", "Apply Once").get(language),
        ButtonVariant::Outline,
    )
    .id("after-completion-apply-once");
    let mut save_default = button(
        text("保存为默认", "Save as Default").get(language),
        ButtonVariant::Default,
    )
    .id("after-completion-save-default");
    if !busy {
        apply_once = apply_once.on_click(cx.listener(|view, _, _, cx| {
            view.home.apply_after_completion(false);
            cx.notify();
        }));
        apply_once =
            apply_once.on_key_down(cx.listener(|view, event: &KeyDownEvent, window, cx| {
                if is_activation_key(event) {
                    window.prevent_default();
                    view.home.apply_after_completion(false);
                    cx.notify();
                }
            }));
        save_default = save_default.on_click(cx.listener(|view, _, _, cx| {
            view.home.apply_after_completion(true);
            cx.notify();
        }));
        save_default =
            save_default.on_key_down(cx.listener(|view, event: &KeyDownEvent, window, cx| {
                if is_activation_key(event) {
                    window.prevent_default();
                    view.home.apply_after_completion(true);
                    cx.notify();
                }
            }));
    } else {
        apply_once = apply_once.opacity(0.45).cursor_not_allowed();
        save_default = save_default.opacity(0.45).cursor_not_allowed();
    }

    let exit_group = div()
        .flex()
        .flex_col()
        .gap_2()
        .rounded_lg()
        .border_1()
        .border_color(palette_rgb(current_render_palette().input))
        .bg(palette_rgb(current_render_palette().card))
        .p_3()
        .child(exits);
    let exits_section = div()
        .flex()
        .flex_col()
        .gap_2()
        .child(
            div()
                .text_size(px(11.))
                .text_color(rgb(TEXT_MUTED))
                .child(text("退出动作（可多选）", "Exit Actions (Multi-select)").get(language)),
        )
        .child(exit_group);
    let power_section = div()
        .flex()
        .flex_col()
        .gap_2()
        .child(
            div()
                .text_size(px(11.))
                .text_color(rgb(TEXT_MUTED))
                .child(text("最终电源动作", "Power Action").get(language)),
        )
        .child(power);
    let dialog = card(
        div()
            .flex()
            .flex_col()
            .gap(px(15.))
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(
                        div()
                            .text_size(px(16.))
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(rgb(TEXT))
                            .child(text("结束后操作", "After Completion Actions").get(language)),
                    )
                    .child(close),
            )
            .child(exits_section)
            .child(power_section)
            .child(
                div()
                    .flex()
                    .justify_end()
                    .gap_2()
                    .pt_1()
                    .child(apply_once)
                    .child(save_default),
            ),
    )
    .p_6()
    .w(px(512.0))
    .max_w_full()
    .id("after-completion-dialog")
    .on_click(cx.listener(|_, _, _, cx| cx.stop_propagation()));

    let mut overlay = div()
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
        .id("after-completion-overlay")
        .child(dialog)
        .on_click(cx.listener(|view, _, _, cx| {
            view.home.set_after_completion_open(false);
            cx.notify();
        }));
    overlay = overlay.capture_key_down(cx.listener(|view, event: &KeyDownEvent, window, cx| {
        if event.keystroke.key.eq_ignore_ascii_case("escape") {
            window.prevent_default();
            cx.stop_propagation();
            view.home.set_after_completion_open(false);
            cx.notify();
        }
    }));
    overlay.into_any_element()
}
