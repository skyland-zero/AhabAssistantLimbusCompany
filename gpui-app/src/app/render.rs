use std::sync::Arc;
use std::time::Duration;

use gpui::{
    Animation, AnimationExt, Context, Image, ImageFormat, Render, RenderImage, Window, div,
    prelude::*, px, rgb,
};

use super::AhabApp;
use crate::{components::style::Palette, pages, shell};

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
    // The JPEG payload is standard RGB. GPUI's normal image decoder has
    // already swapped it to its internal BGRA representation; the live
    // preview surface needs one explicit reversal at this boundary so the
    // preview is not affected by any global image-loader changes.
    swap_red_blue_channels(&mut bytes);
    let Some(buffer) = image::RgbaImage::from_raw(width, height, bytes) else {
        return render_image;
    };

    Arc::new(RenderImage::new(vec![image::Frame::new(buffer)]))
}

impl AhabApp {
    fn sync_screenshot_image(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let revision = self.home.screenshot_revision;
        if self.screenshot_image_revision != revision {
            if self.screenshot_pending_image_revision != Some(revision) {
                if let Some(image) = self.screenshot_pending_render_image.take() {
                    let _ = window.drop_image(image);
                }
                if let Some(source) = self.screenshot_pending_image_source.take() {
                    source.remove_asset(cx);
                }
                self.screenshot_pending_image_revision = Some(revision);
                self.screenshot_pending_image_source =
                    self.home.latest_screenshot.as_ref().map(|frame| {
                        Arc::new(Image::from_bytes(ImageFormat::Jpeg, frame.jpeg.clone()))
                    });
            }

            if self.screenshot_pending_render_image.is_none()
                && let Some(source) = self.screenshot_pending_image_source.clone()
                && let Some(render_image) = source.use_render_image(window, cx)
            {
                self.screenshot_pending_render_image =
                    Some(adapt_preview_render_image(render_image));
            }

            if self.screenshot_pending_image_source.is_none()
                && self.screenshot_pending_render_image.is_none()
            {
                if let Some(image) = self.screenshot_render_image.take() {
                    let _ = window.drop_image(image);
                }
                if let Some(source) = self.screenshot_image_source.take() {
                    source.remove_asset(cx);
                }
                self.screenshot_image_revision = revision;
                self.screenshot_pending_image_revision = None;
            } else if let Some(render_image) = self.screenshot_pending_render_image.take() {
                if let Some(image) = self.screenshot_render_image.take() {
                    let _ = window.drop_image(image);
                }
                if let Some(source) = self.screenshot_image_source.take() {
                    source.remove_asset(cx);
                }
                self.screenshot_image_source = self.screenshot_pending_image_source.take();
                self.screenshot_render_image = Some(render_image);
                self.screenshot_image_revision = revision;
                self.screenshot_pending_image_revision = None;
            }
        }
    }

    #[allow(dead_code)]
    pub fn palette(&self) -> Palette {
        crate::theme::palette_for_settings(&self.state.settings, crate::theme::system_is_dark())
    }

    pub fn palette_for_window(&self, window: &Window) -> Palette {
        let system_is_dark = matches!(
            window.appearance(),
            gpui::WindowAppearance::Dark | gpui::WindowAppearance::VibrantDark
        );
        crate::theme::palette_for_settings(&self.state.settings, system_is_dark)
    }

    fn sync_input_palettes(&mut self, palette: Palette, cx: &mut Context<Self>) {
        for input in [
            self.team_inputs.name.as_ref(),
            self.team_inputs.code.as_ref(),
            self.team_inputs.observe.as_ref(),
            self.team_inputs.json.as_ref(),
            self.team_inputs.keyword_refresh.as_ref(),
            self.team_inputs.normal_refresh.as_ref(),
            self.settings_inputs.cdk.as_ref(),
            self.settings_inputs.wxpusher_spt.as_ref(),
            self.settings_inputs.port.as_ref(),
            self.settings_inputs.timeout.as_ref(),
        ]
        .into_iter()
        .flatten()
        {
            input.update(cx, |input, _| input.set_palette(palette));
        }
    }
}

impl Render for AhabApp {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let current_page = self.current_page;
        let palette = self.palette_for_window(window);
        crate::components::style::set_current_render_palette(palette);
        self.apply_visual_state(cx);
        self.sync_input_palettes(palette, cx);
        if self.home.log_revision != self.home_log_revision_seen {
            self.home_log_scroll.scroll_to_bottom();
            self.home_log_revision_seen = self.home.log_revision;
        }
        self.sync_screenshot_image(window, cx);

        let page = pages::render(current_page, self, cx)
            .relative()
            .flex_1()
            .min_w_0()
            .min_h_0()
            .with_animation(
                format!("page-transition-{current_page:?}"),
                Animation::new(Duration::from_millis(150)).with_easing(gpui::ease_out_quint()),
                |page, progress| page.opacity(progress).top(px(4.0 * (1.0 - progress))),
            );

        div()
            .relative()
            .size_full()
            .flex()
            .flex_col()
            .bg(rgb(palette.background.rgb_hex()))
            .text_color(rgb(palette.foreground.rgb_hex()))
            .font_family("Segoe UI")
            .child(shell::title_bar(window, current_page, self, palette, cx))
            .child(
                div()
                    .flex()
                    .flex_1()
                    .min_w_0()
                    .min_h_0()
                    .flex_col()
                    .overflow_hidden()
                    .child(page),
            )
            .child(pages::render_overlay(current_page, self, cx))
            .child(shell::toast_layer(self.toast.as_ref(), palette))
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
