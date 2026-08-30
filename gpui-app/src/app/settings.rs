use gpui::{AppContext, Context, Window};

use super::AhabApp;
use crate::{components::TextInput, model::Language};

impl AhabApp {
    pub fn ensure_settings_input(&mut self, cx: &mut Context<Self>) {
        let palette = crate::components::style::current_render_palette();
        if self.settings_inputs.cdk.is_none() {
            let cdk = self.settings_page.system.mirrorchyan_cdk.clone();
            let placeholder = match self.state.settings.language {
                Language::ZhCn => "Mirror 酱 CDK（可选）",
                Language::EnUs => "Mirror-Chyan CDK (optional)",
            };
            self.settings_inputs.cdk =
                Some(cx.new(move |cx| TextInput::new_with_palette(cdk, placeholder, palette, cx)));
        }
        if self.settings_inputs.wxpusher_spt.is_none() {
            let spt = self.settings_page.system.wxpusher_spt.clone();
            let placeholder = match self.state.settings.language {
                Language::ZhCn => "WxPusher SPT（可选）",
                Language::EnUs => "WxPusher SPT (optional)",
            };
            self.settings_inputs.wxpusher_spt =
                Some(cx.new(move |cx| {
                    TextInput::new_masked_with_palette(spt, placeholder, palette, cx)
                }));
        }
    }

    pub fn save_settings_cdk(&mut self, cx: &mut Context<Self>) {
        if let Some(input) = self.settings_inputs.cdk.as_ref() {
            self.settings_page.set_cdk(input.read(cx).text());
        }
        cx.notify();
    }

    pub fn save_settings_wxpusher_spt(&mut self, cx: &mut Context<Self>) {
        if let Some(input) = self.settings_inputs.wxpusher_spt.as_ref() {
            self.settings_page.set_wxpusher_spt(input.read(cx).text());
        }
        cx.notify();
    }

    pub fn set_theme_mode(&mut self, mode: crate::model::ThemeMode) {
        self.state.settings.themeMode = mode;
        if let Err(error) = self.state.save() {
            eprintln!("failed to persist theme mode: {error}");
        }
    }

    pub fn set_accent(&mut self, accent: &str) {
        self.state.settings.accentId = accent.to_owned();
        if let Err(error) = self.state.save() {
            eprintln!("failed to persist accent: {error}");
        }
    }

    pub fn set_language(&mut self, language: Language) {
        self.state.settings.language = language;
        if let Err(error) = self.state.save() {
            eprintln!("failed to persist language: {error}");
        }
    }

    pub fn set_right_panel_width(&mut self, width: u32) {
        let width = width.clamp(280, 800);
        self.home.right_panel_width = width as f32;
        self.state.settings.rightPanelWidth = width;
        if let Err(error) = self.state.save() {
            eprintln!("failed to persist right panel width: {error}");
        }
    }

    pub fn set_right_panel_collapsed(
        &mut self,
        collapsed: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.home.right_panel_collapsed = collapsed;
        self.state.settings.rightPanelCollapsed = collapsed;
        if let Err(error) = self.state.save() {
            eprintln!("failed to persist right panel layout: {error}");
        }
        self.reconcile_preview(Some(window), cx);
    }
}
