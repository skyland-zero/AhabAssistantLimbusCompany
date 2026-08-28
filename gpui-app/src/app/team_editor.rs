use gpui::{AppContext, ClipboardItem, Context};

use super::AhabApp;
use crate::{app_inputs::TeamInputs, components::TextInput, model::TeamDetail};

impl AhabApp {
    pub fn open_new_team(&mut self, cx: &mut Context<Self>) {
        self.teams.open_new();
        self.create_team_inputs(cx);
        cx.notify();
    }

    pub fn open_existing_team(&mut self, team: &TeamDetail, cx: &mut Context<Self>) {
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
            .team_inputs
            .json
            .as_ref()
            .map(|entity| entity.read(cx).text())
            .unwrap_or_default();
        match self.teams.import_editor_json(&input) {
            Ok(()) => {
                self.sync_team_inputs_from_state(cx);
                if let Some(entity) = self.team_inputs.json.as_ref() {
                    entity.update(cx, |input, _| input.set_text(""));
                }
            }
            Err(error) => self.teams.feedback = Some(format!("导入失败：{error}")),
        }
        cx.notify();
    }

    pub fn add_team_observe_gift(&mut self, cx: &mut Context<Self>) {
        let input = self
            .team_inputs
            .observe
            .as_ref()
            .map(|entity| entity.read(cx).text())
            .unwrap_or_default();
        if self.teams.add_observe_gift(&input)
            && let Some(entity) = self.team_inputs.observe.as_ref()
        {
            entity.update(cx, |input, _| input.set_text(""));
        }
        cx.notify();
    }

    pub(crate) fn create_team_inputs(&mut self, cx: &mut Context<Self>) {
        self.team_inputs.subscriptions.clear();
        let palette = crate::components::style::current_render_palette();
        let language = self.state.settings.language;
        let editor = self.teams.editor.as_ref().expect("editor just opened");
        let name = editor.team.name.clone();
        let mirror_config = editor.mirror_config();
        let code = mirror_config.team_code.clone();
        let keyword_refresh = mirror_config.max_keyword_refresh.to_string();
        let normal_refresh = mirror_config.max_normal_refresh.to_string();
        let name_placeholder = match language {
            crate::model::Language::ZhCn => "队伍名称",
            crate::model::Language::EnUs => "Team name",
        };
        let code_placeholder = match language {
            crate::model::Language::ZhCn => "编队码（可选）",
            crate::model::Language::EnUs => "Team code (optional)",
        };
        let observe_placeholder = match language {
            crate::model::Language::ZhCn => "输入 E.G.O 饰品名称",
            crate::model::Language::EnUs => "Enter E.G.O gift name",
        };
        let json_placeholder = match language {
            crate::model::Language::ZhCn => "粘贴 Team JSON",
            crate::model::Language::EnUs => "Paste Team JSON",
        };
        self.team_inputs.name = Some(
            cx.new(move |cx| TextInput::new_with_palette(name, name_placeholder, palette, cx)),
        );
        self.team_inputs.code = Some(
            cx.new(move |cx| TextInput::new_with_palette(code, code_placeholder, palette, cx)),
        );
        self.team_inputs.observe = Some(
            cx.new(move |cx| TextInput::new_with_palette("", observe_placeholder, palette, cx)),
        );
        self.team_inputs.json =
            Some(cx.new(move |cx| TextInput::new_with_palette("", json_placeholder, palette, cx)));
        let keyword_input =
            cx.new(move |cx| TextInput::new_with_palette(keyword_refresh, "0-10", palette, cx));
        let keyword_subscription = cx.observe(&keyword_input, |view, input, cx| {
            if let Ok(value) = input.read(cx).text().parse::<u8>() {
                let value = value.min(10);
                view.teams
                    .set_mirror_u8(crate::state::MirrorU8::MaxKeywordRefresh, value);
                input.update(cx, |input, _| input.set_text(value.to_string()));
                cx.notify();
            }
        });
        self.team_inputs.keyword_refresh = Some(keyword_input);
        self.team_inputs.subscriptions.push(keyword_subscription);

        let normal_input =
            cx.new(move |cx| TextInput::new_with_palette(normal_refresh, "0-10", palette, cx));
        let normal_subscription = cx.observe(&normal_input, |view, input, cx| {
            if let Ok(value) = input.read(cx).text().parse::<u8>() {
                let value = value.min(10);
                view.teams
                    .set_mirror_u8(crate::state::MirrorU8::MaxNormalRefresh, value);
                input.update(cx, |input, _| input.set_text(value.to_string()));
                cx.notify();
            }
        });
        self.team_inputs.normal_refresh = Some(normal_input);
        self.team_inputs.subscriptions.push(normal_subscription);
    }

    fn clear_team_inputs(&mut self) {
        self.team_inputs = TeamInputs::default();
    }

    fn sync_team_inputs_to_state(&mut self, cx: &mut Context<Self>) {
        let name = self
            .team_inputs
            .name
            .as_ref()
            .map(|input| input.read(cx).text());
        let code = self
            .team_inputs
            .code
            .as_ref()
            .map(|input| input.read(cx).text());
        let keyword_refresh = self
            .team_inputs
            .keyword_refresh
            .as_ref()
            .and_then(|input| input.read(cx).text().parse::<u8>().ok());
        let normal_refresh = self
            .team_inputs
            .normal_refresh
            .as_ref()
            .and_then(|input| input.read(cx).text().parse::<u8>().ok());
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
            if let Some(value) = keyword_refresh {
                editor
                    .team
                    .mirrorConfig
                    .get_or_insert_with(Default::default)
                    .max_keyword_refresh = value.min(10);
            }
            if let Some(value) = normal_refresh {
                editor
                    .team
                    .mirrorConfig
                    .get_or_insert_with(Default::default)
                    .max_normal_refresh = value.min(10);
            }
        }
    }

    fn sync_team_inputs_from_state(&mut self, cx: &mut Context<Self>) {
        let Some(editor) = self.teams.editor.as_ref() else {
            return;
        };
        let name = editor.team.name.clone();
        let mirror_config = editor.mirror_config();
        let code = mirror_config.team_code;
        let keyword_refresh = mirror_config.max_keyword_refresh.to_string();
        let normal_refresh = mirror_config.max_normal_refresh.to_string();
        if let Some(input) = self.team_inputs.name.as_ref() {
            input.update(cx, |input, _| input.set_text(name));
        }
        if let Some(input) = self.team_inputs.code.as_ref() {
            input.update(cx, |input, _| input.set_text(code));
        }
        if let Some(input) = self.team_inputs.keyword_refresh.as_ref() {
            input.update(cx, |input, _| input.set_text(keyword_refresh));
        }
        if let Some(input) = self.team_inputs.normal_refresh.as_ref() {
            input.update(cx, |input, _| input.set_text(normal_refresh));
        }
    }
}
