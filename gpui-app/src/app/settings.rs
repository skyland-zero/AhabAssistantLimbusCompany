use gpui::{AppContext, Context, Window};

use super::AhabApp;
use crate::{components::TextInput, model::Language};

impl AhabApp {
    /// Create the settings text inputs.
    ///
    /// Called from `attach_window` rather than from `render`: GPUI Kit's
    /// `InputState` needs a window to build, and creating entities during a
    /// render pass was a side effect the old hand-written input did not have.
    pub fn ensure_settings_input(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.settings_inputs.cdk.is_none() {
            let cdk = self.settings_page.system.mirrorchyan_cdk.clone();
            let placeholder = match self.state.settings.language {
                Language::ZhCn => "Mirror 酱 CDK（可选）",
                Language::EnUs => "Mirror-Chyan CDK (optional)",
            };
            self.settings_inputs.cdk = Some(cx.new({
                let window = &mut *window;
                move |cx| TextInput::new(cdk, placeholder, window, cx)
            }));
        }
        if self.settings_inputs.wxpusher_spt.is_none() {
            let spt = self.settings_page.system.wxpusher_spt.clone();
            let placeholder = match self.state.settings.language {
                Language::ZhCn => "WxPusher SPT（可选）",
                Language::EnUs => "WxPusher SPT (optional)",
            };
            self.settings_inputs.wxpusher_spt = Some(cx.new({
                let window = &mut *window;
                move |cx| TextInput::new_masked(spt, placeholder, window, cx)
            }));
        }
    }

    pub fn save_settings_cdk(&mut self, cx: &mut Context<Self>) {
        if let Some(input) = self.settings_inputs.cdk.as_ref() {
            self.settings_page.set_cdk(input.read(cx).text(cx));
        }
        cx.notify();
    }

    pub fn save_settings_wxpusher_spt(&mut self, cx: &mut Context<Self>) {
        if let Some(input) = self.settings_inputs.wxpusher_spt.as_ref() {
            self.settings_page.set_wxpusher_spt(input.read(cx).text(cx));
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

    pub fn set_skin(&mut self, skin: &str) {
        self.state.settings.skinId = skin.to_owned();
        if let Err(error) = self.state.save() {
            eprintln!("failed to persist skin: {error}");
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
