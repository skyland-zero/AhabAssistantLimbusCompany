use gpui::{AppContext, Context};

use super::AhabApp;
use crate::{components::TextInput, model::Language, state::SystemU16};

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
        if self.settings_inputs.port.is_none() {
            let value = self.settings_page.system.simulator_port.to_string();
            let input =
                cx.new(move |cx| TextInput::new_with_palette(value, "ADB port", palette, cx));
            let subscription = cx.observe(&input, |view, input, cx| {
                if let Ok(value) = input.read(cx).text().parse::<u16>()
                    && value != view.settings_page.system.simulator_port
                {
                    view.settings_page
                        .set_system_u16(SystemU16::SimulatorPort, value);
                    cx.notify();
                }
            });
            self.settings_inputs.port = Some(input);
            self.settings_inputs.subscriptions.push(subscription);
        }
        if self.settings_inputs.timeout.is_none() {
            let value = self.settings_page.system.start_emulator_timeout.to_string();
            let input =
                cx.new(move |cx| TextInput::new_with_palette(value, "Timeout", palette, cx));
            let subscription = cx.observe(&input, |view, input, cx| {
                if let Ok(value) = input.read(cx).text().parse::<u16>()
                    && value != view.settings_page.system.start_emulator_timeout
                {
                    view.settings_page
                        .set_system_u16(SystemU16::StartTimeout, value);
                    cx.notify();
                }
            });
            self.settings_inputs.timeout = Some(input);
            self.settings_inputs.subscriptions.push(subscription);
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

    pub fn set_right_panel_collapsed(&mut self, collapsed: bool) {
        self.home.right_panel_collapsed = collapsed;
        self.state.settings.rightPanelCollapsed = collapsed;
        if let Err(error) = self.state.save() {
            eprintln!("failed to persist right panel layout: {error}");
        }
    }
}
