use gpui::Context;

use super::AhabApp;
use crate::ipc::{RpcGateway, contract::method};

impl AhabApp {
    pub fn open_stats_details(&mut self, cx: &mut Context<Self>) {
        self.home.set_stats_details_open(true);
        self.home.stats_details_loading = true;
        self.home.stats_details_error = None;
        self.home.daily_stats = None;
        let rpc = self.home.rpc.clone();
        cx.spawn(async move |this, cx| {
            let request = rpc.request_async(method::STATS_GET_DAILY_SUMMARY, None);
            let response = cx
                .background_executor()
                .spawn(async move { request.recv().ok() })
                .await;
            let result = response
                .map(|response| {
                    RpcGateway::decode_response(method::STATS_GET_DAILY_SUMMARY, response)
                })
                .unwrap_or_else(|| Err(crate::ipc::RpcError::new(-32000, "后端连接已断开")));
            let _ = this.update(cx, |view, cx| {
                match result {
                    Ok(Some(value)) => {
                        view.home.apply_daily_stats(value);
                    }
                    Ok(None) => {
                        view.home.stats_details_loading = false;
                        view.home.stats_details_error = Some("每日统计为空".to_owned());
                    }
                    Err(error) => {
                        view.home.stats_details_loading = false;
                        view.home.stats_details_error = Some(error.message);
                    }
                }
                cx.notify();
            });
        })
        .detach();
        cx.notify();
    }

    pub fn close_stats_details(&mut self, cx: &mut Context<Self>) {
        self.home.set_stats_details_open(false);
        cx.notify();
    }
}
