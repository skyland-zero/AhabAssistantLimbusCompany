use super::*;
use gpui::{WeakEntity, Window};
use gpui_component::Root;

use crate::components::{Tab, segmented_tab_bar};

use crate::components::IconName;

/// The team editor body, rendered from a live handle on the app.
///
/// Same constraint as the other `Root` bodies: a dialog builder runs inside
/// `AhabApp::render` and cannot read the app, while the editor's tabs, switches
/// and inputs all have to show live state. A child view is laid out after that
/// borrow ends, so it can.
///
/// It also takes the dialog down by itself when the editor disappears: a save
/// clears `teams.editor` from an async completion, and that completion has no
/// window to close a dialog with.
pub(crate) struct TeamEditorView {
    root: WeakEntity<AhabApp>,
    /// Never dropped: it is what repaints the tabs, the switches and the inputs.
    _app_events: gpui::Subscription,
}

impl TeamEditorView {
    pub(crate) fn new(root: gpui::Entity<AhabApp>, cx: &mut Context<Self>) -> Self {
        let app_events = cx.observe(&root, |_, _, cx| cx.notify());
        Self {
            root: root.downgrade(),
            _app_events: app_events,
        }
    }
}

impl Render for TeamEditorView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let Some(root) = self.root.upgrade() else {
            return div().into_any_element();
        };

        let (has_editor, dialog_open) = {
            let app = root.read(cx);
            (app.teams.editor.is_some(), app.teams.editor_dialog_open)
        };
        if !has_editor {
            // `editor_dialog_open` is what keeps this idempotent: `on_close` has
            // already taken the dialog down in that path, and `close_dialog`
            // pops whatever is on top rather than a named dialog.
            if dialog_open {
                // Deferred because this render is running inside `Root`'s layer
                // build; updating `Root` here would re-enter it.
                cx.defer_in(window, |_, window, cx| {
                    Root::update(window, cx, |root, window, cx| root.close_dialog(window, cx));
                });
            }
            return div().into_any_element();
        }

        let app = root.read(cx);
        let Some(editor) = app.teams.editor.as_ref() else {
            return div().into_any_element();
        };
        let language = app.state.settings.language;
        let team = editor.team.clone();
        let tab = if editor.tab.is_available(team.purpose) {
            editor.tab
        } else {
            TeamEditorTab::Basic
        };
        let config = editor.mirror_config();
        let feedback = app.teams.feedback.clone();
        let starlight_cost = app.teams.starlight_cost();
        let palette = current_render_palette();

        // Only the tabs this purpose offers are shown, so the bar is built from
        // the filtered list and the click handler maps the index back through it.
        let available: Vec<TeamEditorTab> = TeamEditorTab::ALL
            .into_iter()
            .filter(|candidate| candidate.is_available(team.purpose))
            .collect();
        let index = available.iter().position(|c| *c == tab).unwrap_or(0);
        let tabs: Vec<Tab> = available
            .iter()
            .map(|candidate| {
                let mut element = Tab::new().label(editor_tab_label(*candidate, language));
                if *candidate == TeamEditorTab::Starlight && starlight_cost > 0 {
                    element = element.suffix(
                        div()
                            .px_1()
                            .rounded_md()
                            .bg(palette_rgb(current_render_palette().brand_light))
                            .text_size(px(10.))
                            .text_color(palette_rgb(current_render_palette().brand))
                            .child(starlight_cost.to_string()),
                    );
                }
                element
            })
            .collect();
        let tabs = div().child(
            segmented_tab_bar("team-editor-tabs", index)
                .on_click(app_listener(
                    &self.root,
                    move |view, index: &usize, _, cx| {
                        if let Some(candidate) = available.get(*index) {
                            view.teams.set_editor_tab(*candidate);
                            cx.notify();
                        }
                    },
                ))
                .children(tabs),
        );

        let mut copy = div()
            .id("team-copy-json")
            .flex()
            .items_center()
            .justify_center()
            .gap_1()
            .h(px(28.))
            .px_2()
            .rounded_md()
            .tab_index(0)
            .border_1()
            .border_color(palette_rgb(palette.input))
            .cursor_pointer()
            .focus_visible(|style| style.border_color(palette_rgb(current_render_palette().ring)))
            .hover(|style| style.bg(palette_rgb(current_render_palette().accent_surface)))
            .text_size(px(11.))
            .text_color(palette_rgb(palette.brand))
            .child(icon(IconName::Copy, 14., palette.brand))
            .child(text("复制 JSON", "Copy JSON").get(language))
            .on_click(app_listener(&self.root, |view, _, window, cx| {
                view.copy_team_json(window, cx)
            }));
        copy = copy.on_key_down(app_listener(
            &self.root,
            |view, event: &KeyDownEvent, window, cx| {
                if team_activation_key(event) {
                    window.prevent_default();
                    view.copy_team_json(window, cx);
                }
            },
        ));

        let mut close = button(
            "team-editor-cancel",
            text("取消", "Cancel").get(language),
            ButtonVariant::Ghost,
        )
        .h(px(32.))
        .px_3()
        .py_0()
        .on_click(app_listener(&self.root, |view, _, _, cx| {
            view.close_team_editor(cx)
        }));
        close = close.on_key_down(app_listener(
            &self.root,
            |view, event: &KeyDownEvent, window, cx| {
                if team_activation_key(event) {
                    window.prevent_default();
                    view.close_team_editor(cx);
                }
            },
        ));
        let current_name = app
            .team_inputs
            .name
            .as_ref()
            .map(|input| input.read(cx).text(cx))
            .unwrap_or_else(|| team.name.clone());
        let can_save = !current_name.trim().is_empty() && !app.teams.saving;
        let mut save = button(
            "team-editor-save",
            text("保存队伍", "Save Team").get(language),
            ButtonVariant::Default,
        )
        .h(px(32.))
        .px_3()
        .py_0();
        if can_save {
            save = save
                .on_click(app_listener(&self.root, |view, _, window, cx| {
                    view.save_team_editor(window, cx)
                }))
                .on_key_down(app_listener(
                    &self.root,
                    |view, event: &KeyDownEvent, window, cx| {
                        if team_activation_key(event) {
                            window.prevent_default();
                            view.save_team_editor(window, cx);
                        }
                    },
                ));
        } else {
            save = save.opacity(0.45).cursor_default();
        }

        let content = match tab {
            TeamEditorTab::Basic => {
                editors::basic_editor(&self.root, app, &team, &config, language)
            }
            TeamEditorTab::Shop => editors::shop_editor(&self.root, app, &config, language),
            TeamEditorTab::Combat => editors::combat_editor(&self.root, app, &config, language),
            TeamEditorTab::Starlight => {
                editors::starlight_editor(&self.root, app, &config, language)
            }
            TeamEditorTab::Advanced => div()
                .flex()
                .flex_col()
                .gap_4()
                .child(editors::advanced_editor(&self.root, app, &config, language))
                .child(editors::team_stats_editor(&self.root, app, language)),
        };

        // The height is pinned so the dialog does not resize when the tab does;
        // the content scrolls inside it, which is what the page overlay did.
        div()
            .flex()
            .flex_col()
            .w_full()
            .h(px(520.))
            .min_h_0()
            .child(
                div()
                    .flex_none()
                    .pb_2()
                    // The dialog's own padding is the only horizontal inset.
                    .child(tabs),
            )
            .child(
                scroll_area_with_id("team-editor-scroll", content)
                    .flex_1()
                    .min_h_0()
                    .px_3()
                    .py_4()
                    .rounded_md()
                    .bg(palette_rgb(palette.background)),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .gap_3()
                    .flex_none()
                    .pt_3()
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_3()
                            .flex_1()
                            .min_w_0()
                            .child(
                                feedback
                                    .map(|message| {
                                        div()
                                            .min_w_0()
                                            .text_size(px(11.))
                                            .text_color(palette_rgb(
                                                current_render_palette().warning,
                                            ))
                                            .child(localized_feedback(&message, language))
                                    })
                                    .unwrap_or_else(|| {
                                        div()
                                            .min_w_0()
                                            .text_size(px(11.))
                                            .text_color(palette_rgb(palette.muted_foreground))
                                            .child(text(
                                                "支持中文输入、剪贴板和 JSON 导入",
                                                "Chinese input, clipboard and JSON import are supported",
                                            )
                                            .get(language))
                                    }),
                            )
                            .child(copy),
                    )
                    .child(div().flex().flex_none().gap_2().child(close).child(save)),
            )
            .into_any_element()
    }
}
