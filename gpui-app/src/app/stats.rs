use gpui::{Context, ParentElement as _, Styled as _, Window, div};
use gpui_component::WindowExt as _;

use super::AhabApp;
use crate::ipc::{RpcGateway, contract::method};

impl AhabApp {
    pub fn open_stats_details(&mut self, cx: &mut Context<Self>) {
        self.home.set_mirror_details_open(false);
        self.home.set_stats_details_open(true);
        self.home.stats_details_loading = true;
        self.home.stats_details_error = None;
        self.home.daily_stats = None;
        let rpc = self.home.rpc.clone();
        cx.spawn(async move |this, cx| {
            let request = rpc.request_async(method::STATS_GET_DAILY_SUMMARY, None);
            let response = cx
                .background_executor()
                .spawn(async move { request.recv().await.ok() })
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

    /// Opens the mirror history as a side sheet.
    ///
    /// The records are snapshotted here, while the app is not being rendered:
    /// a `Root` sheet's builder runs inside `AhabApp::render` and cannot borrow
    /// the app. That is only sound because this viewer's data is already
    /// loaded - see `open_stats_details`, which has to fetch first and cannot
    /// use the same trick without losing its loading state.
    pub fn open_mirror_details(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.home.set_stats_details_open(false);
        self.home.set_mirror_details_open(true);
        let language = self.state.settings.language;
        let records: Vec<_> = if self.home.stats.mirrorHistory.is_empty() {
            self.home.stats.lastMirror.clone().into_iter().collect()
        } else {
            self.home.stats.mirrorHistory.clone()
        };
        let app = cx.entity().downgrade();
        let close_app = app.clone();
        window.open_sheet(cx, move |sheet, _window, _cx| {
            let count = records.len();
            sheet
                .title(
                    div()
                        .flex()
                        .items_center()
                        .gap_2()
                        .child(
                            gpui_component::Icon::new(crate::components::IconName::ScrollText)
                                .size(gpui::px(17.))
                                .text_color(crate::components::style::palette_rgb(
                                    crate::components::style::current_render_palette().brand,
                                )),
                        )
                        .child(
                            div()
                                .flex()
                                .flex_col()
                                .gap(gpui::px(1.0))
                                .child(
                                    div()
                                        .text_size(gpui::px(16.))
                                        .font_weight(gpui::FontWeight::SEMIBOLD)
                                        .child(
                                            crate::i18n::paired("镜牢明细", "Mirror Details")
                                                .get(language),
                                        ),
                                )
                                .child(div().text_size(gpui::px(10.)).child(format!(
                                    "{} {count}/30",
                                    crate::i18n::paired("最近", "Latest").get(language)
                                ))),
                        ),
                )
                .size(gpui::relative(0.6))
                .on_close({
                    let close_app = close_app.clone();
                    move |_, _, cx| {
                        let _ = close_app.update(cx, |view, cx| view.close_mirror_details(cx));
                    }
                })
                .child(crate::pages::mirror_history_body(&records, language))
        });
        cx.notify();
    }

    pub fn close_mirror_details(&mut self, cx: &mut Context<Self>) {
        self.home.set_mirror_details_open(false);
        cx.notify();
    }
}
