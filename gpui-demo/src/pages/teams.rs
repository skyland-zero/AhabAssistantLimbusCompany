//! Teams list and the five-section native team editor (P5).
//!
//! The React page used a modal with a large form.  GPUI has no browser dialog
//! dependency here, so the same interaction is rendered as an application
//! overlay.  All changes are kept in `TeamsState` until Save is pressed.

use std::rc::Rc;

use gpui::{Context, Div, ImageSource, KeyDownEvent, div, img, prelude::*, px, relative, svg};

use crate::{
    app::{AhabApp, BORDER, SURFACE, TEXT, TEXT_MUTED},
    assets::{self, Asset, SinnerAsset, StatusEffectAsset},
    components::style::{ColorToken, current_render_palette},
    components::{
        BadgeTone, ButtonVariant, badge, button, card, empty_state, palette_rgb, render_rgb as rgb,
        render_rgba as rgba, scroll_area_with_id, select_option, select_popup, select_trigger,
        switch,
    },
    model::{Language, TeamDetail, TeamMirrorConfig, TeamPurpose},
    state::{
        MirrorBool, MirrorU8, SYSTEM_NAMES, TeamEditorTab, TeamFilter, TeamSelect, TeamsState,
    },
};

const STARLIGHT_NAMES: [&str; 10] = [
    "初始之星",
    "积聚的星云",
    "星际漫游",
    "流星",
    "双星商店",
    "卫星商店",
    "星云的宠爱",
    "星芒的引导",
    "偶然的彗星",
    "全面的可能性",
];
const STARLIGHT_COSTS: [u32; 10] = [10, 10, 20, 20, 30, 30, 40, 40, 50, 60];
const STARLIGHT_DESCRIPTIONS: [&str; 10] = [
    "初始经费增加，卡包/饰品展出+1，免费普通刷新",
    "进阶经费利息+10%~30%，售卖饰品经费加成",
    "卡包出现+1，卡包刷新+2~4，未记录卡包等级提升",
    "初始经费+400~700，初始饰品可选择数+1",
    "展出饰品+1，战斗经费+20%~40%，高阶饰品概率提升",
    "免费关键词刷新，进入第1层送1~3件1级饰品",
    "进入第1层人格等级+3，通关阶段人格等级提升",
    "最大速度+2~3，拼点威力、伤害强化和守护提升",
    "进商店赠送合成/售卖专用饰品，赠送对应关键词饰品",
    "开局自选3级饰品，获得残影饰品",
];
const STARLIGHT_NAMES_EN: [&str; 10] = [
    "Initial Star",
    "Gathered Nebula",
    "Star Wanderer",
    "Meteor",
    "Binary Star Shop",
    "Satellite Shop",
    "Nebula's Favor",
    "Starlight Guide",
    "Chance Comet",
    "All Possibilities",
];
const STARLIGHT_DESCRIPTIONS_EN: [&str; 10] = [
    "Increase starting cost, gift displays by 1, and grant a free normal refresh",
    "Increase advanced-cost interest and gift sale cost bonus",
    "Add one gift pack, refreshes, and unrecorded pack upgrades",
    "Increase starting cost and the number of starting gifts to choose",
    "Add one displayed gift and improve battle cost and high-tier odds",
    "Grant a free keyword refresh and 1-3 level-1 gifts on floor 1",
    "Raise sinner levels on floor 1 and after clearing stages",
    "Improve maximum speed, clash power, damage and protection",
    "Grant shop gifts for fusion/sales and matching keywords",
    "Choose a level-3 gift at start and receive a remnant gift",
];
const SYSTEM_LABELS_EN: [&str; 10] = [
    "Burn", "Bleed", "Tremor", "Rupture", "Sinking", "Poise", "Charge", "Slash", "Pierce", "Blunt",
];

// Small inline vectors keep the controls recognisable without substituting
// text glyphs for the Lucide actions used by the React page.
const ICON_PLUS: &[u8] = br##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round"><path d="M12 5v14M5 12h14"/></svg>"##;
const ICON_EDIT: &[u8] = br##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M12 20h9"/><path d="M16.5 3.5a2.12 2.12 0 0 1 3 3L7 19l-4 1 1-4Z"/></svg>"##;
const ICON_TRASH: &[u8] = br##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M3 6h18"/><path d="M8 6V4h8v2"/><path d="M19 6v14H5V6"/><path d="M10 11v5M14 11v5"/></svg>"##;
const ICON_COPY: &[u8] = br##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect x="9" y="9" width="11" height="11" rx="2"/><path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"/></svg>"##;
const ICON_PASTE: &[u8] = br##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M16 4h2a2 2 0 0 1 2 2v14a2 2 0 0 1-2 2H6a2 2 0 0 1-2-2V6a2 2 0 0 1 2-2h2"/><rect x="8" y="2" width="8" height="4" rx="1"/><path d="m9 14 2 2 4-4"/></svg>"##;
const ICON_CLOSE: &[u8] = br##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round"><path d="M6 6l12 12M18 6 6 18"/></svg>"##;
const ICON_SPARKLES: &[u8] = br##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="m12 3-1.3 5.7L5 10l5.7 1.3L12 17l1.3-5.7L19 10l-5.7-1.3Z"/><path d="m19 16-.7 3.3L15 20l3.3.7L19 24l.7-3.3L23 20l-3.3-.7Z"/></svg>"##;
const ICON_USERS: &[u8] = br##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"><path d="M16 21v-2a4 4 0 0 0-4-4H6a4 4 0 0 0-4 4v2"/><circle cx="9" cy="7" r="4"/><path d="M22 21v-2a4 4 0 0 0-3-3.87M16 3.13a4 4 0 0 1 0 7.75"/></svg>"##;

#[derive(Clone, Copy)]
struct Localized {
    zh: &'static str,
    en: &'static str,
}

impl Localized {
    fn get(self, language: Language) -> &'static str {
        match language {
            Language::ZhCn => self.zh,
            Language::EnUs => self.en,
        }
    }
}

const fn text(zh: &'static str, en: &'static str) -> Localized {
    Localized { zh, en }
}

fn icon(data: &'static [u8], size: f32, color: ColorToken) -> impl IntoElement {
    svg()
        .data(data)
        .w(px(size))
        .h(px(size))
        .text_color(palette_rgb(color))
}

pub fn render(app: &mut AhabApp, cx: &mut Context<AhabApp>) -> Div {
    let language = app.state.settings.language;
    let filter = app.teams.filter;
    let teams: Vec<TeamDetail> = app.teams.filtered_teams().cloned().collect();

    // The React TabsList is a compact, muted strip. Keeping the count as a
    // separate pill makes zero-count purpose tabs match the source behavior.
    let mut filter_bar = div()
        .flex()
        .items_center()
        .gap_1()
        .flex_wrap()
        .p(px(2.))
        .rounded_md()
        .bg(palette_rgb(current_render_palette().muted));
    for candidate in TeamFilter::ALL {
        let active = candidate == filter;
        let count = app.teams.count_for(candidate);
        let mut control = div()
            .id(format!("team-filter-{candidate:?}"))
            .flex()
            .items_center()
            .gap_1()
            .h(px(28.))
            .px_3()
            .rounded_md()
            .tab_index(0)
            .cursor_pointer()
            .text_size(px(12.))
            .text_color(palette_rgb(if active {
                current_render_palette().foreground
            } else {
                current_render_palette().muted_foreground
            }))
            .bg(palette_rgb(if active {
                current_render_palette().card
            } else {
                current_render_palette().muted
            }));
        control = control.child(filter_label(candidate, language));
        if candidate == TeamFilter::All || count > 0 {
            control = control.child(
                div()
                    .px_1()
                    .rounded_md()
                    .bg(palette_rgb(current_render_palette().secondary))
                    .text_size(px(10.))
                    .text_color(palette_rgb(current_render_palette().muted_foreground))
                    .child(count.to_string()),
            );
        }
        control = control
            .on_click(cx.listener(move |view, _, _, cx| {
                view.teams.set_filter(candidate);
                cx.notify();
            }))
            .on_key_down(cx.listener(move |view, event: &KeyDownEvent, window, cx| {
                if team_activation_key(event) {
                    window.prevent_default();
                    view.teams.set_filter(candidate);
                    cx.notify();
                }
            }));
        filter_bar = filter_bar.child(control);
    }

    let mut new_team = div()
        .id("new-team")
        .flex()
        .items_center()
        .justify_center()
        .gap_1()
        .h(px(32.))
        .px_3()
        .rounded_md()
        .tab_index(0)
        .cursor_pointer()
        .bg(rgb(crate::app::ACCENT))
        .text_size(px(12.))
        .text_color(palette_rgb(current_render_palette().brand_foreground))
        .focus_visible(|style| style.border_color(palette_rgb(current_render_palette().ring)))
        .child(icon(
            ICON_PLUS,
            14.,
            current_render_palette().brand_foreground,
        ))
        .child(text("新建队伍", "New Team").get(language))
        .on_click(cx.listener(|view, _, _, cx| view.open_new_team(cx)));
    new_team = new_team.on_key_down(cx.listener(|view, event: &KeyDownEvent, window, cx| {
        if team_activation_key(event) {
            window.prevent_default();
            view.open_new_team(cx);
        }
    }));

    let mut cards = div().flex().flex_wrap().gap_3().items_stretch();
    if app.teams.teams.is_empty() {
        cards = cards.child(
            empty_state(
                text("还没有队伍", "No teams yet").get(language),
                text(
                    "创建一支队伍开始配置镜牢策略。",
                    "Create a team to configure Mirror Dungeon strategies.",
                )
                .get(language),
            )
            .w_full()
            .min_h(px(240.)),
        );
    } else if teams.is_empty() {
        cards = cards.child(
            empty_state(
                text("该分类没有队伍", "No teams in this category").get(language),
                text(
                    "切换分类或创建一支新队伍。",
                    "Switch category or create a new team.",
                )
                .get(language),
            )
            .w_full()
            .min_h(px(240.)),
        );
    } else {
        // A flex basis gives us the same one-column/sufficient-width-two-column
        // behavior as xl:grid-cols-2 without requiring a second render API
        // that exposes the window size.
        for team in teams {
            cards = cards.child(
                team_card(app, cx, team, language)
                    .flex_basis(px(600.))
                    .flex_grow(1.)
                    .flex_shrink(1.),
            );
        }
    }

    let mut root = div().flex().flex_col().flex_1().min_h_0();
    root = root.child(
        div()
            .flex()
            .items_center()
            .justify_between()
            .gap_3()
            .flex_wrap()
            .flex_none()
            .px_4()
            .py_2()
            .bg(palette_rgb(current_render_palette().card))
            .child(filter_bar)
            .child(new_team),
    );
    if let Some(feedback) = app.teams.feedback.clone() {
        root = root.child(
            div()
                .flex_none()
                .mx_4()
                .mt_2()
                .px_3()
                .py_2()
                .rounded_md()
                .bg(palette_rgb(current_render_palette().success_light))
                .text_size(px(11.))
                .text_color(palette_rgb(current_render_palette().success))
                .child(localized_feedback(&feedback, language)),
        );
    }
    root = root.child(
        scroll_area_with_id("teams-list-scroll", div().p_4().child(cards))
            .flex_1()
            .min_h_0(),
    );
    root.on_any_mouse_down(cx.listener(|view, _, _, cx| {
        if view.teams.open_select.is_some() {
            view.teams.close_select();
            cx.notify();
        }
    }))
}

fn team_card(
    app: &mut AhabApp,
    cx: &mut Context<AhabApp>,
    team: TeamDetail,
    language: Language,
) -> Div {
    let config = team.mirrorConfig.clone().unwrap_or_default();
    let discarded = discard_count(&config);
    let has_starlight = config.opening_bonus.iter().any(|level| *level > 0);
    let team_id = team.id.clone();
    let edit_team = team.clone();
    let edit_team_for_key = edit_team.clone();
    let delete_team = team.clone();
    let delete_team_for_key = delete_team.clone();

    let mut edit = div()
        .id(format!("edit-team-{team_id}"))
        .flex()
        .items_center()
        .justify_center()
        .size(px(32.))
        .rounded_md()
        .tab_index(0)
        .cursor_pointer()
        .focus_visible(|style| style.border_color(palette_rgb(current_render_palette().ring)))
        .child(icon(
            ICON_EDIT,
            16.,
            current_render_palette().muted_foreground,
        ))
        .on_click(cx.listener(move |view, _, _, cx| {
            view.open_existing_team(&edit_team, cx);
        }));
    edit = edit.on_key_down(cx.listener(move |view, event: &KeyDownEvent, window, cx| {
        if team_activation_key(event) {
            window.prevent_default();
            view.open_existing_team(&edit_team_for_key, cx);
        }
    }));
    let mut delete = div()
        .id(format!("delete-team-{team_id}"))
        .flex()
        .items_center()
        .justify_center()
        .size(px(32.))
        .rounded_md()
        .tab_index(0)
        .cursor_pointer()
        .focus_visible(|style| style.border_color(palette_rgb(current_render_palette().ring)))
        .child(icon(ICON_TRASH, 16., current_render_palette().danger))
        .on_click(cx.listener(move |view, _, _, cx| {
            view.teams.request_delete(delete_team.clone());
            cx.notify();
        }));
    delete = delete.on_key_down(cx.listener(move |view, event: &KeyDownEvent, window, cx| {
        if team_activation_key(event) {
            window.prevent_default();
            view.teams.request_delete(delete_team_for_key.clone());
            cx.notify();
        }
    }));

    let mut sinner_badges = div().flex().flex_wrap().gap_1();
    for (index, sinner) in team.sinners.iter().enumerate() {
        sinner_badges = sinner_badges.child(badge(
            format!("#{} {}", index + 1, app.teams.sinner_name(sinner)),
            BadgeTone::Neutral,
        ));
    }

    let scheme = normalized_scheme(&team.accessoryScheme);
    let header = div()
        .flex()
        .items_center()
        .gap_2()
        .flex_wrap()
        .child(
            div()
                .min_w_0()
                .text_size(px(14.))
                .text_color(rgb(TEXT))
                .child(team.name.clone()),
        )
        .child(badge(
            purpose_label(team.purpose, language),
            BadgeTone::Neutral,
        ))
        .child(scheme_badge(scheme, language));
    let header = if team.enabled {
        header
    } else {
        header.child(badge(
            text("已停用", "Disabled").get(language),
            BadgeTone::Neutral,
        ))
    };

    let mut details = div()
        .flex()
        .items_center()
        .flex_wrap()
        .gap_2()
        .text_size(px(11.))
        .text_color(rgb(TEXT_MUTED))
        .child(if matches!(language, Language::ZhCn) {
            format!("{} 人格", team.sinners.len())
        } else {
            format!("{} sinners", team.sinners.len())
        });
    if has_starlight {
        details = details.child(
            div()
                .flex()
                .items_center()
                .gap_1()
                .text_color(rgb(crate::app::ACCENT))
                .child(icon(ICON_SPARKLES, 13., current_render_palette().brand))
                .child(text("已配星光", "Starlight ready").get(language)),
        );
    }
    if config.second_system {
        details = details.child(badge(
            text("第二体系", "2nd system").get(language),
            BadgeTone::Neutral,
        ));
    }
    if discarded > 0 {
        details = details.child(badge(
            if matches!(language, Language::ZhCn) {
                format!("舍弃 {} 项", discarded)
            } else {
                format!("Discard ×{}", discarded)
            },
            BadgeTone::Danger,
        ));
    }
    if config.defense_for_solo {
        details = details.child(badge(
            text("良秀单通", "Solo pass").get(language),
            BadgeTone::Accent,
        ));
    }
    if config.use_team_code {
        details = details.child(badge(
            text("编队码", "Team code").get(language),
            BadgeTone::Neutral,
        ));
    }

    card(
        div()
            .flex()
            .items_start()
            .gap_3()
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .child(header)
                    .child(details)
                    .child(sinner_badges),
            )
            .child(div().flex().flex_none().gap_1().child(edit).child(delete)),
    )
    .p_3()
    .opacity(if team.enabled { 1. } else { 0.6 })
    .hover(|style| style.bg(palette_rgb(current_render_palette().secondary)))
}

pub fn render_overlay(app: &mut AhabApp, cx: &mut Context<AhabApp>) -> Div {
    let language = app.state.settings.language;
    let Some(editor) = app.teams.editor.as_ref() else {
        if app.teams.delete_target.is_none() {
            return div();
        }

        let mut surface = div()
            .id("team-delete-overlay")
            .relative()
            .size_full()
            .flex()
            .items_center()
            .justify_center()
            .p_4()
            .bg(rgba(0x080c14d9));
        surface = surface.on_click(cx.listener(|view, _, _, cx| {
            view.teams.cancel_delete();
            cx.notify();
        }));
        surface =
            surface.capture_key_down(cx.listener(|view, event: &KeyDownEvent, window, cx| {
                if event.keystroke.key.eq_ignore_ascii_case("escape") {
                    window.prevent_default();
                    cx.stop_propagation();
                    view.teams.cancel_delete();
                    cx.notify();
                }
            }));
        return div()
            .absolute()
            .top_0()
            .left_0()
            .right_0()
            .bottom_0()
            .child(surface.child(delete_confirmation(app, cx, language)));
    };

    let team = editor.team.clone();
    let tab = editor.tab;
    let config = editor.mirror_config();
    let feedback = app.teams.feedback.clone();
    let starlight_cost = app.teams.starlight_cost();

    let mut tabs = div()
        .flex()
        .items_center()
        .gap_1()
        .flex_wrap()
        .p(px(2.))
        .rounded_md()
        .bg(palette_rgb(current_render_palette().muted));
    for candidate in TeamEditorTab::ALL {
        let active = candidate == tab;
        let mut control = div()
            .id(format!("team-editor-tab-{candidate:?}"))
            .flex()
            .items_center()
            .justify_center()
            .gap_1()
            .h(px(28.))
            .px_3()
            .rounded_md()
            .tab_index(0)
            .cursor_pointer()
            .text_size(px(12.))
            .text_color(palette_rgb(if active {
                current_render_palette().foreground
            } else {
                current_render_palette().muted_foreground
            }))
            .bg(palette_rgb(if active {
                current_render_palette().card
            } else {
                current_render_palette().muted
            }));
        control = control.child(editor_tab_label(candidate, language));
        if candidate == TeamEditorTab::Starlight && starlight_cost > 0 {
            control = control.child(
                div()
                    .px_1()
                    .rounded_md()
                    .bg(palette_rgb(current_render_palette().brand_light))
                    .text_size(px(10.))
                    .text_color(palette_rgb(current_render_palette().brand))
                    .child(starlight_cost.to_string()),
            );
        }
        control = control.on_click(cx.listener(move |view, _, _, cx| {
            view.teams.set_editor_tab(candidate);
            cx.notify();
        }));
        control =
            control.on_key_down(cx.listener(move |view, event: &KeyDownEvent, window, cx| {
                if team_activation_key(event) {
                    window.prevent_default();
                    view.teams.set_editor_tab(candidate);
                    cx.notify();
                }
            }));
        tabs = tabs.child(control);
    }

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
        .border_color(rgb(BORDER))
        .cursor_pointer()
        .focus_visible(|style| style.border_color(palette_rgb(current_render_palette().ring)))
        .text_size(px(11.))
        .text_color(rgb(crate::app::ACCENT))
        .child(icon(ICON_COPY, 14., current_render_palette().brand))
        .child(text("复制 JSON", "Copy JSON").get(language))
        .on_click(cx.listener(|view, _, _, cx| view.copy_team_json(cx)));
    copy = copy.on_key_down(cx.listener(|view, event: &KeyDownEvent, window, cx| {
        if team_activation_key(event) {
            window.prevent_default();
            view.copy_team_json(cx);
        }
    }));

    let mut close = button(text("取消", "Cancel").get(language), ButtonVariant::Ghost)
        .id("team-editor-cancel")
        .h(px(32.))
        .px_3()
        .py_0()
        .on_click(cx.listener(|view, _, _, cx| view.close_team_editor(cx)));
    close = close.on_key_down(cx.listener(|view, event: &KeyDownEvent, window, cx| {
        if team_activation_key(event) {
            window.prevent_default();
            view.close_team_editor(cx);
        }
    }));
    let current_name = app
        .team_name_input
        .as_ref()
        .map(|input| input.read(cx).text())
        .unwrap_or_else(|| team.name.clone());
    let can_save = !current_name.trim().is_empty();
    let mut save = button(
        text("保存队伍", "Save Team").get(language),
        ButtonVariant::Default,
    )
    .id("team-editor-save")
    .h(px(32.))
    .px_3()
    .py_0();
    if can_save {
        save = save
            .on_click(cx.listener(|view, _, _, cx| view.save_team_editor(cx)))
            .on_key_down(cx.listener(|view, event: &KeyDownEvent, window, cx| {
                if team_activation_key(event) {
                    window.prevent_default();
                    view.save_team_editor(cx);
                }
            }));
    } else {
        save = save.opacity(0.45).cursor_default();
    }

    let content = match tab {
        TeamEditorTab::Basic => basic_editor(app, cx, &team, &config, language),
        TeamEditorTab::Shop => shop_editor(app, cx, &config, language),
        TeamEditorTab::Combat => combat_editor(app, cx, &config, language),
        TeamEditorTab::Starlight => starlight_editor(app, cx, &config, language),
        TeamEditorTab::Advanced => advanced_editor(app, cx, &config, language),
    };

    let dialog_body =
        div()
            .id("team-editor-dialog")
            .w(px(768.))
            .h(px(620.))
            .max_w_full()
            .max_h(relative(0.88))
            .min_h_0()
            .overflow_hidden()
            .flex()
            .flex_col()
            .rounded_lg()
            .border_1()
            .border_color(rgb(BORDER))
            .bg(rgb(SURFACE))
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .gap_3()
                    .px_6()
                    .py_3()
                    .border_b_1()
                    .border_color(rgb(BORDER))
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap_1()
                            .child(div().text_size(px(16.)).text_color(rgb(TEXT)).child(
                                if team.id.is_empty() {
                                    text("新建队伍", "New Team").get(language)
                                } else {
                                    text("编辑队伍", "Edit Team").get(language)
                                },
                            ))
                            .child(
                                div().text_size(px(10.)).text_color(rgb(TEXT_MUTED)).child(
                                    text(
                                        "保存前所有修改只存在于当前编辑器",
                                        "Changes stay in this editor until Save",
                                    )
                                    .get(language),
                                ),
                            ),
                    )
                    .child(copy),
            )
            .child(
                div()
                    .flex_none()
                    .px_6()
                    .py_2()
                    .border_b_1()
                    .border_color(rgb(BORDER))
                    .child(tabs),
            )
            .child(
                scroll_area_with_id("team-editor-scroll", content)
                    .flex_1()
                    .min_h_0()
                    .px_6()
                    .py_4(),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .gap_3()
                    .flex_none()
                    .px_6()
                    .py_3()
                    .border_t_1()
                    .border_color(rgb(BORDER))
                    .bg(palette_rgb(current_render_palette().card))
                    .child(
                        feedback
                            .map(|message| {
                                div()
                                    .flex_1()
                                    .min_w_0()
                                    .text_size(px(11.))
                                    .text_color(palette_rgb(current_render_palette().warning))
                                    .child(localized_feedback(&message, language))
                            })
                            .unwrap_or_else(|| {
                                div()
                                .flex_1()
                                .min_w_0()
                                .text_size(px(11.))
                                .text_color(rgb(TEXT_MUTED))
                                .child(text(
                                    "支持中文输入、剪贴板和 JSON 导入",
                                    "Chinese input, clipboard and JSON import are supported",
                                )
                                .get(language))
                            }),
                    )
                    .child(div().flex().flex_none().gap_2().child(close).child(save)),
            )
            .on_click(cx.listener(|_, _, _, cx| cx.stop_propagation()))
            .on_any_mouse_down(cx.listener(|view, _, _, cx| {
                if view.teams.open_select.is_some() {
                    view.teams.close_select();
                    cx.notify();
                }
            }));

    let mut surface = div()
        .id("team-editor-overlay")
        .relative()
        .size_full()
        .flex()
        .items_center()
        .justify_center()
        .p_4()
        .bg(rgba(0x080c14d9));
    surface = surface.on_click(cx.listener(|view, _, _, cx| {
        if view.teams.delete_target.is_some() {
            view.teams.cancel_delete();
            cx.notify();
        } else {
            view.close_team_editor(cx);
        }
    }));
    surface = surface.capture_key_down(cx.listener(|view, event: &KeyDownEvent, window, cx| {
        if event.keystroke.key.eq_ignore_ascii_case("escape") {
            window.prevent_default();
            cx.stop_propagation();
            if view.teams.delete_target.is_some() {
                view.teams.cancel_delete();
            } else {
                view.close_team_editor(cx);
            }
            cx.notify();
        }
    }));
    surface = surface.child(dialog_body);

    if app.teams.delete_target.is_some() {
        let delete_layer = div()
            .id("team-delete-layer")
            .absolute()
            .top_0()
            .left_0()
            .right_0()
            .bottom_0()
            .flex()
            .items_center()
            .justify_center()
            .p_4()
            .bg(rgba(0x080c14d9))
            .on_click(cx.listener(|_, _, _, cx| cx.stop_propagation()));
        surface = surface.child(delete_layer.child(delete_confirmation(app, cx, language)));
    }

    div()
        .absolute()
        .top_0()
        .left_0()
        .right_0()
        .bottom_0()
        .child(surface)
}

fn basic_editor(
    app: &mut AhabApp,
    cx: &mut Context<AhabApp>,
    team: &TeamDetail,
    config: &TeamMirrorConfig,
    language: Language,
) -> Div {
    let name_input = app.team_name_input.clone();
    let code_input = app.team_code_input.clone();

    let name_field = if let Some(input) = name_input {
        labeled_field(
            text("队伍名称", "Team name").get(language),
            div().child(input),
        )
    } else {
        labeled_field(
            text("队伍名称", "Team name").get(language),
            div().child(text("编辑器初始化中", "Initializing").get(language)),
        )
    };

    let purpose = team_select(
        app,
        cx,
        TeamSelect::Purpose,
        purpose_key(team.purpose),
        vec![
            (
                "mirror".to_owned(),
                purpose_label(TeamPurpose::Mirror, language).to_owned(),
            ),
            (
                "luxcavation".to_owned(),
                purpose_label(TeamPurpose::Luxcavation, language).to_owned(),
            ),
            (
                "general".to_owned(),
                purpose_label(TeamPurpose::General, language).to_owned(),
            ),
        ],
        "team-purpose",
        180.,
        Rc::new(|teams, value| {
            let purpose = match value.as_str() {
                "mirror" => TeamPurpose::Mirror,
                "luxcavation" => TeamPurpose::Luxcavation,
                "general" => TeamPurpose::General,
                _ => return,
            };
            teams.set_editor_purpose(purpose);
        }),
    );
    let purpose_field = labeled_field(text("用途", "Purpose").get(language), purpose);

    let mut systems = div().flex().flex_wrap().gap_2();
    for (index, name) in SYSTEM_NAMES.iter().enumerate() {
        let selected = team.accessoryScheme == *name;
        let mut control =
            system_choice(index, selected, false, language).id(format!("team-system-{name}"));
        let system_name = (*name).to_owned();
        let system_name_for_key = system_name.clone();
        control = control
            .on_click(cx.listener(move |view, _, _, cx| {
                view.teams
                    .set_editor_scheme(system_name.clone(), index as u8);
                cx.notify();
            }))
            .on_key_down(cx.listener(move |view, event: &KeyDownEvent, window, cx| {
                if team_activation_key(event) {
                    window.prevent_default();
                    view.teams
                        .set_editor_scheme(system_name_for_key.clone(), index as u8);
                    cx.notify();
                }
            }));
        systems = systems.child(control);
    }

    let mut sinners = div().flex().flex_wrap().gap_2();
    for sinner in app.teams.sinners.clone() {
        let selected = team.sinners.iter().position(|id| id == &sinner.id);
        let id = sinner.id.clone();
        let path = sinner_path(&id);
        let key_id = id.clone();
        let palette = current_render_palette();
        let mut control = div()
            .id(format!("team-sinner-{id}"))
            .w(px(100.))
            .h(px(104.))
            .flex()
            .flex_col()
            .items_center()
            .justify_center()
            .gap_1()
            .rounded_lg()
            .tab_index(0)
            .border_1()
            .border_color(palette_rgb(if selected.is_some() {
                palette.brand
            } else {
                palette.border
            }))
            .bg(palette_rgb(if selected.is_some() {
                palette.brand_light
            } else {
                palette.secondary
            }))
            .cursor_pointer()
            .focus_visible(|style| style.border_color(palette_rgb(current_render_palette().ring)))
            .on_click(cx.listener(move |view, _, _, cx| {
                view.teams.toggle_sinner(&id);
                cx.notify();
            }));
        control =
            control.on_key_down(cx.listener(move |view, event: &KeyDownEvent, window, cx| {
                if team_activation_key(event) {
                    window.prevent_default();
                    view.teams.toggle_sinner(&key_id);
                    cx.notify();
                }
            }));
        control = control.child(
            div()
                .relative()
                .w(px(48.))
                .h(px(48.))
                .child(img(path).size_full())
                .child(
                    selected
                        .map(|index| {
                            div()
                                .absolute()
                                .top_0()
                                .right_0()
                                .px_1()
                                .rounded_md()
                                .bg(palette_rgb(current_render_palette().brand))
                                .text_size(px(9.))
                                .text_color(palette_rgb(current_render_palette().brand_foreground))
                                .child(format!("#{}", index + 1))
                        })
                        .unwrap_or_else(div),
                ),
        );
        control = control.child(
            div()
                .w_full()
                .px_1()
                .truncate()
                .text_center()
                .text_size(px(10.))
                .text_color(rgb(TEXT))
                .child(sinner.name),
        );
        sinners = sinners.child(control);
    }

    let code_field = if config.use_team_code {
        code_input
            .map(|input| {
                labeled_field(
                    text("编队码", "Team code").get(language),
                    div().child(input),
                )
            })
            .unwrap_or_else(|| {
                labeled_field(
                    text("编队码", "Team code").get(language),
                    div().child(text("编辑器初始化中", "Initializing").get(language)),
                )
            })
    } else {
        div()
    };

    let team_code_switch = mirror_switch(
        app,
        cx,
        MirrorBool::UseTeamCode,
        config.use_team_code,
        "basic-use-team-code",
    );
    let fixed_switch = mirror_switch(
        app,
        cx,
        MirrorBool::FixedTeamUse,
        config.fixed_team_use,
        "basic-fixed-team",
    );
    let enabled_switch = switch(team.enabled)
        .id("team-enabled")
        .on_click(cx.listener(|view, _, _, cx| {
            let enabled = view
                .teams
                .editor
                .as_ref()
                .map(|editor| !editor.team.enabled)
                .unwrap_or(true);
            view.teams.set_editor_enabled(enabled);
            cx.notify();
        }))
        .on_key_down(cx.listener(|view, event: &KeyDownEvent, window, cx| {
            if team_activation_key(event) {
                window.prevent_default();
                let enabled = view
                    .teams
                    .editor
                    .as_ref()
                    .map(|editor| !editor.team.enabled)
                    .unwrap_or(true);
                view.teams.set_editor_enabled(enabled);
                cx.notify();
            }
        }));

    let mut clear_sinners = button(
        text("清空人格", "Clear Sinners").get(language),
        ButtonVariant::Ghost,
    )
    .id("team-clear-sinners");
    clear_sinners = clear_sinners.on_click(cx.listener(|view, _, _, cx| {
        view.teams.clear_sinners();
        cx.notify();
    }));

    let team_code_card = card(
        div()
            .flex()
            .flex_col()
            .gap_2()
            .child(control_row(
                text("启用编队码", "Use Team Code").get(language),
                team_code_switch,
            ))
            .child(
                div().text_size(px(10.)).text_color(rgb(TEXT_MUTED)).child(
                    text(
                        "启用后保存编队码，运行时由后端解析。",
                        "Save the team code and let the backend parse it at runtime.",
                    )
                    .get(language),
                ),
            )
            .child(code_field),
    )
    .p_3()
    .flex_basis(px(300.))
    .flex_grow(1.);
    let fixed_card = card(
        div()
            .flex()
            .flex_col()
            .gap_2()
            .child(control_row(
                text("固定队伍用途", "Fixed Team Purpose").get(language),
                fixed_switch,
            ))
            .child(if config.fixed_team_use {
                team_select(
                    app,
                    cx,
                    TeamSelect::FixedTeamUse,
                    config.fixed_team_use_select.to_string(),
                    vec![
                        ("0".to_owned(), fixed_team_use_label(0, language).to_owned()),
                        ("1".to_owned(), fixed_team_use_label(1, language).to_owned()),
                        ("2".to_owned(), fixed_team_use_label(2, language).to_owned()),
                    ],
                    "basic-fixed-team-range",
                    180.,
                    Rc::new(|teams, value| {
                        if let Ok(value) = value.parse::<u8>() {
                            teams.set_mirror_u8(MirrorU8::FixedTeamUseSelect, value);
                        }
                    }),
                )
                .into_any_element()
            } else {
                div().into_any_element()
            }),
    )
    .p_3()
    .flex_basis(px(300.))
    .flex_grow(1.);

    div()
        .flex()
        .flex_col()
        .gap_4()
        .child(
            div()
                .flex()
                .flex_wrap()
                .gap_3()
                .child(name_field.flex_basis(px(300.)).flex_grow(1.))
                .child(purpose_field.flex_basis(px(220.)).flex_grow(1.)),
        )
        .child(field_block(
            text("饰品体系", "Gift System").get(language),
            systems,
        ))
        .child(field_block(
            if matches!(language, Language::ZhCn) {
                format!("人格顺序（{} / 12）", team.sinners.len())
            } else {
                format!("Sinner order ({} / 12)", team.sinners.len())
            },
            div().flex().flex_col().gap_2().child(sinners).child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .gap_2()
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .text_size(px(10.))
                            .text_color(rgb(TEXT_MUTED))
                            .child(
                                text(
                                    "按点击顺序分配 #1~#12；再次点击可移除。",
                                    "Click to assign slots #1-#12; click again to remove.",
                                )
                                .get(language),
                            ),
                    )
                    .child(clear_sinners),
            ),
        ))
        .child(
            div()
                .flex()
                .flex_wrap()
                .gap_3()
                .child(team_code_card)
                .child(fixed_card),
        )
        .child(control_row(
            text("队伍启用", "Team Enabled").get(language),
            enabled_switch,
        ))
}

fn shop_editor(
    app: &mut AhabApp,
    cx: &mut Context<AhabApp>,
    config: &TeamMirrorConfig,
    language: Language,
) -> Div {
    let mut discard = div().flex().flex_wrap().gap_2();
    for (index, name) in SYSTEM_NAMES.iter().enumerate() {
        let selected = discard_value(&config.discard_systems, index);
        let mut control =
            system_choice(index, selected, true, language).id(format!("discard-system-{name}"));
        control = control.on_click(cx.listener(move |view, _, _, cx| {
            view.teams.toggle_discard_system(index);
            cx.notify();
        }));
        discard = discard.child(control);
    }

    let restrictions = [
        (
            text("不治疗", "Do Not Heal"),
            MirrorBool::DoNotHeal,
            config.do_not_heal,
        ),
        (
            text("不购买", "Do Not Buy"),
            MirrorBool::DoNotBuy,
            config.do_not_buy,
        ),
        (
            text("不合成", "Do Not Fuse"),
            MirrorBool::DoNotFuse,
            config.do_not_fuse,
        ),
        (
            text("不出售", "Do Not Sell"),
            MirrorBool::DoNotSell,
            config.do_not_sell,
        ),
        (
            text("不升级", "Do Not Enhance"),
            MirrorBool::DoNotEnhance,
            config.do_not_enhance,
        ),
    ];
    let mut restriction_rows = div().flex().flex_wrap().gap_3();
    for (index, (label, field, value)) in restrictions.into_iter().enumerate() {
        restriction_rows = restriction_rows.child(control_row(
            label.get(language),
            mirror_switch(app, cx, field, value, format!("shop-restriction-{index}")),
        ));
    }

    let fusions = [
        (
            text("仅激进合成", "Aggressive Fusion Only"),
            MirrorBool::OnlyAggressiveFuse,
            config.only_aggressive_fuse,
        ),
        (
            text("不合成体系饰品", "Do Not Fuse System Gifts"),
            MirrorBool::DoNotSystemFuse,
            config.do_not_system_fuse,
        ),
        (
            text("仅合成体系饰品", "System Gifts Only"),
            MirrorBool::OnlySystemFuse,
            config.only_system_fuse,
        ),
        (
            text("激进模式也升级", "Enhance in Aggressive Mode"),
            MirrorBool::AggressiveAlsoEnhance,
            config.aggressive_also_enhance,
        ),
        (
            text("激进模式保留体系", "Keep System Gifts in Aggressive Mode"),
            MirrorBool::AggressiveSaveSystems,
            config.aggressive_save_systems,
        ),
    ];
    let mut fusion_rows = div().flex().flex_col().gap_2();
    for (index, (label, field, value)) in fusions.into_iter().enumerate() {
        fusion_rows = fusion_rows.child(control_row(
            label.get(language),
            mirror_switch(app, cx, field, value, format!("fusion-{index}")),
        ));
    }

    let after_level = card(
        div()
            .flex()
            .flex_col()
            .gap_2()
            .child(control_row(
                text("四级饰品后执行策略", "After Tier 4 Fusion").get(language),
                mirror_switch(
                    app,
                    cx,
                    MirrorBool::AfterLevelIv,
                    config.after_level_IV,
                    "shop-after-level-iv",
                ),
            ))
            .child(if config.after_level_IV {
                team_select(
                    app,
                    cx,
                    TeamSelect::AfterLevelIv,
                    config.after_level_IV_select.to_string(),
                    vec![
                        ("0".to_owned(), after_level_label(0, language).to_owned()),
                        ("1".to_owned(), after_level_label(1, language).to_owned()),
                        ("2".to_owned(), after_level_label(2, language).to_owned()),
                    ],
                    "shop-after-level-iv-action",
                    180.,
                    Rc::new(|teams, value| {
                        if let Ok(value) = value.parse::<u8>() {
                            teams.set_mirror_u8(MirrorU8::AfterLevelIvSelect, value);
                        }
                    }),
                )
                .into_any_element()
            } else {
                div().into_any_element()
            }),
    );

    let keyword_refresh = app
        .team_keyword_refresh_input
        .clone()
        .map(|input| div().w(px(80.)).child(input))
        .unwrap_or_else(|| div().child(text("初始化中", "Initializing").get(language)));
    let normal_refresh = app
        .team_normal_refresh_input
        .clone()
        .map(|input| div().w(px(80.)).child(input))
        .unwrap_or_else(|| div().child(text("初始化中", "Initializing").get(language)));
    let refresh = card(
        div()
            .flex()
            .flex_col()
            .gap_2()
            .child(
                div()
                    .text_size(px(13.))
                    .text_color(rgb(TEXT))
                    .child(text("商店刷新上限", "Shop Refresh Limits").get(language)),
            )
            .child(control_row(
                text("关键词刷新", "Keyword Refreshes").get(language),
                keyword_refresh,
            ))
            .child(control_row(
                text("普通刷新", "Normal Refreshes").get(language),
                normal_refresh,
            )),
    );

    let mut floors = div().flex().flex_wrap().gap_2();
    for floor in 0..5 {
        let ignored = config.ignore_shop.get(floor).copied().unwrap_or(false);
        let mut control = button(
            if ignored {
                format!("{} {}F", text("已忽略", "Ignored").get(language), floor + 1)
            } else {
                format!("{}F", floor + 1)
            },
            if ignored {
                ButtonVariant::Destructive
            } else {
                ButtonVariant::Outline
            },
        )
        .id(format!("ignore-shop-floor-{}", floor + 1))
        .h(px(32.))
        .px_3()
        .py_0();
        control = control
            .on_click(cx.listener(move |view, _, _, cx| {
                view.teams.toggle_ignore_shop(floor);
                cx.notify();
            }))
            .on_key_down(cx.listener(move |view, event: &KeyDownEvent, window, cx| {
                if team_activation_key(event) {
                    window.prevent_default();
                    view.teams.toggle_ignore_shop(floor);
                    cx.notify();
                }
            }));
        floors = floors.child(control);
    }
    let ignore = card(
        div()
            .flex()
            .flex_col()
            .gap_2()
            .child(
                div()
                    .text_size(px(13.))
                    .text_color(rgb(TEXT))
                    .child(text("忽略商店楼层", "Ignore Shop Floors").get(language)),
            )
            .child(floors),
    );

    div()
        .flex()
        .flex_col()
        .gap_4()
        .child(field_block(
            text("商店策略", "Shop Strategy").get(language),
            team_select(
                app,
                cx,
                TeamSelect::ShopStrategy,
                config.shop_strategy.to_string(),
                vec![
                    ("0".to_owned(), shop_strategy_label(0, language).to_owned()),
                    ("1".to_owned(), shop_strategy_label(1, language).to_owned()),
                    ("2".to_owned(), shop_strategy_label(2, language).to_owned()),
                ],
                "shop-strategy",
                180.,
                Rc::new(|teams, value| {
                    if let Ok(value) = value.parse::<u8>() {
                        teams.set_mirror_u8(MirrorU8::ShopStrategy, value);
                    }
                }),
            ),
        ))
        .child(field_block(
            text("舍弃饰品体系", "Discard Gift Systems").get(language),
            div()
                .flex()
                .flex_col()
                .gap_2()
                .child(
                    div().text_size(px(10.)).text_color(rgb(TEXT_MUTED)).child(
                        text(
                            "选择不参与商店购买、合成与保留的体系。",
                            "Choose systems to avoid in shop purchases, fusion and retention.",
                        )
                        .get(language),
                    ),
                )
                .child(discard),
        ))
        .child(field_block(
            text("基础操作限制", "Shop Action Restrictions").get(language),
            div()
                .flex()
                .flex_col()
                .gap_2()
                .child(
                    div().text_size(px(10.)).text_color(rgb(TEXT_MUTED)).child(
                        text(
                            "限制会在镜牢商店阶段按队伍配置执行。",
                            "Restrictions are applied during Mirror Dungeon shop phases.",
                        )
                        .get(language),
                    ),
                )
                .child(restriction_rows),
        ))
        .child(field_block(
            text("合成策略", "Fusion Strategy").get(language),
            fusion_rows,
        ))
        .child(
            div()
                .flex()
                .gap_3()
                .flex_wrap()
                .child(after_level)
                .child(refresh)
                .child(ignore),
        )
}

fn combat_editor(
    app: &mut AhabApp,
    cx: &mut Context<AhabApp>,
    config: &TeamMirrorConfig,
    language: Language,
) -> Div {
    let second_system = card(
        div()
            .flex()
            .flex_col()
            .gap_3()
            .child(control_row(
                text("启用第二体系", "Enable Second System").get(language),
                mirror_switch(
                    app,
                    cx,
                    MirrorBool::SecondSystem,
                    config.second_system,
                    "combat-second-system",
                ),
            ))
            .child(if config.second_system {
                div()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .child(control_row(
                        text("第二体系", "Secondary System").get(language),
                        team_select(
                            app,
                            cx,
                            TeamSelect::SecondSystem,
                            config.second_system_select.to_string(),
                            SYSTEM_NAMES
                                .iter()
                                .enumerate()
                                .map(|(index, _)| {
                                    (index.to_string(), system_label(index, language).to_owned())
                                })
                                .collect(),
                            "combat-second-system-select",
                            180.,
                            Rc::new(|teams, value| {
                                if let Ok(value) = value.parse::<u8>() {
                                    teams.set_mirror_u8(MirrorU8::SecondSystemSelect, value);
                                }
                            }),
                        ),
                    ))
                    .child(control_row(
                        text("起始楼层", "Start Floor").get(language),
                        team_select(
                            app,
                            cx,
                            TeamSelect::SecondSystemFloor,
                            config.second_system_setting.to_string(),
                            (2..=5)
                                .map(|floor| (floor.to_string(), floor_label(floor, language)))
                                .collect(),
                            "combat-second-system-floor",
                            180.,
                            Rc::new(|teams, value| {
                                if let Ok(value) = value.parse::<u8>() {
                                    teams.set_mirror_u8(MirrorU8::SecondSystemStartFloor, value);
                                }
                            }),
                        ),
                    ))
                    .child(control_row(
                        text("四级合成", "Fuse Tier 4").get(language),
                        mirror_switch(
                            app,
                            cx,
                            MirrorBool::SecondSystemFuseIv,
                            config.second_system_fuse_IV,
                            "combat-fuse-iv",
                        ),
                    ))
                    .child(control_row(
                        text("购买饰品", "Buy Gifts").get(language),
                        mirror_switch(
                            app,
                            cx,
                            MirrorBool::SecondSystemBuy,
                            config.second_system_buy,
                            "combat-buy",
                        ),
                    ))
                    .child(control_row(
                        text("选择奖励", "Select Rewards").get(language),
                        mirror_switch(
                            app,
                            cx,
                            MirrorBool::SecondSystemSelectReward,
                            config.second_system_select_reward,
                            "combat-reward",
                        ),
                    ))
                    .child(control_row(
                        text("升级饰品", "Upgrade Gifts").get(language),
                        mirror_switch(
                            app,
                            cx,
                            MirrorBool::SecondSystemPowerUp,
                            config.second_system_power_up,
                            "combat-power-up",
                        ),
                    ))
            } else {
                div().text_size(px(11.)).text_color(rgb(TEXT_MUTED)).child(
                    text(
                        "开启后可配置第二体系和起始楼层。",
                        "Enable this to configure the secondary system and start floor.",
                    )
                    .get(language),
                )
            }),
    );

    let preferences = card(
        div()
            .flex()
            .flex_col()
            .gap_2()
            .child(
                div()
                    .text_size(px(13.))
                    .text_color(rgb(TEXT))
                    .child(text("战斗与技能偏好", "Combat & Skill Preferences").get(language)),
            )
            .child(control_row(
                text("避免三技能", "Avoid Skill 3").get(language),
                mirror_switch(
                    app,
                    cx,
                    MirrorBool::AvoidSkill3,
                    config.avoid_skill_3,
                    "combat-avoid-skill-3",
                ),
            ))
            .child(control_row(
                text("优先三技能", "Prioritize Skill 3").get(language),
                mirror_switch(
                    app,
                    cx,
                    MirrorBool::PrioritizeSkill3,
                    config.prioritize_skill_3,
                    "combat-prioritize-skill-3",
                ),
            ))
            .child(control_row(
                text("每层重新编队", "Re-form Team Each Floor").get(language),
                mirror_switch(
                    app,
                    cx,
                    MirrorBool::ReformationEachFloor,
                    config.re_formation_each_floor,
                    "combat-reformation",
                ),
            )),
    );

    let defense = card(
        div()
            .flex()
            .flex_col()
            .gap_2()
            .child(
                div().text_size(px(13.)).text_color(rgb(TEXT)).child(
                    text(
                        "防御策略（互斥）",
                        "Defense Strategies (Mutually Exclusive)",
                    )
                    .get(language),
                ),
            )
            .child(control_row(
                text("首回合防御", "Defend in Round 1").get(language),
                mirror_switch(
                    app,
                    cx,
                    MirrorBool::DefenseFirstRound,
                    config.defense_first_round,
                    "combat-defense-first",
                ),
            ))
            .child(control_row(
                text("良秀单通防御", "Solo Ryoshu Defense").get(language),
                mirror_switch(
                    app,
                    cx,
                    MirrorBool::DefenseForSolo,
                    config.defense_for_solo,
                    "combat-defense-solo",
                ),
            ))
            .child(if config.defense_for_solo {
                control_row(
                    text("防御回合", "Defense Turns").get(language),
                    team_select(
                        app,
                        cx,
                        TeamSelect::DefenseTurns,
                        config.defense_for_solo_turns.to_string(),
                        (1..=5)
                            .map(|turns| (turns.to_string(), turns_label(turns, language)))
                            .collect(),
                        "combat-defense-turns",
                        180.,
                        Rc::new(|teams, value| {
                            if let Ok(value) = value.parse::<u8>() {
                                teams.set_mirror_u8(MirrorU8::DefenseTurns, value);
                            }
                        }),
                    ),
                )
                .into_any_element()
            } else {
                div().into_any_element()
            }),
    );

    let replacement = card(
        div()
            .flex()
            .flex_col()
            .gap_2()
            .child(control_row(
                text("启用技能替换", "Enable Skill Replacement").get(language),
                mirror_switch(
                    app,
                    cx,
                    MirrorBool::SkillReplacement,
                    config.skill_replacement,
                    "combat-skill-replacement",
                ),
            ))
            .child(if config.skill_replacement {
                div().flex().flex_col().gap_2().child(control_row(
                    text("替换模式", "Replacement Mode").get(language),
                    team_select(
                        app,
                        cx,
                        TeamSelect::SkillReplacementMode,
                        config.skill_replacement_mode.to_string(),
                        vec![
                            ("0".to_owned(), "1 → 2".to_owned()),
                            ("1".to_owned(), "1 → 3".to_owned()),
                        ],
                        "combat-skill-replacement-mode",
                        180.,
                        Rc::new(|teams, value| {
                            if let Ok(value) = value.parse::<u8>() {
                                teams.set_mirror_u8(MirrorU8::SkillReplacementMode, value);
                            }
                        }),
                    ),
                ))
            } else {
                div()
            }),
    );

    div().flex().flex_col().gap_4().child(second_system).child(
        div()
            .flex()
            .gap_3()
            .flex_wrap()
            .child(preferences)
            .child(defense)
            .child(replacement),
    )
}

fn starlight_editor(
    app: &mut AhabApp,
    cx: &mut Context<AhabApp>,
    config: &TeamMirrorConfig,
    language: Language,
) -> Div {
    let use_starlight = card(
        div()
            .flex()
            .flex_col()
            .gap_1()
            .child(control_row(
                text("使用开局星光", "Use Starting Starlight").get(language),
                mirror_switch(
                    app,
                    cx,
                    MirrorBool::UseStarlight,
                    config.use_starlight,
                    "starlight-enabled",
                ),
            ))
            .child(
                div().text_size(px(10.)).text_color(rgb(TEXT_MUTED)).child(
                    text(
                        "开局时按下方等级消耗星光点数。",
                        "Spend starlight at the start according to the levels below.",
                    )
                    .get(language),
                ),
            ),
    )
    .p_3()
    .flex_none();

    let mut quick = div().flex().items_center().gap_1().flex_wrap();
    for level in 0..=3_u8 {
        let mut control = button(
            starlight_level_label(level, language),
            ButtonVariant::Outline,
        )
        .id(format!("starlight-all-{level}"))
        .h(px(26.))
        .px_2()
        .py_0();
        control = control
            .on_click(cx.listener(move |view, _, _, cx| {
                view.teams.set_all_starlight(level);
                cx.notify();
            }))
            .on_key_down(cx.listener(move |view, event: &KeyDownEvent, window, cx| {
                if team_activation_key(event) {
                    window.prevent_default();
                    view.teams.set_all_starlight(level);
                    cx.notify();
                }
            }));
        quick = quick.child(control);
    }

    let cost_badge = div()
        .flex()
        .items_center()
        .gap_1()
        .px_2()
        .py_1()
        .rounded_md()
        .bg(palette_rgb(current_render_palette().brand_light))
        .text_size(px(11.))
        .text_color(palette_rgb(current_render_palette().brand))
        .child(icon(ICON_SPARKLES, 13., current_render_palette().brand))
        .child(starlight_cost_label(app.teams.starlight_cost(), language));
    let quick_card = card(
        div()
            .flex()
            .items_center()
            .justify_between()
            .gap_3()
            .flex_wrap()
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .child(
                        div()
                            .text_size(px(11.))
                            .text_color(rgb(TEXT_MUTED))
                            .child(text("一键设置等级", "Set All Levels").get(language)),
                    )
                    .child(quick),
            )
            .child(cost_badge),
    )
    .p_3()
    .flex_none();

    let mut items = div().flex().flex_wrap().gap_2();
    for index in 0..10 {
        let level = config.opening_bonus.get(index).copied().unwrap_or(0).min(3);
        let mut levels = div().flex().gap_1();
        for candidate in 0..=3_u8 {
            let mut control = button(
                starlight_short_level(candidate),
                if candidate == level {
                    ButtonVariant::Secondary
                } else {
                    ButtonVariant::Ghost
                },
            )
            .id(format!("starlight-{index}-{candidate}"))
            .h(px(24.))
            .px_2()
            .py_0();
            control = control
                .on_click(cx.listener(move |view, _, _, cx| {
                    view.teams.set_starlight_level(index, candidate);
                    cx.notify();
                }))
                .on_key_down(cx.listener(move |view, event: &KeyDownEvent, window, cx| {
                    if team_activation_key(event) {
                        window.prevent_default();
                        view.teams.set_starlight_level(index, candidate);
                        cx.notify();
                    }
                }));
            levels = levels.child(control);
        }
        let cost = div()
            .flex()
            .items_center()
            .gap_1()
            .text_size(px(10.))
            .text_color(rgb(TEXT_MUTED))
            .child(icon(ICON_SPARKLES, 11., current_render_palette().brand))
            .child(starlight_points_label(STARLIGHT_COSTS[index], language));
        items = items.child(
            card(
                div()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .gap_2()
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap_1()
                                    .min_w_0()
                                    .text_size(px(12.))
                                    .text_color(rgb(TEXT))
                                    .child(starlight_name(index, language)),
                            )
                            .child(cost),
                    )
                    .child(levels)
                    .child(
                        div()
                            .text_size(px(10.))
                            .text_color(rgb(TEXT_MUTED))
                            .child(starlight_description(index, language)),
                    ),
            )
            .p_3()
            .flex_basis(px(320.))
            .flex_grow(1.),
        );
    }

    div()
        .flex()
        .flex_col()
        .gap_3()
        .child(use_starlight)
        .child(quick_card)
        .child(items)
}

fn advanced_editor(
    app: &mut AhabApp,
    cx: &mut Context<AhabApp>,
    config: &TeamMirrorConfig,
    language: Language,
) -> Div {
    let observe_input = app.team_observe_input.clone();
    let json_input = app.team_json_input.clone();
    let observe_switch = mirror_switch(
        app,
        cx,
        MirrorBool::ObserveEgoGift,
        config.observe_ego_gift,
        "advanced-observe-ego",
    );
    let custom_weight = mirror_switch(
        app,
        cx,
        MirrorBool::UseCustomThemeWeight,
        config.use_custom_theme_pack_weight,
        "advanced-theme-weight",
    );

    let mut gifts = div().flex().flex_wrap().gap_1();
    for gift in &config.observe_ego_gift_selected {
        let gift_for_remove = gift.clone();
        let remove = div()
            .id(format!("remove-gift-{gift}"))
            .flex()
            .items_center()
            .justify_center()
            .size(px(22.))
            .rounded_md()
            .cursor_pointer()
            .child(icon(
                ICON_CLOSE,
                12.,
                current_render_palette().muted_foreground,
            ))
            .on_click(cx.listener(move |view, _, _, cx| {
                view.teams.remove_observe_gift(&gift_for_remove);
                cx.notify();
            }));
        gifts = gifts.child(
            div()
                .flex()
                .items_center()
                .gap_1()
                .px_2()
                .py_1()
                .rounded_md()
                .bg(palette_rgb(current_render_palette().secondary))
                .text_size(px(11.))
                .text_color(palette_rgb(current_render_palette().foreground))
                .child(gift.clone())
                .child(remove),
        );
    }

    let mut add_observe = button(text("添加", "Add").get(language), ButtonVariant::Default)
        .id("advanced-add-observe")
        .h(px(34.))
        .px_3()
        .py_0()
        .on_click(cx.listener(|view, _, _, cx| view.add_team_observe_gift(cx)));
    add_observe = add_observe.on_key_down(cx.listener(|view, event: &KeyDownEvent, window, cx| {
        if team_activation_key(event) {
            window.prevent_default();
            view.add_team_observe_gift(cx);
        }
    }));
    let observe_field = observe_input
        .map(|input| {
            div()
                .flex_1()
                .min_w_0()
                .child(input)
                .on_key_down(cx.listener(|view, event: &KeyDownEvent, window, cx| {
                    if event.keystroke.key.eq_ignore_ascii_case("enter") {
                        window.prevent_default();
                        cx.stop_propagation();
                        view.add_team_observe_gift(cx);
                    }
                }))
                .into_any_element()
        })
        .unwrap_or_else(|| div().into_any_element());
    let observe_content = if config.observe_ego_gift {
        card(
            div()
                .flex()
                .flex_col()
                .gap_2()
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap_2()
                        .child(observe_field)
                        .child(add_observe),
                )
                .child(gifts),
        )
        .p_3()
    } else {
        div()
    };

    let mut import_toggle = div()
        .id("advanced-json-toggle")
        .flex()
        .items_center()
        .justify_center()
        .gap_1()
        .h(px(28.))
        .px_2()
        .rounded_md()
        .tab_index(0)
        .border_1()
        .border_color(rgb(BORDER))
        .cursor_pointer()
        .focus_visible(|style| style.border_color(palette_rgb(current_render_palette().ring)))
        .text_size(px(11.))
        .text_color(rgb(crate::app::ACCENT))
        .child(icon(ICON_PASTE, 14., current_render_palette().brand))
        .child(text("粘贴 / 导入 JSON", "Paste / Import JSON").get(language));
    import_toggle = import_toggle
        .on_click(cx.listener(|view, _, _, cx| {
            if let Some(editor) = view.teams.editor.as_mut() {
                editor.json_import_open = !editor.json_import_open;
            }
            cx.notify();
        }))
        .on_key_down(cx.listener(|view, event: &KeyDownEvent, window, cx| {
            if team_activation_key(event) {
                window.prevent_default();
                if let Some(editor) = view.teams.editor.as_mut() {
                    editor.json_import_open = !editor.json_import_open;
                }
                cx.notify();
            }
        }));
    let json_panel = if app
        .teams
        .editor
        .as_ref()
        .map(|editor| editor.json_import_open)
        .unwrap_or(false)
    {
        div()
            .flex()
            .flex_col()
            .gap_2()
            .child(
                json_input
                    .map(|input| input.into_any_element())
                    .unwrap_or_else(|| div().into_any_element()),
            )
            .child(
                div()
                    .flex()
                    .justify_end()
                    .gap_2()
                    .child({
                        let mut close =
                            button(text("关闭", "Close").get(language), ButtonVariant::Ghost)
                                .id("advanced-json-close")
                                .h(px(28.))
                                .px_3()
                                .py_0()
                                .on_click(cx.listener(|view, _, _, cx| {
                                    if let Some(editor) = view.teams.editor.as_mut() {
                                        editor.json_import_open = false;
                                    }
                                    cx.notify();
                                }));
                        close = close.on_key_down(cx.listener(
                            |view, event: &KeyDownEvent, window, cx| {
                                if team_activation_key(event) {
                                    window.prevent_default();
                                    if let Some(editor) = view.teams.editor.as_mut() {
                                        editor.json_import_open = false;
                                    }
                                    cx.notify();
                                }
                            },
                        ));
                        close
                    })
                    .child({
                        let mut import = button(
                            text("校验并覆盖", "Validate & Apply").get(language),
                            ButtonVariant::Default,
                        )
                        .id("advanced-json-import")
                        .h(px(28.))
                        .px_3()
                        .py_0()
                        .on_click(cx.listener(|view, _, _, cx| view.import_team_json(cx)));
                        import = import.on_key_down(cx.listener(
                            |view, event: &KeyDownEvent, window, cx| {
                                if team_activation_key(event) {
                                    window.prevent_default();
                                    view.import_team_json(cx);
                                }
                            },
                        ));
                        import
                    }),
            )
    } else {
        div()
    };

    div()
        .flex()
        .flex_col()
        .gap_4()
        .child(
            card(
                div()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .child(control_row(
                        text("观测 E.G.O 饰品", "Observe E.G.O Gifts").get(language),
                        observe_switch,
                    ))
                    .child(
                        div().text_size(px(10.)).text_color(rgb(TEXT_MUTED)).child(
                            text(
                                "输入名称后点击添加；重复名称不会重复加入。",
                                "Enter a gift name and click Add; duplicates are ignored.",
                            )
                            .get(language),
                        ),
                    )
                    .child(observe_content),
            )
            .p_3()
            .flex_none(),
        )
        .child(
            card(
                div()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .child(control_row(
                        text("使用队伍专属主题包权重", "Use Custom Theme Pack Weight")
                            .get(language),
                        custom_weight,
                    ))
                    .child(
                        div().text_size(px(10.)).text_color(rgb(TEXT_MUTED)).child(
                            text(
                                "保存后由镜牢执行器读取该队伍的主题包权重。",
                                "The Mirror executor reads this team's pack weights after Save.",
                            )
                            .get(language),
                        ),
                    ),
            )
            .p_3()
            .flex_none(),
        )
        .child(
            card(
                div()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .gap_2()
                            .child(div().text_size(px(13.)).text_color(rgb(TEXT)).child(
                                text("配置导入导出", "Configuration Import / Export").get(language),
                            ))
                            .child(import_toggle),
                    )
                    .child(json_panel),
            )
            .p_3()
            .flex_none(),
        )
}

fn delete_confirmation(
    app: &mut AhabApp,
    cx: &mut Context<AhabApp>,
    language: Language,
) -> impl IntoElement {
    let Some(team) = app.teams.delete_target.as_ref() else {
        return div().into_any_element();
    };
    let name = team.name.clone();
    div()
        .id("delete-confirmation")
        .w(px(400.))
        .max_w_full()
        .p_4()
        .rounded_lg()
        .border_1()
        .border_color(palette_rgb(current_render_palette().danger))
        .bg(palette_rgb(current_render_palette().danger_light))
        .child(
            div()
                .flex()
                .flex_col()
                .gap_3()
                .child(
                    div()
                        .text_size(px(15.))
                        .text_color(rgb(TEXT))
                        .child(text("确认删除队伍？", "Delete this team?").get(language)),
                )
                .child(
                    div()
                        .text_size(px(12.))
                        .text_color(rgb(TEXT_MUTED))
                        .child(name),
                )
                .child(
                    div()
                        .flex()
                        .justify_end()
                        .gap_2()
                        .child({
                            let mut cancel =
                                button(text("取消", "Cancel").get(language), ButtonVariant::Ghost)
                                    .id("delete-cancel")
                                    .h(px(32.))
                                    .px_3()
                                    .py_0()
                                    .on_click(cx.listener(|view, _, _, cx| {
                                        view.teams.cancel_delete();
                                        cx.notify();
                                    }));
                            cancel = cancel.on_key_down(cx.listener(
                                |view, event: &KeyDownEvent, window, cx| {
                                    if team_activation_key(event) {
                                        window.prevent_default();
                                        view.teams.cancel_delete();
                                        cx.notify();
                                    }
                                },
                            ));
                            cancel
                        })
                        .child({
                            let mut confirm = button(
                                text("删除", "Delete").get(language),
                                ButtonVariant::Destructive,
                            )
                            .id("delete-confirm")
                            .h(px(32.))
                            .px_3()
                            .py_0()
                            .on_click(cx.listener(|view, _, _, cx| {
                                if let Err(error) = view.teams.confirm_delete() {
                                    view.teams.feedback = Some(error);
                                }
                                cx.notify();
                            }));
                            confirm = confirm.on_key_down(cx.listener(
                                |view, event: &KeyDownEvent, window, cx| {
                                    if team_activation_key(event) {
                                        window.prevent_default();
                                        if let Err(error) = view.teams.confirm_delete() {
                                            view.teams.feedback = Some(error);
                                        }
                                        cx.notify();
                                    }
                                },
                            ));
                            confirm
                        }),
                ),
        )
        .on_click(cx.listener(|_, _, _, cx| cx.stop_propagation()))
        .into_any_element()
}

fn mirror_switch(
    _app: &mut AhabApp,
    cx: &mut Context<AhabApp>,
    field: MirrorBool,
    value: bool,
    id: impl Into<String>,
) -> gpui::Stateful<Div> {
    let mut control = switch(value).id(id.into());
    control = control.on_click(cx.listener(move |view, _, _, cx| {
        view.teams.set_mirror_bool(field, !value);
        cx.notify();
    }));
    control.on_key_down(cx.listener(move |view, event: &KeyDownEvent, window, cx| {
        if matches!(
            event.keystroke.key.to_ascii_lowercase().as_str(),
            "enter" | "space"
        ) {
            window.prevent_default();
            view.teams.set_mirror_bool(field, !value);
            cx.notify();
        }
    }))
}

fn team_select(
    app: &AhabApp,
    cx: &mut Context<AhabApp>,
    select: TeamSelect,
    current: impl Into<String>,
    options: Vec<(String, String)>,
    id: impl Into<String>,
    width: f32,
    on_change: Rc<dyn Fn(&mut TeamsState, String)>,
) -> Div {
    let current = current.into();
    let selected_index = options
        .iter()
        .position(|(value, _)| value == &current)
        .unwrap_or(0);
    let selected_label = options
        .get(selected_index)
        .map(|(_, label)| label.clone())
        .unwrap_or_else(|| current.clone());
    let values = options
        .iter()
        .map(|(value, _)| value.clone())
        .collect::<Vec<_>>();
    let open = app.teams.is_select_open(select);
    let palette = current_render_palette();
    let id = id.into();
    let mut trigger = select_trigger(selected_label, open, &palette)
        .id(id.clone())
        .w(px(width));
    trigger = trigger.on_click(cx.listener(move |view, _, _, cx| {
        if open {
            view.teams.close_select();
        } else {
            view.teams.toggle_select(select);
        }
        cx.stop_propagation();
        cx.notify();
    }));
    let key_change = on_change.clone();
    let key_values = values.clone();
    let key_current = current.clone();
    trigger = trigger.on_key_down(cx.listener(move |view, event: &KeyDownEvent, window, cx| {
        let key = event.keystroke.key.to_ascii_lowercase();
        if key == "escape" {
            window.prevent_default();
            view.teams.close_select();
            cx.notify();
            return;
        }
        if matches!(key.as_str(), "enter" | "space") {
            window.prevent_default();
            view.teams.toggle_select(select);
            cx.notify();
            return;
        }

        let current_index = key_values
            .iter()
            .position(|candidate| candidate == &key_current)
            .unwrap_or(0);
        let next_index = match key.as_str() {
            "left" | "arrowleft" | "up" | "arrowup" => Some(
                (current_index as isize - 1).rem_euclid(key_values.len().max(1) as isize) as usize,
            ),
            "right" | "arrowright" => Some((current_index + 1) % key_values.len().max(1)),
            "down" | "arrowdown" if open => Some((current_index + 1) % key_values.len().max(1)),
            "down" | "arrowdown" => Some(current_index),
            "home" => Some(0),
            "end" => key_values.len().checked_sub(1),
            _ => None,
        };
        if let Some(next_index) = next_index
            && let Some(value) = key_values.get(next_index)
        {
            window.prevent_default();
            if matches!(key.as_str(), "down" | "arrowdown" | "up" | "arrowup") && !open {
                view.teams.toggle_select(select);
            } else {
                key_change(&mut view.teams, value.clone());
                if !open {
                    view.teams.close_select();
                }
            }
            cx.notify();
        }
    }));

    let mut option_list = div().flex().flex_col().gap_1();
    for (value, option_label) in options {
        let selected = value == current;
        let option_id = format!("{id}-option-{value}");
        let click_change = on_change.clone();
        let click_value = value.clone();
        let mut option = select_option(option_label, selected, &palette)
            .id(option_id)
            .on_click(cx.listener(move |view, _, _, cx| {
                click_change(&mut view.teams, click_value.clone());
                view.teams.close_select();
                cx.stop_propagation();
                cx.notify();
            }));
        let key_change = on_change.clone();
        let key_value = value;
        option = option.on_key_down(cx.listener(move |view, event: &KeyDownEvent, window, cx| {
            if team_activation_key(event) {
                window.prevent_default();
                key_change(&mut view.teams, key_value.clone());
                view.teams.close_select();
                cx.notify();
            } else if event.keystroke.key.eq_ignore_ascii_case("escape") {
                window.prevent_default();
                view.teams.close_select();
                cx.notify();
            }
        }));
        option_list = option_list.child(option);
    }

    let mut root = div().relative().w(px(width)).child(trigger);
    if open {
        root = root.child(select_popup(option_list, &palette).shadow_sm());
    }
    root
}

fn team_activation_key(event: &KeyDownEvent) -> bool {
    matches!(
        event.keystroke.key.to_ascii_lowercase().as_str(),
        "enter" | "space"
    )
}

fn cycle_u8_value(current: u8, max: u8, direction: i8) -> u8 {
    if max == 0 {
        return 0;
    }
    let current = current.min(max) as i16;
    let max = i16::from(max);
    (current + i16::from(direction)).rem_euclid(max + 1) as u8
}

fn field_block(title: impl Into<String>, body: impl IntoElement) -> Div {
    card(
        div()
            .flex()
            .flex_col()
            .gap_2()
            .child(
                div()
                    .text_size(px(13.))
                    .text_color(rgb(TEXT))
                    .child(title.into()),
            )
            .child(body),
    )
}

fn labeled_field(title: impl Into<String>, input: impl IntoElement) -> Div {
    div()
        .flex()
        .flex_col()
        .gap_1()
        .child(
            div()
                .text_size(px(11.))
                .text_color(rgb(TEXT_MUTED))
                .child(title.into()),
        )
        .child(input)
}

fn control_row(label: impl Into<String>, control: impl IntoElement) -> Div {
    div()
        .flex()
        .items_center()
        .justify_between()
        .gap_3()
        .py_1()
        .child(
            div()
                .text_size(px(11.))
                .text_color(rgb(TEXT_MUTED))
                .child(label.into()),
        )
        .child(control)
}

fn system_choice(index: usize, selected: bool, destructive: bool, language: Language) -> Div {
    let index = index.min(SYSTEM_NAMES.len() - 1);
    let palette = current_render_palette();
    let (border, background, foreground) = if destructive && selected {
        (palette.danger, palette.danger_light, palette.danger)
    } else if selected {
        (palette.brand, palette.brand_light, palette.brand)
    } else {
        (palette.border, palette.card, palette.muted_foreground)
    };
    div()
        .flex()
        .items_center()
        .gap_2()
        .h(px(34.))
        .flex_basis(px(120.))
        .flex_grow(1.)
        .min_w(px(108.))
        .px_2()
        .rounded_md()
        .border_1()
        .border_color(palette_rgb(border))
        .bg(palette_rgb(background))
        .tab_index(0)
        .cursor_pointer()
        .focus_visible(|style| style.border_color(palette_rgb(current_render_palette().ring)))
        .text_size(px(11.))
        .text_color(palette_rgb(foreground))
        .child(
            img(status_effect_path(SYSTEM_NAMES[index]))
                .w(px(18.))
                .h(px(18.)),
        )
        .child(
            div()
                .min_w_0()
                .truncate()
                .child(system_label(index, language)),
        )
}

fn scheme_badge(scheme: &str, language: Language) -> Div {
    let scheme = normalized_scheme(scheme);
    div()
        .flex()
        .items_center()
        .gap_1()
        .h(px(20.))
        .px_2()
        .rounded_md()
        .border_1()
        .border_color(rgb(BORDER))
        .text_size(px(11.))
        .text_color(rgb(TEXT_MUTED))
        .child(img(status_effect_path(scheme)).w(px(14.)).h(px(14.)))
        .child(scheme_label(scheme, language))
}

fn normalized_scheme(scheme: &str) -> &str {
    SYSTEM_NAMES
        .iter()
        .copied()
        .find(|name| *name == scheme)
        .unwrap_or(SYSTEM_NAMES[0])
}

fn filter_label(filter: TeamFilter, language: Language) -> &'static str {
    match filter {
        TeamFilter::All => text("全部", "All").get(language),
        TeamFilter::Mirror => text("镜牢", "Mirror").get(language),
        TeamFilter::Luxcavation => text("经验本", "EXP Dungeon").get(language),
        TeamFilter::General => text("通用", "General").get(language),
    }
}

fn purpose_label(purpose: TeamPurpose, language: Language) -> &'static str {
    match purpose {
        TeamPurpose::Mirror => text("镜牢", "Mirror").get(language),
        TeamPurpose::Luxcavation => text("经验本", "EXP Dungeon").get(language),
        TeamPurpose::General => text("通用", "General").get(language),
    }
}

fn purpose_key(purpose: TeamPurpose) -> &'static str {
    match purpose {
        TeamPurpose::Mirror => "mirror",
        TeamPurpose::Luxcavation => "luxcavation",
        TeamPurpose::General => "general",
    }
}

fn editor_tab_label(tab: TeamEditorTab, language: Language) -> &'static str {
    match tab {
        TeamEditorTab::Basic => text("基础编成", "Basic & Formation").get(language),
        TeamEditorTab::Shop => text("商店与合成", "Shop & Fusion").get(language),
        TeamEditorTab::Combat => text("二体系与战斗", "Second System & Combat").get(language),
        TeamEditorTab::Starlight => text("开局星光", "Starlight Bonus").get(language),
        TeamEditorTab::Advanced => text("观测与高级", "Observe & Advanced").get(language),
    }
}

fn system_label(index: usize, language: Language) -> &'static str {
    let index = index.min(SYSTEM_NAMES.len().saturating_sub(1));
    if matches!(language, Language::EnUs) {
        SYSTEM_LABELS_EN[index]
    } else {
        crate::state::SYSTEM_LABELS[index]
    }
}

fn starlight_name(index: usize, language: Language) -> &'static str {
    if matches!(language, Language::EnUs) {
        STARLIGHT_NAMES_EN[index]
    } else {
        STARLIGHT_NAMES[index]
    }
}

fn starlight_description(index: usize, language: Language) -> &'static str {
    if matches!(language, Language::EnUs) {
        STARLIGHT_DESCRIPTIONS_EN[index]
    } else {
        STARLIGHT_DESCRIPTIONS[index]
    }
}

fn scheme_label(scheme: &str, language: Language) -> &'static str {
    let labels = [
        ("burn", text("燃烧", "Burn")),
        ("bleed", text("流血", "Bleed")),
        ("tremor", text("震颤", "Tremor")),
        ("rupture", text("破裂", "Rupture")),
        ("sinking", text("沉沦", "Sinking")),
        ("poise", text("呼吸", "Poise")),
        ("charge", text("充能", "Charge")),
        ("slash", text("斩击", "Slash")),
        ("pierce", text("突刺", "Pierce")),
        ("blunt", text("打击", "Blunt")),
    ];
    labels
        .iter()
        .find(|(id, _)| *id == scheme)
        .map(|(_, label)| label.get(language))
        .unwrap_or_else(|| text("燃烧", "Burn").get(language))
}

fn discard_value(systems: &crate::model::DiscardSystems, index: usize) -> bool {
    match index {
        0 => systems.burn,
        1 => systems.bleed,
        2 => systems.tremor,
        3 => systems.rupture,
        4 => systems.sinking,
        5 => systems.poise,
        6 => systems.charge,
        7 => systems.slash,
        8 => systems.pierce,
        9 => systems.blunt,
        _ => false,
    }
}

fn discard_count(config: &TeamMirrorConfig) -> usize {
    (0..10)
        .filter(|index| discard_value(&config.discard_systems, *index))
        .count()
}

fn shop_strategy_label(value: u8, language: Language) -> &'static str {
    match value {
        0 => text("默认", "Default").get(language),
        1 => text("保守", "Conservative").get(language),
        2 => text("激进", "Aggressive").get(language),
        _ => text("默认", "Default").get(language),
    }
}

fn after_level_label(value: u8, language: Language) -> &'static str {
    match value {
        0 => text("停止", "Stop").get(language),
        1 => text("继续", "Continue").get(language),
        2 => text("升级", "Enhance").get(language),
        _ => text("停止", "Stop").get(language),
    }
}

fn fixed_team_use_label(value: u8, language: Language) -> &'static str {
    match value {
        0 => text("困难专用", "Hard only").get(language),
        1 => text("普通专用", "Normal only").get(language),
        2 => text("全部通用", "All modes").get(language),
        _ => text("困难专用", "Hard only").get(language),
    }
}

fn floor_label(floor: u8, language: Language) -> String {
    match language {
        Language::ZhCn => format!("第 {floor} 层"),
        Language::EnUs => format!("Floor {floor}"),
    }
}

fn turns_label(turns: u8, language: Language) -> String {
    match language {
        Language::ZhCn => format!("{turns} 回合"),
        Language::EnUs => format!("{turns} turns"),
    }
}

fn starlight_level_label(level: u8, language: Language) -> &'static str {
    match level {
        0 => text("全部关闭", "All off").get(language),
        1 => text("全部基础", "All base").get(language),
        2 => text("全部 2+", "All 2+").get(language),
        3 => text("全部 3++", "All 3++").get(language),
        _ => text("全部关闭", "All off").get(language),
    }
}

fn starlight_short_level(level: u8) -> &'static str {
    match level {
        0 => "0",
        1 => "1",
        2 => "2+",
        3 => "3++",
        _ => "0",
    }
}

fn starlight_cost_label(cost: u32, language: Language) -> String {
    match language {
        Language::ZhCn => format!("总消耗 {cost} 点"),
        Language::EnUs => format!("Total Starlight Cost: {cost}"),
    }
}

fn starlight_points_label(cost: u32, language: Language) -> String {
    match language {
        Language::ZhCn => format!("{cost} 点"),
        Language::EnUs => format!("{cost} pts"),
    }
}

fn localized_feedback(feedback: &str, language: Language) -> String {
    if matches!(language, Language::ZhCn) {
        return feedback.to_owned();
    }
    if let Some(detail) = feedback.strip_prefix("导入失败：") {
        return format!("Import failed: {detail}");
    }
    match feedback {
        "已导入队伍 JSON（尚未保存）" => "Team JSON imported (not saved yet)".to_owned(),
        "队伍 JSON 已复制" => "Team JSON copied".to_owned(),
        "队伍已保存" => "Team saved".to_owned(),
        "队伍已删除" => "Team deleted".to_owned(),
        "队伍名称不能为空" => "Team name is required".to_owned(),
        "队伍最多选择 12 名人格" => "A team can contain at most 12 sinners".to_owned(),
        "当前没有打开队伍编辑器" => "No team editor is open".to_owned(),
        "队伍 JSON 必须是对象" => "Team JSON must be an object".to_owned(),
        "队伍 JSON 缺少 name" => "Team JSON is missing name".to_owned(),
        "purpose 无效" => "Invalid team purpose".to_owned(),
        "sinners 无效" => "Invalid sinner list".to_owned(),
        "mirrorConfig 必须是对象" => "mirrorConfig must be an object".to_owned(),
        "mirrorConfig 默认值无效" => "Invalid mirrorConfig defaults".to_owned(),
        _ => feedback.to_owned(),
    }
}

fn sinner_path(id: &str) -> ImageSource {
    let asset = SinnerAsset::from_id(id).unwrap_or(SinnerAsset::DonQuixote);
    assets::image_source(Asset::Sinner(asset))
}

fn status_effect_path(scheme: &str) -> ImageSource {
    let asset = StatusEffectAsset::from_id(scheme).unwrap_or(StatusEffectAsset::General);
    assets::image_source(Asset::StatusEffect(asset))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_labels_cover_contract_values() {
        assert_eq!(purpose_label(TeamPurpose::Mirror, Language::ZhCn), "镜牢");
        assert_eq!(scheme_label("poise", Language::ZhCn), "呼吸");
        assert_eq!(shop_strategy_label(2, Language::ZhCn), "激进");
        assert_eq!(after_level_label(1, Language::ZhCn), "继续");
        assert_eq!(fixed_team_use_label(2, Language::ZhCn), "全部通用");
    }

    #[test]
    fn unknown_scheme_falls_back_to_burn() {
        assert_eq!(scheme_label("unknown", Language::ZhCn), "燃烧");
        assert_eq!(discard_count(&TeamMirrorConfig::default()), 0);
    }

    #[test]
    fn dynamic_labels_follow_the_selected_language() {
        assert_eq!(starlight_cost_label(30, Language::ZhCn), "总消耗 30 点");
        assert_eq!(
            starlight_cost_label(30, Language::EnUs),
            "Total Starlight Cost: 30"
        );
        assert_eq!(starlight_points_label(10, Language::EnUs), "10 pts");
        assert_eq!(
            localized_feedback("队伍 JSON 已复制", Language::EnUs),
            "Team JSON copied"
        );
    }

    #[test]
    fn cycle_control_wraps_and_clamps_keyboard_values() {
        assert_eq!(cycle_u8_value(0, 2, -1), 2);
        assert_eq!(cycle_u8_value(2, 2, 1), 0);
        assert_eq!(cycle_u8_value(9, 2, -1), 1);
        assert_eq!(cycle_u8_value(1, 0, 1), 0);
    }
}
