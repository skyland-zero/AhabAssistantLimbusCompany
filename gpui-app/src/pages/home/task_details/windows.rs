use super::super::*;

pub fn set_windows_details(app: &mut AhabApp, cx: &mut Context<AhabApp>, busy: bool) -> Div {
    let config = &app.home.tasks.set_windows;
    let language = app.state.settings.language;
    let tab = app.home.options_tab(FixedTaskId::SetWindows);
    let position = home_select(
        app,
        cx,
        HomeSelectConfig {
            select: HomeSelect::WindowPosition,
            current: config.set_win_position.clone(),
            options: vec![
                (
                    "center".to_owned(),
                    text("屏幕居中", "Center").get(language).to_owned(),
                ),
                (
                    "left_top".to_owned(),
                    text("靠左对齐", "Align Left").get(language).to_owned(),
                ),
                (
                    "right_top".to_owned(),
                    text("靠右对齐", "Align Right").get(language).to_owned(),
                ),
                (
                    "free".to_owned(),
                    text("保持原位 (不移动)", "Keep Current (Do Not Move)")
                        .get(language)
                        .to_owned(),
                ),
            ],
            id: "window-position".to_owned(),
            width: 144.,
            disabled: busy,
            on_change: Rc::new(|home, value| home.set_window_position(value)),
        },
    );
    let size_button = home_select(
        app,
        cx,
        HomeSelectConfig {
            select: HomeSelect::WindowResolution,
            current: config.set_win_size.to_string(),
            options: vec![
                ("720".to_owned(), "1280 × 720".to_owned()),
                ("1080".to_owned(), "1920 × 1080".to_owned()),
                ("1440".to_owned(), "2560 × 1440".to_owned()),
            ],
            id: "window-size".to_owned(),
            width: 144.,
            disabled: busy,
            on_change: Rc::new(|home, value| {
                if let Ok(value) = value.parse::<u16>() {
                    home.set_window_size(value);
                }
            }),
        },
    );
    let restore_switch = task_option_switch(
        "",
        config.set_reduce_miscontact,
        "reduce-miscontact",
        busy,
        cx,
        |home| {
            home.update_tasks(|tasks| {
                tasks.set_windows.set_reduce_miscontact = !tasks.set_windows.set_reduce_miscontact
            })
        },
    );
    let screenshot_button = home_select(
        app,
        cx,
        HomeSelectConfig {
            select: HomeSelect::ScreenshotInterval,
            current: format!("{:.1}", config.screenshot_interval),
            options: vec![
                (
                    "0.1".to_owned(),
                    format!("0.1s ({})", text("极速", "Fast").get(language)),
                ),
                (
                    "0.2".to_owned(),
                    "0.2s".to_owned(),
                ),
                (
                    "0.5".to_owned(),
                    format!("0.5s ({})", text("默认", "Default").get(language)),
                ),
                (
                    "1.0".to_owned(),
                    format!("1.0s ({})", text("较慢", "Slow").get(language)),
                ),
            ],
            id: "screenshot-interval".to_owned(),
            width: 144.,
            disabled: busy,
            on_change: Rc::new(|home, value| {
                if let Ok(value) = value.parse::<f32>() {
                    home.set_screenshot_interval(value);
                }
            }),
        },
    );
    let mouse_button = home_select(
        app,
        cx,
        HomeSelectConfig {
            select: HomeSelect::MouseInterval,
            current: format!("{:.1}", config.mouse_action_interval),
            options: vec![
                ("0.1".to_owned(), "0.1s".to_owned()),
                (
                    "0.3".to_owned(),
                    format!("0.3s ({})", text("默认", "Default").get(language)),
                ),
                ("0.5".to_owned(), "0.5s".to_owned()),
            ],
            id: "mouse-interval".to_owned(),
            width: 144.,
            disabled: busy,
            on_change: Rc::new(|home, value| {
                if let Ok(value) = value.parse::<f32>() {
                    home.set_mouse_action_interval(value);
                }
            }),
        },
    );
    let post_message = task_option_switch(
        "",
        config.use_post_message,
        "post-message",
        busy,
        cx,
        |home| {
            home.update_tasks(|tasks| {
                tasks.set_windows.use_post_message = !tasks.set_windows.use_post_message
            })
        },
    );

    let content = match tab {
        TaskOptionsTab::General => settings_grid(
            vec![
                control_row(
                    text("窗口分辨率", "Window Resolution").get(language),
                    size_button,
                ),
                control_row(text("窗口位置", "Window Position").get(language), position),
                control_row(
                    text("结束后恢复窗口", "Restore Window on Finish").get(language),
                    restore_switch,
                ),
            ],
            220.,
        ),
        TaskOptionsTab::Advanced => settings_grid(
            vec![
                control_row(
                    text("截图间隔", "Screenshot Interval").get(language),
                    screenshot_button,
                ),
                control_row(
                    text("鼠标操作间隔", "Mouse Action Interval").get(language),
                    mouse_button,
                ),
                control_row(
                    text("异步 PostMessage 输入", "Async PostMessage Input").get(language),
                    post_message,
                ),
            ],
            220.,
        ),
    };
    div()
        .flex()
        .flex_col()
        .gap_2()
        .child(options_tabs(FixedTaskId::SetWindows, tab, language, cx))
        .child(content)
}
