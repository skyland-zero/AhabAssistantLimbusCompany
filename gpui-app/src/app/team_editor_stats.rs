use gpui::{Context, ParentElement as _, Styled as _, Window, div};
use gpui_component::Root;

use super::AhabApp;

impl AhabApp {
    pub fn refresh_team_stats(&mut self, cx: &mut Context<Self>) {
        if let Err(error) = self.teams.refresh_team_stats() {
            self.teams.feedback = Some(error);
        }
        cx.notify();
    }

    /// Ask to clear the team's history and open the confirmation as a Root
    /// dialog, matching the delete confirmation's flow.
    pub fn open_clear_stats_confirmation(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if !self.teams.request_clear_team_stats() {
            return;
        }
        let language = self.state.settings.language;
        let app = cx.entity().downgrade();
        Root::update(window, cx, move |root, window, cx| {
            root.open_dialog(
                move |dialog, _window, _cx| {
                    dialog
                        .title(
                            crate::i18n::paired("清除历史统计数据？", "Clear history?")
                                .get(language),
                        )
                        .content(move |content, _window, _cx| {
                            content.child(
                                crate::i18n::paired(
                                    "该队伍的统计数据将被清空，且无法恢复。",
                                    "This team's statistics will be erased and cannot be restored.",
                                )
                                .get(language),
                            )
                        })
                        .footer({
                            let cancel = crate::i18n::paired("取消", "Cancel").get(language);
                            let confirm = crate::i18n::paired("清除", "Clear").get(language);
                            let cancel_app = app.clone();
                            let confirm_app = app.clone();
                            div()
                                .flex()
                                .justify_end()
                                .gap_2()
                                .child(
                                    crate::components::button(
                                        "stats-clear-dialog-cancel",
                                        cancel,
                                        crate::components::ButtonVariant::Ghost,
                                    )
                                    .on_click(
                                        move |_, window, cx| {
                                            let _ = cancel_app.update(cx, |view, cx| {
                                                view.cancel_clear_team_stats(cx)
                                            });
                                            Root::update(window, cx, |root, window, cx| {
                                                root.close_dialog(window, cx)
                                            });
                                        },
                                    ),
                                )
                                .child(
                                    crate::components::button(
                                        "stats-clear-dialog-confirm",
                                        confirm,
                                        crate::components::ButtonVariant::Destructive,
                                    )
                                    .on_click(
                                        move |_, window, cx| {
                                            let _ = confirm_app.update(cx, |view, cx| {
                                                view.confirm_clear_team_stats(cx)
                                            });
                                            Root::update(window, cx, |root, window, cx| {
                                                root.close_dialog(window, cx)
                                            });
                                        },
                                    ),
                                )
                        })
                },
                window,
                cx,
            );
        });
        cx.notify();
    }

    pub fn cancel_clear_team_stats(&mut self, cx: &mut Context<Self>) {
        self.teams.cancel_clear_team_stats();
        cx.notify();
    }

    pub fn confirm_clear_team_stats(&mut self, cx: &mut Context<Self>) {
        if let Err(error) = self.teams.clear_team_stats() {
            self.teams.feedback = Some(error);
        }
        cx.notify();
    }
}
