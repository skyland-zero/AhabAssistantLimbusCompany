use gpui::{
    ClipboardItem, Context, Entity, Render, ScrollHandle, Window, WindowAppearance, div,
    prelude::*, rgb,
};

use crate::{
    components::{TextInput, style::Palette},
    i18n::{self, Key as I18nKey},
    pages, shell,
    state::{
        AppState, HomeState, ResourcesState, SettingsPageState, TeamsState, ThemePacksState,
        ToolboxState,
    },
};

// Keep the existing page-facing imports stable while the palette lives with
// the reusable controls.
pub use crate::components::style::{
    ACCENT, BACKGROUND, BORDER, SURFACE, SURFACE_HOVER, TEXT, TEXT_MUTED,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Page {
    Home,
    Teams,
    ThemePacks,
    Toolbox,
    Resources,
    Help,
    Settings,
}

impl Page {
    pub const ALL: [Page; 7] = [
        Self::Home,
        Self::Teams,
        Self::ThemePacks,
        Self::Toolbox,
        Self::Resources,
        Self::Help,
        Self::Settings,
    ];

    pub const fn label(self) -> &'static str {
        match self {
            Self::Home => "主控台",
            Self::Teams => "队伍管理",
            Self::ThemePacks => "主题包",
            Self::Toolbox => "工具箱",
            Self::Resources => "资源中心",
            Self::Help => "帮助",
            Self::Settings => "设置",
        }
    }

    pub fn label_for(self, language: crate::model::Language) -> &'static str {
        let key = match self {
            Self::Home => I18nKey::NavHome,
            Self::Teams => I18nKey::NavTeams,
            Self::ThemePacks => I18nKey::NavThemes,
            Self::Toolbox => I18nKey::NavToolbox,
            Self::Resources => I18nKey::NavResources,
            Self::Help => I18nKey::NavHelp,
            Self::Settings => I18nKey::NavSettings,
        };
        i18n::text(language, key)
    }

    pub const fn name(self) -> &'static str {
        match self {
            Self::Home => "CONSOLE",
            Self::Teams => "TEAMS",
            Self::ThemePacks => "THEME PACKS",
            Self::Toolbox => "TOOLBOX",
            Self::Resources => "RESOURCES",
            Self::Help => "HELP",
            Self::Settings => "SETTINGS",
        }
    }
}

/// Root UI state. Home owns its task, execution, device, and log state so the
/// page can evolve without scattering business state through render methods.
pub struct AhabApp {
    pub current_page: Page,
    pub state: AppState,
    pub home: HomeState,
    pub teams: TeamsState,
    pub theme_packs: ThemePacksState,
    pub toolbox: ToolboxState,
    pub resources: ResourcesState,
    pub settings_page: SettingsPageState,
    pub team_name_input: Option<Entity<TextInput>>,
    pub team_code_input: Option<Entity<TextInput>>,
    pub team_observe_input: Option<Entity<TextInput>>,
    pub team_json_input: Option<Entity<TextInput>>,
    pub settings_cdk_input: Option<Entity<TextInput>>,
    pub help_scroll: ScrollHandle,
}

impl AhabApp {
    pub fn new() -> Self {
        let state = AppState::load();
        let home = HomeState::with_layout(
            state.settings.rightPanelWidth,
            state.settings.rightPanelCollapsed,
        );

        Self {
            current_page: Page::Home,
            state,
            home,
            teams: TeamsState::new(),
            theme_packs: ThemePacksState::new(),
            toolbox: ToolboxState::new(),
            resources: ResourcesState::new(),
            settings_page: SettingsPageState::new(),
            team_name_input: None,
            team_code_input: None,
            team_observe_input: None,
            team_json_input: None,
            settings_cdk_input: None,
            help_scroll: ScrollHandle::new(),
        }
    }

    pub fn palette(&self) -> Palette {
        crate::theme::palette_for_settings(&self.state.settings, crate::theme::system_is_dark())
    }

    pub fn palette_for_window(&self, window: &Window) -> Palette {
        let system_is_dark = matches!(
            window.appearance(),
            WindowAppearance::Dark | WindowAppearance::VibrantDark
        );
        crate::theme::palette_for_settings(&self.state.settings, system_is_dark)
    }

    pub fn select_page(&mut self, page: Page, cx: &mut Context<Self>) {
        self.current_page = page;
        cx.notify();
    }

    pub fn ensure_settings_input(&mut self, cx: &mut Context<Self>) {
        let palette = self.palette();
        if self.settings_cdk_input.is_none() {
            let cdk = self.settings_page.system.mirrorchyan_cdk.clone();
            self.settings_cdk_input = Some(cx.new(move |cx| {
                TextInput::new_with_palette(cdk, "Mirror 酱 CDK（可选）", palette, cx)
            }));
        }
    }

    pub fn save_settings_cdk(&mut self, cx: &mut Context<Self>) {
        if let Some(input) = self.settings_cdk_input.as_ref() {
            self.settings_page.set_cdk(input.read(cx).text());
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

    pub fn set_language(&mut self, language: crate::model::Language) {
        self.state.settings.language = language;
        if let Err(error) = self.state.save() {
            eprintln!("failed to persist language: {error}");
        }
    }

    pub fn open_new_team(&mut self, cx: &mut Context<Self>) {
        self.teams.open_new();
        self.create_team_inputs(cx);
        cx.notify();
    }

    pub fn open_existing_team(&mut self, team: &crate::model::TeamDetail, cx: &mut Context<Self>) {
        self.teams.open_edit(team);
        self.create_team_inputs(cx);
        cx.notify();
    }

    pub fn close_team_editor(&mut self, cx: &mut Context<Self>) {
        self.teams.close_editor();
        self.clear_team_inputs();
        cx.notify();
    }

    pub fn save_team_editor(&mut self, cx: &mut Context<Self>) {
        self.sync_team_inputs_to_state(cx);
        match self.teams.save_editor() {
            Ok(()) => self.clear_team_inputs(),
            Err(error) => self.teams.feedback = Some(error),
        }
        cx.notify();
    }

    pub fn copy_team_json(&mut self, cx: &mut Context<Self>) {
        self.sync_team_inputs_to_state(cx);
        if let Some(json) = self.teams.export_editor_json() {
            cx.write_to_clipboard(ClipboardItem::new_string(json));
            self.teams.feedback = Some("队伍 JSON 已复制".to_owned());
        }
        cx.notify();
    }

    pub fn import_team_json(&mut self, cx: &mut Context<Self>) {
        let input = self
            .team_json_input
            .as_ref()
            .map(|entity| entity.read(cx).text())
            .unwrap_or_default();
        match self.teams.import_editor_json(&input) {
            Ok(()) => {
                self.sync_team_inputs_from_state(cx);
                if let Some(entity) = self.team_json_input.as_ref() {
                    entity.update(cx, |input, _| input.set_text(""));
                }
            }
            Err(error) => self.teams.feedback = Some(format!("导入失败：{error}")),
        }
        cx.notify();
    }

    pub fn add_team_observe_gift(&mut self, cx: &mut Context<Self>) {
        let input = self
            .team_observe_input
            .as_ref()
            .map(|entity| entity.read(cx).text())
            .unwrap_or_default();
        if self.teams.add_observe_gift(&input) {
            if let Some(entity) = self.team_observe_input.as_ref() {
                entity.update(cx, |input, _| input.set_text(""));
            }
        }
        cx.notify();
    }

    fn create_team_inputs(&mut self, cx: &mut Context<Self>) {
        let palette = self.palette();
        let editor = self.teams.editor.as_ref().expect("editor just opened");
        let name = editor.team.name.clone();
        let code = editor.mirror_config().team_code;
        self.team_name_input =
            Some(cx.new(move |cx| TextInput::new_with_palette(name, "队伍名称", palette, cx)));
        self.team_code_input = Some(
            cx.new(move |cx| TextInput::new_with_palette(code, "编队码（可选）", palette, cx)),
        );
        self.team_observe_input =
            Some(cx.new(move |cx| {
                TextInput::new_with_palette("", "输入 E.G.O 饰品名称", palette, cx)
            }));
        self.team_json_input =
            Some(cx.new(move |cx| TextInput::new_with_palette("", "粘贴 Team JSON", palette, cx)));
    }

    fn clear_team_inputs(&mut self) {
        self.team_name_input = None;
        self.team_code_input = None;
        self.team_observe_input = None;
        self.team_json_input = None;
    }

    fn sync_team_inputs_to_state(&mut self, cx: &mut Context<Self>) {
        let name = self
            .team_name_input
            .as_ref()
            .map(|input| input.read(cx).text());
        let code = self
            .team_code_input
            .as_ref()
            .map(|input| input.read(cx).text());
        if let Some(editor) = self.teams.editor.as_mut() {
            if let Some(name) = name {
                editor.team.name = name;
            }
            if let Some(code) = code {
                editor
                    .team
                    .mirrorConfig
                    .get_or_insert_with(Default::default)
                    .team_code = code;
            }
        }
    }

    fn sync_team_inputs_from_state(&mut self, cx: &mut Context<Self>) {
        let Some(editor) = self.teams.editor.as_ref() else {
            return;
        };
        let name = editor.team.name.clone();
        let code = editor.mirror_config().team_code;
        if let Some(input) = self.team_name_input.as_ref() {
            input.update(cx, |input, _| input.set_text(name));
        }
        if let Some(input) = self.team_code_input.as_ref() {
            input.update(cx, |input, _| input.set_text(code));
        }
    }

    fn sync_input_palettes(&mut self, palette: Palette, cx: &mut Context<Self>) {
        for input in [
            self.team_name_input.as_ref(),
            self.team_code_input.as_ref(),
            self.team_observe_input.as_ref(),
            self.team_json_input.as_ref(),
            self.settings_cdk_input.as_ref(),
        ]
        .into_iter()
        .flatten()
        {
            input.update(cx, |input, _| input.set_palette(palette));
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

impl Render for AhabApp {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let current_page = self.current_page;
        let language = self.state.settings.language;
        let palette = self.palette_for_window(window);
        self.sync_input_palettes(palette, cx);

        div()
            .relative()
            .size_full()
            .flex()
            .flex_col()
            .bg(rgb(palette.background.rgb_hex()))
            .text_color(rgb(palette.foreground.rgb_hex()))
            .font_family("Segoe UI")
            .child(shell::title_bar(window, language, palette))
            .child(shell::tab_bar(current_page, self, palette, cx))
            .child(
                div()
                    .flex()
                    .flex_1()
                    .min_w_0()
                    .min_h_0()
                    .flex_col()
                    .overflow_hidden()
                    .child(
                        pages::render(current_page, self, cx)
                            .flex_1()
                            .min_w_0()
                            .min_h_0(),
                    ),
            )
            .child(pages::render_overlay(current_page, self, cx))
    }
}
