use gpui::WeakEntity;

use super::*;

/// Everything one preset card renders, snapshotted when the sheet opens.
///
/// The picker lives on a `Root` sheet, and a sheet's builder runs *inside* the
/// owner's render pass, so it can neither read nor update `AhabApp` - doing so
/// panics with "cannot update ... while it is already being updated". The sheet
/// therefore renders from this snapshot, and the only live handle it holds is a
/// `WeakEntity` whose `update` runs later, when a card is actually clicked.
pub(crate) struct PresetPickerEntry {
    preset_id: String,
    name: String,
    description: String,
    floor_hint: String,
    route_name: String,
    sinners: Vec<String>,
}

impl PresetPickerEntry {
    pub(crate) fn new(preset: &TeamPreset, app: &AhabApp, language: Language) -> Self {
        Self {
            preset_id: preset.presetId.clone(),
            name: preset.name.get(language).to_owned(),
            description: preset.description.get(language).to_owned(),
            floor_hint: preset.floorHint.get(language).to_owned(),
            route_name: preset.routeName.get(language).to_owned(),
            sinners: preset
                .team
                .sinners
                .iter()
                .take(12)
                .map(|sinner| app.teams.sinner_name(sinner))
                .collect(),
        }
    }
}

fn preset_card(entry: PresetPickerEntry, app: WeakEntity<AhabApp>) -> Div {
    let palette = current_render_palette();
    let mut sinner_list = div().flex().flex_wrap().gap_1();
    for sinner in &entry.sinners {
        sinner_list = sinner_list.child(badge(sinner.clone(), BadgeTone::Neutral));
    }
    let select_app = app.clone();
    let preset_id = entry.preset_id.clone();
    let mut control = card(
        div()
            .flex()
            .flex_col()
            .gap_2()
            .child(
                div()
                    .text_size(px(14.))
                    .font_weight(gpui::FontWeight::MEDIUM)
                    .text_color(palette_rgb(palette.foreground))
                    .child(entry.name),
            )
            .child(
                div()
                    .text_size(px(11.))
                    .text_color(palette_rgb(palette.muted_foreground))
                    .child(entry.description),
            )
            .child(
                div()
                    .text_size(px(10.))
                    .text_color(palette_rgb(palette.muted_foreground))
                    .child(entry.floor_hint),
            )
            .child(
                div()
                    .text_size(px(10.))
                    .text_color(palette_rgb(palette.brand))
                    .child(entry.route_name),
            )
            .child(sinner_list),
    )
    .id(format!("team-preset-{}", preset_id))
    .w_full()
    .min_h(px(170.))
    .p_3()
    .bg(palette_rgb(palette.secondary))
    .border_1()
    .border_color(palette_rgb(palette.input))
    .tab_index(0)
    .cursor_pointer()
    .hover(|style| {
        style
            .bg(palette_rgb(current_render_palette().accent_surface))
            .border_color(palette_rgb(current_render_palette().brand))
    })
    .on_click({
        let app = select_app.clone();
        let preset_id = preset_id.clone();
        move |_, window, cx| {
            let _ = app.update(cx, |view, cx| {
                view.select_team_preset(&preset_id, window, cx)
            });
        }
    });
    control = control.on_key_down(move |event: &KeyDownEvent, window, cx| {
        if team_activation_key(event) {
            window.prevent_default();
            let _ = select_app.update(cx, |view, cx| {
                view.select_team_preset(&preset_id, window, cx)
            });
        }
    });
    div().w_full().child(control)
}

/// The preset catalog as a plain grid, ready to hand to a `Sheet`.
pub(crate) fn preset_picker_body(
    entries: &[PresetPickerEntry],
    app: WeakEntity<AhabApp>,
    language: Language,
) -> Div {
    let mut preset_grid = div().w_full().grid().grid_cols(2).gap_4();
    for entry in entries {
        preset_grid = preset_grid.child(preset_card(
            PresetPickerEntry {
                preset_id: entry.preset_id.clone(),
                name: entry.name.clone(),
                description: entry.description.clone(),
                floor_hint: entry.floor_hint.clone(),
                route_name: entry.route_name.clone(),
                sinners: entry.sinners.clone(),
            },
            app.clone(),
        ));
    }
    let content = if entries.is_empty() {
        scroll_area_with_id(
            "team-preset-picker-scroll",
            empty_state(
                text("暂无内置预设", "No built-in presets").get(language),
                text(
                    "请确认后端已完成数据加载。",
                    "Wait for the backend catalog to load.",
                )
                .get(language),
            )
            .w_full()
            .min_h(px(220.)),
        )
    } else {
        scroll_area_with_id("team-preset-picker-scroll", preset_grid)
    };
    div()
        .flex()
        .flex_col()
        .gap_3()
        .w_full()
        .child(content.flex_1().min_h_0())
}
