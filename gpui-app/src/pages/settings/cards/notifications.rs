use super::*;

pub fn notification_card(
    _app: &mut AhabApp,
    cx: &mut Context<AhabApp>,
    spt_input: Option<gpui::Entity<TextInput>>,
    language: Language,
) -> Div {
    let mut save = button(
        text("保存 SPT", "Save SPT").get(language),
        ButtonVariant::Default,
    )
    .id("settings-save-wxpusher-spt")
    .px_3()
    .py_1()
    .text_size(px(12.));
    save = save
        .on_click(cx.listener(|view, _, _, cx| view.save_settings_wxpusher_spt(cx)))
        .on_key_down(cx.listener(|view, event: &KeyDownEvent, window, cx| {
            if is_activation_key(event) {
                window.prevent_default();
                view.save_settings_wxpusher_spt(cx);
            }
        }));

    let test_input = spt_input.clone();
    let key_input = spt_input.clone();
    let mut test = action_button(
        text("发送测试通知", "Send Test").get(language),
        ButtonVariant::Outline,
        None,
        28.,
    )
    .id("settings-test-wxpusher")
    .on_click(cx.listener(move |view, _, _, cx| {
        let spt = test_input
            .as_ref()
            .map(|input| input.read(cx).text())
            .unwrap_or_default();
        view.settings_page.test_notification(spt);
        cx.notify();
    }));
    test = test.on_key_down(cx.listener(move |view, event: &KeyDownEvent, window, cx| {
        if is_activation_key(event) {
            window.prevent_default();
            let spt = key_input
                .as_ref()
                .map(|input| input.read(cx).text())
                .unwrap_or_default();
            view.settings_page.test_notification(spt);
            cx.notify();
        }
    }));

    let input = spt_input
        .map(|input| div().w(px(220.)).child(input))
        .unwrap_or_else(|| div().child(text("初始化中", "Initializing…").get(language)));
    let body = div()
        .flex()
        .flex_col()
        .gap(px(12.))
        .px_3p5()
        .pb_3p5()
        .child(setting_row(
            text("WxPusher SPT", "WxPusher SPT").get(language),
            text(
                "配置个人 SPT 后接收任务完成与失败摘要，不发送原始日志",
                "Configure a personal SPT for task summaries; raw logs are never sent",
            )
            .get(language),
            div()
                .flex()
                .items_center()
                .gap_2()
                .child(input)
                .child(save)
                .child(test),
        ));
    settings_card(text("任务通知", "Task Notifications").get(language), body)
}
