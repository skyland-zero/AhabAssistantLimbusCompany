use std::time::Duration;

use gpui::{Animation, AnimationExt, Context, Entity, Render, Window, div, prelude::*, px, rgb};

use super::AhabApp;
use super::HomeInvalidation;
use crate::{components::style::Palette, pages, shell};

impl AhabApp {
    pub(crate) fn ensure_titlebar_status_dot(
        &mut self,
        busy: bool,
        paused: bool,
        palette: Palette,
        cx: &mut Context<Self>,
    ) -> Entity<shell::StatusDot> {
        let status_dot = if let Some(status_dot) = self.titlebar_status_dot.clone() {
            status_dot
        } else {
            let status_dot = cx.new(|_| shell::StatusDot::new(busy, paused, palette));
            self.titlebar_status_dot = Some(status_dot.clone());
            status_dot
        };

        status_dot.update(cx, |view, _| view.sync_snapshot(busy, paused, palette));
        status_dot
    }

    pub(crate) fn ensure_home_views(&mut self, cx: &mut Context<Self>) {
        if self.home_views.is_none() {
            self.home_views = Some(crate::pages::HomeViewRefs::new(cx));
        }
        if let Some(views) = self.home_views.clone() {
            views.sync_from_app(self, cx);
        }
    }

    pub(crate) fn notify_home_views(
        &mut self,
        invalidation: HomeInvalidation,
        cx: &mut Context<Self>,
    ) {
        let Some(views) = self.home_views.clone() else {
            return;
        };
        views.apply_invalidation(self, cx, invalidation);
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
