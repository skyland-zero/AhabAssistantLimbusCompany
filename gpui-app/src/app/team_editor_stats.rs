use gpui::Context;

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
                    .spawn(async move { request.recv().ok() })
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

    pub fn request_clear_team_stats(&mut self, cx: &mut Context<Self>) {
        self.teams.request_clear_team_stats();
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
                    .spawn(async move { request.recv().ok() })
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
