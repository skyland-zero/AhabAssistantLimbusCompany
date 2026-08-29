use super::super::*;

pub fn enkephalin_details(app: &mut AhabApp, cx: &mut Context<AhabApp>, busy: bool) -> Div {
    let config = &app.home.tasks.buy_enkephalin;
    let language = app.state.settings.language;
    let tab = app.home.options_tab(FixedTaskId::BuyEnkephalin);
    let number = task_counter(
        config.set_lunacy_to_enkephalin,
        0,
        10,
        "enkephalin-count",
        busy,
        cx,
        Rc::new(|home, delta| home.adjust_enkephalin_count(delta)),
    );
    let general = div().flex().flex_col().gap_2().child(control_row(
        text("换体次数（0~10）", "Refill Times (0-10)").get(language),
        number,
    ));

    let detail = detail_switch(
        config.Dr_Grandet_mode,
        FixedTaskId::BuyEnkephalin,
        cx,
        busy,
        "grandet-mode",
    );
    let skip = task_option_switch(
        "",
        config.skip_enkephalin,
        "skip-enkephalin",
        busy,
        cx,
        |home| {
            home.update_tasks(|tasks| {
                tasks.buy_enkephalin.skip_enkephalin = !tasks.buy_enkephalin.skip_enkephalin
            })
        },
    );
    let advanced = div().flex().flex_col().gap_2().child(settings_grid(
        vec![
            control_row(text("葛朗台模式", "Dr. Grandet Mode").get(language), detail),
            div()
                .pt_2()
                .border_t_1()
                .border_color(rgb(BORDER))
                .child(control_row(
                    text("跳过模块合成", "Skip Module Crafting").get(language),
                    skip,
                ))
                .child(
                    div().text_size(px(11.0)).text_color(rgb(TEXT_MUTED)).child(
                        text(
                            "除狂气换体外，不自动将多余体力合成为脑啡肽模块。",
                            "Do not convert surplus enkephalin into modules.",
                        )
                        .get(language),
                    ),
                ),
        ],
        220.,
    ));
    div()
        .flex()
        .flex_col()
        .gap_2()
        .child(options_tabs(
            FixedTaskId::BuyEnkephalin,
            tab,
            app.state.settings.language,
            cx,
        ))
        .child(match tab {
            TaskOptionsTab::General => general,
            TaskOptionsTab::Advanced => advanced,
        })
}
