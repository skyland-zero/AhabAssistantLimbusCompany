use super::*;

use gpui::WeakEntity;

/// The after-completion editor body, rendered from a live handle on the app.
///
/// Same constraint as `DailyDetailsView`: a `Root` dialog's builder runs inside
/// `AhabApp::render`, so it cannot read the app - and this editor has to show
/// the draft changing as the switches are toggled, which a snapshot taken at
/// open time cannot do. As a child view it is rendered after that borrow has
/// ended, so it can read the app and repaint itself.
pub(crate) struct AfterCompletionView {
    root: WeakEntity<AhabApp>,
    /// Never dropped: it is what repaints the switches once the app notifies.
    _app_events: gpui::Subscription,
}

impl AfterCompletionView {
    pub(crate) fn new(root: gpui::Entity<AhabApp>, cx: &mut Context<Self>) -> Self {
        let app_events = cx.observe(&root, |_, _, cx| cx.notify());
        Self {
            root: root.downgrade(),
            _app_events: app_events,
        }
    }
}

impl Render for AfterCompletionView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let Some(root) = self.root.upgrade() else {
            return div().into_any_element();
        };

        let (config, language, busy, power_open) = {
            let app = root.read(cx);
            let config = app
                .home
                .after_completion_draft
                .clone()
                .unwrap_or_else(|| app.home.tasks.afterCompletion.clone());
            (
                config,
                app.state.settings.language,
                app.home.is_busy(),
                app.home.is_select_open(HomeSelect::AfterPowerAction),
            )
        };

        after_completion_body(&self.root, &config, language, busy, power_open).into_any_element()
    }
}

/// The editor's fields and actions, without the dialog chrome: `Root` supplies
/// the title, the close button and the Esc handling.
fn after_completion_body(
    root: &WeakEntity<AhabApp>,
    config: &crate::model::AfterCompletionConfig,
    language: Language,
    busy: bool,
    power_open: bool,
) -> Div {
    let mut exits = div().flex().flex_col().gap_2();
    for action in [
        AfterExitAction::ExitGame,
        AfterExitAction::ExitEmulator,
        AfterExitAction::ExitAalc,
    ] {
        let control = task_option_switch_for_view(
            "",
            config.actions.contains(&action),
            match action {
                AfterExitAction::ExitGame => "after-exit-game",
                AfterExitAction::ExitEmulator => "after-exit-emulator",
                AfterExitAction::ExitAalc => "after-exit-aalc",
            },
            busy,
            root,
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

    let power = home_select_for_view(
        root,
        power_open,
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

    let mut apply_once = button(
        "after-completion-apply-once",
        text("仅本次生效", "Apply Once").get(language),
        ButtonVariant::Outline,
    );
    let mut save_default = button(
        "after-completion-save-default",
        text("保存为默认", "Save as Default").get(language),
        ButtonVariant::Default,
    );
    if busy {
        apply_once = apply_once.opacity(0.45).cursor_not_allowed();
        save_default = save_default.opacity(0.45).cursor_not_allowed();
    } else {
        let apply_host = root.clone();
        apply_once = apply_once.on_click(move |_, window, cx| {
            if let Some(root) = apply_host.upgrade() {
                let window = &mut *window;
                root.update(cx, move |view, cx| {
                    view.apply_after_completion(false, window, cx);
                });
            }
        });
        let save_host = root.clone();
        save_default = save_default.on_click(move |_, window, cx| {
            if let Some(root) = save_host.upgrade() {
                let window = &mut *window;
                root.update(cx, move |view, cx| {
                    view.apply_after_completion(true, window, cx);
                });
            }
        });
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

    div()
        .flex()
        .flex_col()
        .gap(px(15.))
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
        )
}
