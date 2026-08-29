use gpui::{AppContext, ClipboardItem, Context};

use super::AhabApp;
use crate::{
    app_inputs::TeamInputs,
    components::TextInput,
    ipc::{RpcGateway, contract::method},
    model::TeamDetail,
};

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
        if self.teams.saving {
            self.teams.feedback = Some("队伍正在保存，请等待后端响应".to_owned());
            cx.notify();
            return;
        }
        self.teams.close_editor();
        self.clear_team_inputs();
        cx.notify();
    }

    pub fn save_team_editor(&mut self, cx: &mut Context<Self>) {
        self.sync_team_inputs_to_state(cx);
        if self.teams.rpc.is_sidecar() {
            let (submitted, value) = match self.teams.prepare_save() {
                Ok(prepared) => prepared,
                Err(error) => {
                    self.teams.feedback = Some(error);
                    cx.notify();
                    return;
                }
            };
            let rpc = self.teams.rpc.clone();
            cx.spawn(async move |this, cx| {
                let request = rpc.request_async(method::TEAM_SAVE, Some(value));
                let response = cx
                    .background_executor()
                    .spawn(async move { request.recv().ok() })
                    .await;
                let result = match response {
                    None => Err("后端连接已断开".to_owned()),
                    Some(response) => {
                        match RpcGateway::decode_response(method::TEAM_SAVE, response) {
                            Err(error) => Err(error.message),
                            Ok(None) => Err("team.save 返回了空结果".to_owned()),
                            Ok(Some(value)) if value.as_bool() == Some(true) => {
                                let list_request = rpc.request_async(method::TEAM_LIST, None);
                                let list_response = cx
                                    .background_executor()
                                    .spawn(async move { list_request.recv().ok() })
                                    .await;
                                match list_response {
                                    None => Err("team.save 已成功，但无法读取队伍列表".to_owned()),
                                    Some(response) => match RpcGateway::decode_response(
                                        method::TEAM_LIST,
                                        response,
                                    ) {
                                        Err(error) => Err(error.message),
                                        Ok(None) => Err("team.list 返回了空结果".to_owned()),
                                        Ok(Some(value)) => {
                                            resolve_saved_team_from_list(value, &submitted)
                                        }
                                    },
                                }
                            }
                            Ok(Some(value)) => serde_json::from_value(value)
                                .map_err(|error| format!("team.save 返回了无效队伍：{error}")),
                        }
                    }
                };
                let _ = this.update(cx, |view, cx| {
                    match result {
                        Ok(saved) => {
                            view.teams.apply_saved_team(&submitted, saved);
                            view.clear_team_inputs();
                        }
                        Err(error) => view.teams.fail_save(error),
                    }
                    cx.notify();
                });
            })
            .detach();
        } else {
            match self.teams.save_editor() {
                Ok(()) => self.clear_team_inputs(),
                Err(error) => self.teams.feedback = Some(error),
            }
        }
        cx.notify();
    }

    pub fn confirm_delete(&mut self, cx: &mut Context<Self>) {
        if self.teams.rpc.is_sidecar() {
            let team = match self.teams.prepare_delete() {
                Ok(team) => team,
                Err(error) => {
                    self.teams.feedback = Some(error);
                    cx.notify();
                    return;
                }
            };
            let team_id = team.id.clone();
            let rpc = self.teams.rpc.clone();
            cx.spawn(async move |this, cx| {
                let request = rpc.request_async(
                    method::TEAM_DELETE,
                    Some(serde_json::json!({"id": team_id.clone()})),
                );
                let response = cx
                    .background_executor()
                    .spawn(async move { request.recv().ok() })
                    .await;
                let result = match response {
                    None => Err("后端连接已断开".to_owned()),
                    Some(response) => {
                        match RpcGateway::decode_response(method::TEAM_DELETE, response) {
                            Err(error) => Err(error.message),
                            Ok(Some(value)) if value.as_bool() == Some(true) => Ok(()),
                            Ok(Some(_)) => Err("team.delete 返回了无效结果".to_owned()),
                            Ok(None) => Err("team.delete 返回了空结果".to_owned()),
                        }
                    }
                };
                let _ = this.update(cx, |view, cx| {
                    match result {
                        Ok(()) => view.teams.apply_deleted_team(&team_id),
                        Err(error) => view.teams.fail_delete(error),
                    }
                    cx.notify();
                });
            })
            .detach();
        } else if let Err(error) = self.teams.confirm_delete() {
            self.teams.feedback = Some(error);
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
            if let Some(config) = editor.team.mirrorConfig.as_mut() {
                if let Some(code) = code {
                    config.team_code = code;
                }
                if let Some(value) = keyword_refresh {
                    config.max_keyword_refresh = value.min(10);
                }
                if let Some(value) = normal_refresh {
                    config.max_normal_refresh = value.min(10);
                }
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

fn resolve_saved_team_from_list(
    value: serde_json::Value,
    submitted: &TeamDetail,
) -> Result<TeamDetail, String> {
    let teams: Vec<TeamDetail> = serde_json::from_value(value)
        .map_err(|error| format!("team.list 返回了无效队伍：{error}"))?;
    let exact = teams
        .iter()
        .into_iter()
        .find(|team| {
            team.id.as_str() == submitted.id.as_str()
                || (submitted.id.is_empty()
                    && team.name == submitted.name
                    && team.purpose == submitted.purpose
                    && team.sinners == submitted.sinners)
        })
        .cloned();
    exact
        .or_else(|| {
            teams.into_iter().rev().find(|team| {
                submitted.id.is_empty()
                    && team.name == submitted.name
                    && team.purpose == submitted.purpose
            })
        })
        .ok_or_else(|| "team.save 已成功，但无法从列表定位队伍".to_owned())
}
