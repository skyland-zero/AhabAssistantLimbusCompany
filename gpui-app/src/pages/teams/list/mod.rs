use super::*;

pub(crate) fn render(app: &mut AhabApp, cx: &mut Context<AhabApp>) -> Div {
    let language = app.state.settings.language;
    let filter = app.teams.filter;

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

    let fixed_slots = app.teams.fixed_slots_for_filter(filter);
    let extra_teams = app.teams.extra_teams_for_filter(filter);
    let has_fixed_slots = !fixed_slots.is_empty();
    let mut cards = div().flex().flex_col().gap_4();
    if fixed_slots.is_empty() && extra_teams.is_empty() {
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
        let mut fixed_cards = div().w_full().grid().grid_cols(2).gap_3().items_stretch();
        for slot in fixed_slots {
            let item = if let Some(team) = slot.team {
                team_card(app, cx, team, Some(slot.number), language)
            } else {
                empty_slot_card(app, cx, slot.number, language)
            };
            fixed_cards = fixed_cards.child(item.w_full());
        }
        if has_fixed_slots {
            cards = cards.child(fixed_cards);
        }

        if !extra_teams.is_empty() {
            if !matches!(filter, TeamFilter::General) {
                cards = cards.child(
                    div()
                        .text_size(px(12.))
                        .font_weight(gpui::FontWeight::MEDIUM)
                        .text_color(palette_rgb(current_render_palette().muted_foreground))
                        .child(text("额外编队", "Extra teams").get(language)),
                );
            }
            let mut extra_cards = div().w_full().grid().grid_cols(2).gap_3().items_stretch();
            for team in extra_teams {
                extra_cards = extra_cards.child(
                    team_card(
                        app,
                        cx,
                        team.clone(),
                        team_number_from_id(&team.id),
                        language,
                    )
                    .w_full(),
                );
            }
            cards = cards.child(extra_cards);
        }
    }

    let mut root = page_root().flex_1().min_h_0();
    root = root.child(page_toolbar(
        div()
            .flex()
            .items_center()
            .justify_between()
            .gap_3()
            .flex_wrap()
            .flex_none()
            .child(filter_bar)
            .child(new_team),
    ));
    if let Some(feedback) = app.teams.feedback.clone() {
        root = root.child(
            card(
                div()
                    .flex_none()
                    .text_size(px(11.))
                    .text_color(palette_rgb(current_render_palette().success))
                    .child(localized_feedback(&feedback, language)),
            )
            .w_full()
            .p_3()
            .bg(palette_rgb(current_render_palette().success_light)),
        );
    }
    root.child(
        scroll_area_with_id(app, "teams-list-scroll", div().w_full().child(cards))
            .flex_1()
            .min_h_0(),
    )
}

mod cards;

use cards::{empty_slot_card, team_card};
