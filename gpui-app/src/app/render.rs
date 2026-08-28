use std::time::Duration;

use gpui::{Animation, AnimationExt, Context, Render, Window, div, prelude::*, px, rgb};

use super::AhabApp;
use crate::{components::style::Palette, pages, shell};

impl AhabApp {
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
