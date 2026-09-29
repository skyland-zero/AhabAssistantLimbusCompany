use gpui::{AppContext, ClipboardItem, Context, ParentElement as _, Styled as _, Window, div};
use gpui_component::{Root, WindowExt as _};

use super::AhabApp;
use crate::{
    app_inputs::TeamInputs,
    components::TextInput,
    ipc::{RpcGateway, contract::method},
    model::TeamDetail,
};

impl AhabApp {
    pub fn open_new_team(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.teams.open_new();
        self.create_team_inputs(window, cx);
        self.show_team_editor(window, cx);
    }

    pub fn open_existing_team(
        &mut self,
        team: &TeamDetail,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.teams.open_edit(team);
        self.create_team_inputs(window, cx);
        self.refresh_team_stats(cx);
        self.show_team_editor(window, cx);
    }

    pub fn open_new_team_for_slot(
        &mut self,
        number: u32,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.teams.open_new_for_slot(number);
        self.create_team_inputs(window, cx);
        self.show_team_editor(window, cx);
    }

    /// Shows the team editor as a `Root` dialog.
    ///
    /// The dialog supplies the centering, the close button, the focus trap and
    /// Esc, all of which the page overlay used to reimplement. Its body is
    /// `TeamEditorView`: a dialog builder runs inside `AhabApp::render` and
    /// cannot read the app, while the editor's tabs, switches and inputs all
    /// render live state.
    ///
    /// Split from the three `open_*_team` methods because a capture has to push
    /// a confirmation on top of this same dialog in the same turn.
    pub fn show_team_editor(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let language = self.state.settings.language;
        let is_new = self
            .teams
            .editor
            .as_ref()
            .is_none_or(|editor| editor.team.id.is_empty());
        let title = if is_new {
            crate::i18n::paired("新建队伍", "New Team")
        } else {
            crate::i18n::paired("编辑队伍", "Edit Team")
        };
        let subtitle = crate::i18n::paired(
            "保存前所有修改只存在于当前编辑器",
            "Changes stay in this editor until Save",
        );

        self.teams.editor_dialog_open = true;
        let app = cx.entity();
        let view = cx.new(|cx| crate::pages::TeamEditorView::new(app.clone(), cx));
        let close_app = app.downgrade();
        Root::update(window, cx, move |root, window, cx| {
            root.open_dialog(
                // `Fn`, not `FnOnce`: the builder runs again on every render of
                // the dialog layer, so captures are cloned per call.
                move |dialog, _window, _cx| {
                    let close_app = close_app.clone();
                    dialog
                        .title(
                            div()
                                .flex()
                                .flex_col()
                                .gap(gpui::px(1.0))
                                .child(
                                    div()
                                        .text_size(gpui::px(16.))
                                        .font_weight(gpui::FontWeight::SEMIBOLD)
                                        .child(title.get(language)),
                                )
                                .child(
                                    div().text_size(gpui::px(10.)).child(subtitle.get(language)),
                                ),
                        )
                        .w(gpui::px(680.))
                        // Closing through `Root` - the X, Esc or the overlay -
                        // drops the draft, which is what the state setters do.
                        // The dialog is already going away here, so this must
                        // not ask `Root` to close it again.
                        .on_close(move |_, _, cx| {
                            let _ = close_app.update(cx, |view, cx| view.dismiss_team_editor(cx));
                        })
                        .content({
                            let view = view.clone();
                            move |content, _window, _cx| content.child(view.clone())
                        })
                },
                window,
                cx,
            )
        });
        cx.notify();
    }

    pub fn open_team_preset_picker_for_slot(
        &mut self,
        number: u32,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.teams.open_preset_picker_for_slot(number);
        self.clear_team_inputs();
        self.open_team_preset_picker_sheet(window, cx);
        cx.notify();
    }

    pub fn open_team_preset_picker_for_team(
        &mut self,
        team: &TeamDetail,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.teams.open_preset_picker_for_team(team);
        self.clear_team_inputs();
        self.open_team_preset_picker_sheet(window, cx);
        cx.notify();
    }

    /// Opens the built-in preset catalog as a side sheet.
    ///
    /// The picker used to be a centred modal card; as a sheet it keeps the same
    /// two-step flow (`cancel_team_preset_flow` / `select_team_preset`) but the
    /// scrim, the click-outside dismissal and the Escape handling all become the
    /// sheet's job, which is why the hand-written ones are gone.
    fn open_team_preset_picker_sheet(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let language = self.state.settings.language;
        let title = crate::i18n::paired("选择预设编队", "Choose a team preset").get(language);
        // Snapshot taken here, while the app is not being rendered: the sheet's
        // builder runs inside `AhabApp::render`, so it can neither read nor
        // update the app and would panic trying.
        let target_label = self
            .teams
            .preset_picker
            .as_ref()
            .map(|picker| crate::state::preset_target_label(&picker.target, language))
            .unwrap_or_default();
        let entries: Vec<crate::pages::PresetPickerEntry> = self
            .teams
            .presets
            .iter()
            .map(|preset| crate::pages::PresetPickerEntry::new(preset, self, language))
            .collect();
        let app = cx.entity().downgrade();
        let body_app = app.clone();
        let close_app = app.clone();
        window.open_sheet(cx, move |sheet, _window, _cx| {
            let body = crate::pages::preset_picker_body(&entries, body_app.clone(), language);
            sheet
                .title(
                    div().flex().flex_col().gap_1().child(title).child(
                        div()
                            .text_size(gpui::px(11.))
                            .text_color(crate::components::style::palette_rgb(
                                crate::components::style::current_render_palette().muted_foreground,
                            ))
                            .child(target_label.clone()),
                    ),
                )
                .size(gpui::relative(0.55))
                .on_close({
                    // Cloned per call: the sheet builder is `Fn`, so the callback
                    // cannot take the captured handle by move.
                    let close_app = close_app.clone();
                    move |_, _, cx| {
                        let _ = close_app.update(cx, |view, cx| view.cancel_team_preset_flow(cx));
                    }
                })
                .child(body)
        });
    }

    pub fn select_team_preset(
        &mut self,
        preset_id: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.clear_team_inputs();
        match self.teams.select_preset(preset_id) {
            Ok(true) => self.save_team_editor(window, cx),
            // `Ok(false)` means the preset targets an existing team, so
            // `select_preset` parked the pending overwrite in the state and the
            // confirmation has to be shown before it is applied.
            Ok(false) => self.open_preset_overwrite_confirmation(window, cx),
            Err(error) => self.teams.feedback = Some(error),
        }
        cx.notify();
    }

    /// Confirm replacing an existing team with a built-in preset.
    ///
    /// Same shape as the delete and stats-clear confirmations: the flag stays in
    /// the state so the domain layer can validate it, and the dialog only drives
    /// the two transitions.
    pub fn open_preset_overwrite_confirmation(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(overwrite) = self.teams.preset_overwrite.as_ref() else {
            return;
        };
        let target = overwrite.target.clone();
        let preset_name = overwrite
            .preset
            .name
            .get(self.state.settings.language)
            .to_owned();
        let language = self.state.settings.language;
        let target_label = crate::state::preset_target_label(
            &crate::state::TeamPresetTarget::Existing(Box::new(target)),
            language,
        );
        let app = cx.entity().downgrade();
        Root::update(window, cx, move |root, window, cx| {
            root.open_dialog(
                move |dialog, _window, _cx| {
                    dialog
                        .title(
                            crate::i18n::paired("确认覆盖编队？", "Overwrite this team?")
                                .get(language),
                        )
                        .content({
                            let body = match language {
                                crate::model::Language::ZhCn => {
                                    format!("{target_label} 将被预设“{preset_name}”完整覆盖。")
                                }
                                crate::model::Language::EnUs => format!(
                                    "{target_label} will be fully replaced by “{preset_name}”."
                                ),
                            };
                            move |content, _window, _cx| content.child(body.clone())
                        })
                        .footer({
                            let cancel = crate::i18n::paired("取消", "Cancel").get(language);
                            let confirm =
                                crate::i18n::paired("确认覆盖", "Confirm overwrite").get(language);
                            let cancel_app = app.clone();
                            let confirm_app = app.clone();
                            div()
                                .flex()
                                .justify_end()
                                .gap_2()
                                .child(
                                    crate::components::button(
                                        "preset-overwrite-cancel",
                                        cancel,
                                        crate::components::ButtonVariant::Ghost,
                                    )
                                    .on_click(
                                        move |_, window, cx| {
                                            let _ = cancel_app.update(cx, |view, cx| {
                                                view.cancel_team_preset_flow(cx)
                                            });
                                            Root::update(window, cx, |root, window, cx| {
                                                root.close_dialog(window, cx)
                                            });
                                        },
                                    ),
                                )
                                .child(
                                    crate::components::button(
                                        "preset-overwrite-confirm",
                                        confirm,
                                        crate::components::ButtonVariant::Destructive,
                                    )
                                    .on_click(
                                        move |_, window, cx| {
                                            let _ = confirm_app.update(cx, |view, cx| {
                                                view.confirm_team_preset_overwrite(window, cx)
                                            });
                                            Root::update(window, cx, |root, window, cx| {
                                                root.close_dialog(window, cx)
                                            });
                                        },
                                    ),
                                )
                        })
                },
                window,
                cx,
            );
        });
    }

    pub fn cancel_team_preset_flow(&mut self, cx: &mut Context<Self>) {
        self.teams.close_preset_picker();
        self.teams.close_preset_overwrite();
        cx.notify();
    }

    pub fn confirm_team_preset_overwrite(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.clear_team_inputs();
        match self.teams.confirm_preset_overwrite() {
            Ok(true) => self.save_team_editor(window, cx),
            Ok(false) => {}
            Err(error) => self.teams.feedback = Some(error),
        }
        cx.notify();
    }

    pub fn set_team_enabled(&mut self, team: &TeamDetail, enabled: bool, cx: &mut Context<Self>) {
        if self.teams.rpc.is_sidecar() {
            let params = match self.teams.begin_team_enabled(team, enabled) {
                Ok(params) => params,
                Err(error) => {
                    self.teams.feedback = Some(error);
                    cx.notify();
                    return;
                }
            };
            let team_id = team.id.clone();
            let rpc = self.teams.rpc.clone();
            cx.spawn(async move |this, cx| {
                let request = rpc.request_async(method::TEAM_SAVE, Some(params));
                let response = cx
                    .background_executor()
                    .spawn(async move { request.recv().await.ok() })
                    .await;
                let result = match response {
                    None => Err("后端连接已断开".to_owned()),
                    Some(response) => match RpcGateway::decode_response(method::TEAM_SAVE, response) {
                        Err(error) => Err(error.message),
                        Ok(None) => Err("team.save 返回了空结果".to_owned()),
                        Ok(Some(value)) if value.as_bool() == Some(true) => {
                            let list_request = rpc.request_async(method::TEAM_LIST, None);
                            let list_response = cx
                                .background_executor()
                                .spawn(async move { list_request.recv().await.ok() })
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
                                        let teams: Result<Vec<TeamDetail>, _> =
                                            serde_json::from_value(value);
                                        teams
                                            .map_err(|error| format!("team.list 返回了无效队伍：{error}"))
                                            .and_then(|teams| {
                                                teams
                                                    .into_iter()
                                                    .find(|team| team.id == team_id)
                                                    .ok_or_else(|| {
                                                        "team.save 已成功，但无法从列表定位队伍".to_owned()
                                                    })
                                            })
                                    }
                                },
                            }
                        }
                        Ok(Some(value)) => serde_json::from_value(value)
                            .map_err(|error| format!("team.save 返回了无效队伍：{error}")),
                    },
                };
                let _ = this.update(cx, |view, cx| {
                    match result {
                        Ok(saved) => view.teams.apply_team_enabled(saved),
                        Err(error) => view.teams.fail_team_enabled(error),
                    }
                    cx.notify();
                });
            })
            .detach();
        } else if let Err(error) = self.teams.set_team_enabled(team, enabled) {
            self.teams.feedback = Some(error);
        }
        cx.notify();
    }

    /// The editor's Cancel button.
    ///
    /// Refuses while a save is in flight, the way the overlay did - the draft is
    /// what the response is about. The dialog itself is not closed here: the body
    /// takes it down as soon as it sees `editor` is gone, which keeps `Root`'s
    /// "close the top dialog" out of this method's hands.
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

    /// Clears the editor state without touching the dialog layer.
    ///
    /// This is the `on_close` path: `Root` is already taking the dialog down, so
    /// asking it to close again would pop whatever dialog is beneath.
    pub fn dismiss_team_editor(&mut self, cx: &mut Context<Self>) {
        self.teams.editor_dialog_open = false;
        self.teams.close_editor();
        self.clear_team_inputs();
        cx.notify();
    }

    pub fn save_team_editor(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.sync_team_inputs_to_state(window, cx);
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
                    .spawn(async move { request.recv().await.ok() })
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
                                    .spawn(async move { list_request.recv().await.ok() })
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
                    .spawn(async move { request.recv().await.ok() })
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

    /// Ask to delete a team and open the confirmation as a Root dialog.
    ///
    /// The dialog lives in GPUI Kit's `Root` layer instead of the page overlay,
    /// which is what gives it focus trapping, Esc handling and correct stacking
    /// above the team editor. `delete_target` is still set, because the rest of
    /// the team state reads it to know a deletion is pending.
    pub fn open_delete_confirmation(
        &mut self,
        team: TeamDetail,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let name = team.name.clone();
        self.teams.request_delete(team);
        let language = self.state.settings.language;
        let app = cx.entity().downgrade();
        Root::update(window, cx, move |root, window, cx| {
            root.open_dialog(
                // The builder is `Fn`, not `FnOnce`: it can run again whenever
                // the dialog re-renders, so the captures are cloned per call
                // rather than moved out.
                move |dialog, _window, _cx| {
                    let name = name.clone();
                    let cancel_app = app.clone();
                    let confirm_app = app.clone();
                    // `on_ok` / `on_cancel` receive a plain `&mut App` rather than
                    // a `Context<AhabApp>`, so the state write goes through the
                    // weak handle; its `update` hands back the `Context<AhabApp>`
                    // the domain methods need.
                    dialog
                        .title(
                            crate::i18n::paired("确认删除队伍？", "Delete this team?")
                                .get(language),
                        )
                        .content(move |content, _window, _cx| content.child(name.clone()))
                        // `Dialog` renders no OK/Cancel buttons of its own -
                        // `render_ok` / `render_cancel` are only wired up by
                        // `AlertDialog` - so the action row is supplied here.
                        .footer({
                            let cancel = crate::i18n::paired("取消", "Cancel").get(language);
                            let confirm = crate::i18n::paired("删除", "Delete").get(language);
                            let cancel_app = app.clone();
                            let confirm_app = app.clone();
                            div()
                                .flex()
                                .justify_end()
                                .gap_2()
                                .child(
                                    crate::components::button(
                                        "delete-dialog-cancel",
                                        cancel,
                                        crate::components::ButtonVariant::Ghost,
                                    )
                                    .on_click(
                                        move |_, window, cx| {
                                            let _ = cancel_app.update(cx, |view, cx| {
                                                view.teams.cancel_delete();
                                                cx.notify();
                                            });
                                            Root::update(window, cx, |root, window, cx| {
                                                root.close_dialog(window, cx)
                                            });
                                        },
                                    ),
                                )
                                .child(
                                    crate::components::button(
                                        "delete-dialog-confirm",
                                        confirm,
                                        crate::components::ButtonVariant::Destructive,
                                    )
                                    .on_click(
                                        move |_, window, cx| {
                                            let _ = confirm_app
                                                .update(cx, |view, cx| view.confirm_delete(cx));
                                            Root::update(window, cx, |root, window, cx| {
                                                root.close_dialog(window, cx)
                                            });
                                        },
                                    ),
                                )
                        })
                        .on_cancel(move |_, _, cx| {
                            let _ = cancel_app.update(cx, |view, cx| {
                                view.teams.cancel_delete();
                                cx.notify();
                            });
                            true
                        })
                        .on_ok(move |_, _, cx| {
                            let _ = confirm_app.update(cx, |view, cx| view.confirm_delete(cx));
                            true
                        })
                },
                window,
                cx,
            );
        });
    }

    pub fn copy_team_json(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.sync_team_inputs_to_state(window, cx);
        if let Some(json) = self.teams.export_editor_json() {
            cx.write_to_clipboard(ClipboardItem::new_string(json));
            self.teams.feedback = Some("队伍 JSON 已复制".to_owned());
        }
        cx.notify();
    }

    pub fn import_team_json(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let input = self
            .team_inputs
            .json
            .as_ref()
            .map(|entity| entity.read(cx).text(cx))
            .unwrap_or_default();
        match self.teams.import_editor_json(&input) {
            Ok(()) => {
                self.sync_team_inputs_from_state(window, cx);
                if let Some(entity) = self.team_inputs.json.as_ref() {
                    entity.update(cx, |input, cx| input.set_text("", window, cx));
                }
            }
            Err(error) => self.teams.feedback = Some(format!("导入失败：{error}")),
        }
        cx.notify();
    }

    pub fn add_team_observe_gift(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let input = self
            .team_inputs
            .observe
            .as_ref()
            .map(|entity| entity.read(cx).text(cx))
            .unwrap_or_default();
        if self.teams.add_observe_gift(&input)
            && let Some(entity) = self.team_inputs.observe.as_ref()
        {
            entity.update(cx, |input, cx| input.set_text("", window, cx));
        }
        cx.notify();
    }

    pub(crate) fn create_team_inputs(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.team_inputs.subscriptions.clear();
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
            crate::model::Language::ZhCn => "高级坐标，如 general_3_4_8",
            crate::model::Language::EnUs => "Advanced coordinate, e.g. general_3_4_8",
        };
        let json_placeholder = match language {
            crate::model::Language::ZhCn => "粘贴 Team JSON",
            crate::model::Language::EnUs => "Paste Team JSON",
        };
        self.team_inputs.name = Some(cx.new({
            let window = &mut *window;
            move |cx| TextInput::new(name, name_placeholder, window, cx)
        }));
        self.team_inputs.code = Some(cx.new({
            let window = &mut *window;
            move |cx| TextInput::new(code, code_placeholder, window, cx)
        }));
        self.team_inputs.observe = Some(cx.new({
            let window = &mut *window;
            move |cx| TextInput::new("", observe_placeholder, window, cx)
        }));
        self.team_inputs.json = Some(cx.new({
            let window = &mut *window;
            move |cx| TextInput::new("", json_placeholder, window, cx)
        }));
        let keyword_input = cx.new({
            let window = &mut *window;
            move |cx| TextInput::new(keyword_refresh, "0-10", window, cx)
        });
        let keyword_subscription =
            cx.observe_in(&keyword_input, window, |view, input, window, cx| {
                if let Ok(value) = input.read(cx).text(cx).parse::<u8>() {
                    let value = value.min(10);
                    view.teams
                        .set_mirror_u8(crate::state::MirrorU8::MaxKeywordRefresh, value);
                    input.update(cx, |input, cx| {
                        input.set_text(value.to_string(), window, cx)
                    });
                    cx.notify();
                }
            });
        self.team_inputs.keyword_refresh = Some(keyword_input);
        self.team_inputs.subscriptions.push(keyword_subscription);

        let normal_input = cx.new({
            let window = &mut *window;
            move |cx| TextInput::new(normal_refresh, "0-10", window, cx)
        });
        let normal_subscription =
            cx.observe_in(&normal_input, window, |view, input, window, cx| {
                if let Ok(value) = input.read(cx).text(cx).parse::<u8>() {
                    let value = value.min(10);
                    view.teams
                        .set_mirror_u8(crate::state::MirrorU8::MaxNormalRefresh, value);
                    input.update(cx, |input, cx| {
                        input.set_text(value.to_string(), window, cx)
                    });
                    cx.notify();
                }
            });
        self.team_inputs.normal_refresh = Some(normal_input);
        self.team_inputs.subscriptions.push(normal_subscription);
    }

    fn clear_team_inputs(&mut self) {
        self.team_inputs = TeamInputs::default();
    }

    fn sync_team_inputs_to_state(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        let name = self
            .team_inputs
            .name
            .as_ref()
            .map(|input| input.read(cx).text(cx));
        let code = self
            .team_inputs
            .code
            .as_ref()
            .map(|input| input.read(cx).text(cx));
        let keyword_refresh = self
            .team_inputs
            .keyword_refresh
            .as_ref()
            .and_then(|input| input.read(cx).text(cx).parse::<u8>().ok());
        let normal_refresh = self
            .team_inputs
            .normal_refresh
            .as_ref()
            .and_then(|input| input.read(cx).text(cx).parse::<u8>().ok());
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

    fn sync_team_inputs_from_state(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(editor) = self.teams.editor.as_ref() else {
            return;
        };
        let name = editor.team.name.clone();
        let mirror_config = editor.mirror_config();
        let code = mirror_config.team_code;
        let keyword_refresh = mirror_config.max_keyword_refresh.to_string();
        let normal_refresh = mirror_config.max_normal_refresh.to_string();
        if let Some(input) = self.team_inputs.name.as_ref() {
            input.update(cx, |input, cx| input.set_text(name, window, cx));
        }
        if let Some(input) = self.team_inputs.code.as_ref() {
            input.update(cx, |input, cx| input.set_text(code, window, cx));
        }
        if let Some(input) = self.team_inputs.keyword_refresh.as_ref() {
            input.update(cx, |input, cx| input.set_text(keyword_refresh, window, cx));
        }
        if let Some(input) = self.team_inputs.normal_refresh.as_ref() {
            input.update(cx, |input, cx| input.set_text(normal_refresh, window, cx));
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
