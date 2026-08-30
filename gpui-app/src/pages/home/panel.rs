use super::*;

use std::sync::Arc;

use gpui::{Context, Image, ImageFormat, ObjectFit, Render, RenderImage, Window, img};

use crate::{
    model::{Language, PreviewStatus, ScreenshotFrame},
    state::HomeState,
};

pub(super) struct PreviewView {
    latest_screenshot: Option<ScreenshotFrame>,
    preview_status: PreviewStatus,
    preview_error: Option<String>,
    language: Language,
    home_screenshot_revision: u64,
    screenshot_image_source: Option<Arc<Image>>,
    screenshot_render_image: Option<Arc<RenderImage>>,
    screenshot_image_revision: u64,
    screenshot_pending_image_source: Option<Arc<Image>>,
    screenshot_pending_render_image: Option<Arc<RenderImage>>,
    screenshot_pending_image_revision: Option<u64>,
}

impl PreviewView {
    pub(super) fn new() -> Self {
        Self {
            latest_screenshot: None,
            preview_status: PreviewStatus::Stopped,
            preview_error: None,
            language: Language::ZhCn,
            home_screenshot_revision: u64::MAX,
            screenshot_image_source: None,
            screenshot_render_image: None,
            screenshot_image_revision: u64::MAX,
            screenshot_pending_image_source: None,
            screenshot_pending_render_image: None,
            screenshot_pending_image_revision: None,
        }
    }

    pub(super) fn sync_snapshot(
        &mut self,
        latest_screenshot: Option<ScreenshotFrame>,
        screenshot_revision: u64,
        preview_status: PreviewStatus,
        preview_error: Option<String>,
        language: Language,
    ) {
        self.language = language;
        self.preview_status = preview_status;
        self.preview_error = preview_error;
        if self.home_screenshot_revision != screenshot_revision {
            self.latest_screenshot = latest_screenshot;
            self.home_screenshot_revision = screenshot_revision;
        }
    }

    fn sync_screenshot_image(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let revision = self.home_screenshot_revision;
        if self.screenshot_image_revision == revision {
            return;
        }

        if self.screenshot_pending_image_revision != Some(revision) {
            if let Some(image) = self.screenshot_pending_render_image.take() {
                let _ = window.drop_image(image);
            }
            self.screenshot_pending_image_revision = Some(revision);
            self.screenshot_pending_image_source = self
                .latest_screenshot
                .as_ref()
                .map(|frame| Arc::new(Image::from_bytes(ImageFormat::Jpeg, frame.jpeg.clone())));
        }

        if self.screenshot_pending_render_image.is_none()
            && let Some(source) = self.screenshot_pending_image_source.clone()
            && let Some(render_image) = source.use_render_image(window, cx)
        {
            self.screenshot_pending_render_image = Some(adapt_preview_render_image(render_image));
        }

        if self.screenshot_pending_image_source.is_none()
            && self.screenshot_pending_render_image.is_none()
        {
            if let Some(image) = self.screenshot_render_image.take() {
                let _ = window.drop_image(image);
            }
            self.screenshot_image_source = None;
            self.screenshot_image_revision = revision;
            self.screenshot_pending_image_revision = None;
        } else if let Some(render_image) = self.screenshot_pending_render_image.take() {
            if let Some(image) = self.screenshot_render_image.take() {
                let _ = window.drop_image(image);
            }
            self.screenshot_image_source = self.screenshot_pending_image_source.take();
            self.screenshot_render_image = Some(render_image);
            self.screenshot_image_revision = revision;
            self.screenshot_pending_image_revision = None;
        }
    }

    pub(super) fn clear_render_resources(&mut self, window: &mut Window) {
        if let Some(image) = self.screenshot_render_image.take() {
            let _ = window.drop_image(image);
        }
        if let Some(image) = self.screenshot_pending_render_image.take() {
            let _ = window.drop_image(image);
        }
        self.screenshot_image_source = None;
        self.screenshot_pending_image_source = None;
        self.screenshot_pending_image_revision = None;
        self.screenshot_image_revision = self.home_screenshot_revision;
    }
}

impl Render for PreviewView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.sync_screenshot_image(window, cx);
        let language = self.language;
        let mut screenshot_body = div()
            .relative()
            .w_full()
            .aspect_ratio(16.0 / 9.0)
            .overflow_hidden()
            .rounded_md()
            .border_1()
            .border_color(rgba(0))
            .bg(rgb(BACKGROUND))
            .text_size(px(11.0));

        if let Some(image) = self.screenshot_render_image.clone() {
            screenshot_body = screenshot_body
                .child(
                    div()
                        .absolute()
                        .top_0()
                        .left_0()
                        .right_0()
                        .bottom_0()
                        .child(img(image).size_full().object_fit(ObjectFit::Contain)),
                )
                .child(
                    div()
                        .absolute()
                        .top(px(8.0))
                        .right(px(8.0))
                        .child(live_indicator()),
                );
        } else {
            let screenshot_detail = if self.latest_screenshot.is_some() {
                text("画面加载中", "Loading game screen").get(language)
            } else {
                match self.preview_status {
                    PreviewStatus::Starting => {
                        text("正在获取画面", "Getting game screen").get(language)
                    }
                    PreviewStatus::Error => {
                        text("实时画面获取失败", "Live screen unavailable").get(language)
                    }
                    PreviewStatus::Running => {
                        text("等待最新画面", "Waiting for latest frame").get(language)
                    }
                    PreviewStatus::Stopped => text(
                        "连接设备后显示实时画面",
                        "Connect a device to view the game",
                    )
                    .get(language),
                }
            };
            let detail = self
                .preview_error
                .clone()
                .unwrap_or_else(|| screenshot_detail.to_owned());
            screenshot_body = screenshot_body
                .flex()
                .flex_col()
                .items_center()
                .justify_center()
                .gap_1()
                .border_dashed()
                .border_color(rgb(SURFACE_HOVER))
                .text_color(rgb(TEXT_MUTED))
                .child(
                    div()
                        .opacity(0.25)
                        .child(action_icon(ICON_MONITOR_PLAY, 32., TEXT_MUTED)),
                )
                .child(detail);
        }

        let screenshot_header = div()
            .h(px(32.0))
            .flex_none()
            .flex()
            .items_center()
            .px(px(10.0))
            .child(panel_heading(
                ICON_MONITOR_PLAY,
                text("实时画面", "Live Screen").get(language),
            ));

        panel_card(
            div()
                .flex()
                .flex_col()
                .child(screenshot_header)
                .child(div().p(px(10.0)).child(screenshot_body)),
        )
    }
}

pub(super) fn splitter(app: &mut AhabApp, cx: &mut Context<AhabApp>) -> gpui::Stateful<Div> {
    let collapsed = app.home.right_panel_collapsed;
    let width = if collapsed {
        SPLITTER_COLLAPSED_WIDTH
    } else {
        SPLITTER_WIDTH
    };
    let mut handle = div()
        .id("home-panel-splitter")
        .w(px(width))
        .h_full()
        .flex_none()
        .flex()
        .items_center()
        .justify_center()
        .cursor(gpui::CursorStyle::ResizeColumn)
        .hover(|style| style.bg(rgba((ACCENT << 8) | 0x35)))
        .child(div().w(px(2.0)).h(px(32.0)).rounded_full().bg(rgba(0)))
        .on_drag(SplitterDragGhost, |_, _, _, cx| {
            cx.new(|_| SplitterDragGhost)
        });
    handle = handle.on_drag_move(cx.listener(
        |view, event: &gpui::DragMoveEvent<SplitterDragGhost>, window, cx| {
            let viewport_width = window.viewport_size().width.as_f32();
            let requested_width = viewport_width - event.event.position.x.as_f32();
            if requested_width < SPLITTER_COLLAPSE_THRESHOLD {
                view.set_right_panel_collapsed(true, window, cx);
            } else {
                let available_max = (viewport_width - MIN_LEFT_PANEL_WIDTH - SPLITTER_WIDTH)
                    .clamp(RIGHT_PANEL_MIN_WIDTH, RIGHT_PANEL_MAX_WIDTH);
                view.set_right_panel_width(
                    requested_width
                        .clamp(RIGHT_PANEL_MIN_WIDTH, available_max)
                        .round() as u32,
                );
                view.set_right_panel_collapsed(false, window, cx);
            }
            cx.notify();
        },
    ));
    handle
}

pub(super) fn right_panel(app: &mut AhabApp, cx: &mut Context<AhabApp>) -> Div {
    let width = bounded_right_panel_width(app.home.right_panel_width);
    let screenshot_card = screenshot_card(app);

    div()
        .w(px(width))
        .min_w(px(240.0))
        .h_full()
        .min_h_0()
        .flex_shrink_1()
        .flex()
        .flex_col()
        .gap_2()
        .overflow_x_hidden()
        .pt(px(8.0))
        .pb(px(8.0))
        .pl(px(4.0))
        .pr(px(8.0))
        .child(super::device_panel::connection_card(app, cx).flex_none())
        .child(screenshot_card)
        .child(super::log_panel::logs_card(app))
}

fn screenshot_card(app: &AhabApp) -> Div {
    let preview = app.home_views.as_ref().map(|views| views.preview_view());
    div()
        .flex_none()
        .when_some(preview, |this, preview| this.child(preview))
}

fn live_indicator() -> Div {
    let success = palette_rgb(current_render_palette().success);
    div()
        .flex()
        .items_center()
        .gap(px(3.0))
        .text_size(px(9.0))
        .font_weight(FontWeight::SEMIBOLD)
        .text_color(success)
        .child("LIVE")
        .child(div().w(px(5.0)).h(px(5.0)).rounded_full().bg(success))
}

fn swap_red_blue_channels(bytes: &mut [u8]) {
    let (pixels, _) = bytes.as_chunks_mut::<4>();
    for pixel in pixels {
        pixel.swap(0, 2);
    }
}

fn adapt_preview_render_image(render_image: Arc<RenderImage>) -> Arc<RenderImage> {
    let Some(source_bytes) = render_image.as_bytes(0) else {
        return render_image;
    };

    let size = render_image.size(0);
    let width = size.width.0.max(0) as u32;
    let height = size.height.0.max(0) as u32;
    let expected_len = width as usize * height as usize * 4;
    if width == 0 || height == 0 || source_bytes.len() != expected_len {
        return render_image;
    }

    let mut bytes = source_bytes.to_vec();
    // JPEG is decoded as RGB while GPUI's image surface expects the
    // component order used by the D3D-backed renderer.
    swap_red_blue_channels(&mut bytes);
    let Some(buffer) = image::RgbaImage::from_raw(width, height, bytes) else {
        return render_image;
    };

    Arc::new(RenderImage::new(vec![image::Frame::new(buffer)]))
}

pub(super) fn panel_card(child: impl IntoElement) -> Div {
    div()
        .min_w_0()
        .overflow_hidden()
        .rounded_lg()
        .border_1()
        .border_color(rgba(0))
        .bg(rgb(SURFACE))
        .child(child)
}

pub(super) fn panel_heading(icon_data: &'static [u8], title: &'static str) -> Div {
    div()
        .flex()
        .items_center()
        .gap_2()
        .text_size(px(12.0))
        .text_color(rgb(TEXT_MUTED))
        .child(action_icon(icon_data, 14., TEXT_MUTED))
        .child(title)
}

pub(super) fn first_executable_task(home: &HomeState) -> Option<FixedTaskId> {
    let enabled = &home.tasks.enabledTasks;
    if enabled.daily_task {
        Some(FixedTaskId::DailyTask)
    } else if enabled.get_reward {
        Some(FixedTaskId::GetReward)
    } else if enabled.buy_enkephalin {
        Some(FixedTaskId::BuyEnkephalin)
    } else if enabled.mirror {
        Some(FixedTaskId::Mirror)
    } else {
        None
    }
}

pub(super) fn bounded_right_panel_width(width: f32) -> f32 {
    if !width.is_finite() {
        RIGHT_PANEL_DEFAULT_WIDTH
    } else {
        width.clamp(RIGHT_PANEL_MIN_WIDTH, RIGHT_PANEL_MAX_WIDTH)
    }
}

pub(super) fn reward_mode_label(mode: u8, language: Language) -> &'static str {
    match mode {
        0 => text("全部", "All").get(language),
        1 => text("狂气/通行证", "Lunacy/Pass").get(language),
        2 => text("邮件", "Mail").get(language),
        _ => text("全部", "All").get(language),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preview_render_adapter_swaps_red_and_blue_once() {
        let render_image = Arc::new(RenderImage::new(vec![image::Frame::new(
            image::RgbaImage::from_raw(1, 1, vec![0x10, 0x20, 0x30, 0xff]).unwrap(),
        )]));

        let adapted = adapt_preview_render_image(render_image);

        assert_eq!(adapted.as_bytes(0).unwrap(), &[0x30, 0x20, 0x10, 0xff]);
    }

    #[test]
    fn channel_swap_keeps_alpha_and_green_channels() {
        let mut bytes = vec![0x10, 0x20, 0x30, 0x40, 0xaa, 0xbb, 0xcc, 0xdd];

        swap_red_blue_channels(&mut bytes);

        assert_eq!(bytes, vec![0x30, 0x20, 0x10, 0x40, 0xcc, 0xbb, 0xaa, 0xdd]);
    }
}
