use gpui::{Context, ParentElement as _, Styled as _, Window, div};
use gpui_component::Root;

use super::AhabApp;
use crate::{
    ipc::{RpcGateway, contract::method},
    model::TeamStats,
};

impl AhabApp {
    pub fn refresh_team_stats(&mut self, cx: &mut Context<Self>) {
        if self.teams.rpc.is_sidecar() {
            let request = match self.teams.begin_team_stats_load() {
                Ok(Some(request)) => request,
                Ok(None) => return,
                Err(error) => {
                    self.teams.feedback = Some(error);
                    cx.notify();
                    return;
                }
            };
            let (team_id, params) = request;
            let rpc = self.teams.rpc.clone();
            cx.spawn(async move |this, cx| {
                let request = rpc.request_async(method::TEAM_STATS_GET, Some(params));
                let response = cx
                    .background_executor()
                    .spawn(async move { request.recv().await.ok() })
                    .await;
                let result = match response {
                    None => Err("后端连接已断开".to_owned()),
                    Some(response) => {
                        match RpcGateway::decode_response(method::TEAM_STATS_GET, response) {
                            Err(error) => Err(error.message),
                            Ok(None) => Err("team.stats.get 返回了空结果".to_owned()),
                            Ok(Some(value)) => serde_json::from_value::<TeamStats>(value)
                                .map_err(|error| format!("team.stats.get 返回了无效统计：{error}")),
                        }
                    }
                };
                let _ = this.update(cx, |view, cx| {
                    match result {
                        Ok(stats) => {
                            view.teams.apply_team_stats(&team_id, stats);
                        }
                        Err(error) => view.teams.fail_team_stats(&team_id, error),
                    }
                    cx.notify();
                });
            })
            .detach();
        } else if let Err(error) = self.teams.refresh_team_stats() {
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
        if self.teams.rpc.is_sidecar() {
            let request = match self.teams.begin_team_stats_clear() {
                Ok(Some(request)) => request,
                Ok(None) => return,
                Err(error) => {
                    self.teams.feedback = Some(error);
                    cx.notify();
                    return;
                }
            };
            let (team_id, params) = request;
            let rpc = self.teams.rpc.clone();
            cx.spawn(async move |this, cx| {
                let request = rpc.request_async(method::TEAM_STATS_CLEAR, Some(params));
                let response = cx
                    .background_executor()
                    .spawn(async move { request.recv().await.ok() })
                    .await;
                let result = match response {
                    None => Err("后端连接已断开".to_owned()),
                    Some(response) => {
                        match RpcGateway::decode_response(method::TEAM_STATS_CLEAR, response) {
                            Err(error) => Err(error.message),
                            Ok(None) => Err("team.stats.clear 返回了空结果".to_owned()),
                            Ok(Some(value)) => {
                                serde_json::from_value::<TeamStats>(value).map_err(|error| {
                                    format!("team.stats.clear 返回了无效统计：{error}")
                                })
                            }
                        }
                    }
                };
                let _ = this.update(cx, |view, cx| {
                    match result {
                        Ok(stats) => {
                            view.teams.apply_cleared_team_stats(&team_id, stats);
                        }
                        Err(error) => view.teams.fail_team_stats(&team_id, error),
                    }
                    cx.notify();
                });
            })
            .detach();
        } else if let Err(error) = self.teams.clear_team_stats() {
            self.teams.feedback = Some(error);
        }
        cx.notify();
    }
}
