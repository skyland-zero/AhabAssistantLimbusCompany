use super::*;

pub fn system_card(
    _app: &mut AhabApp,
    cx: &mut Context<AhabApp>,
    system: &crate::model::SystemSettingsConfig,
    language: Language,
) -> Div {
    let body = div()
        .flex()
        .flex_col()
        .gap(px(12.))
        .px_4()
        .pb_4()
        .child(settings_list(vec![
            setting_row(
                text("内存占用保护", "Memory Protection").get(language),
                text(
                    "电脑总内存占用超过 90% 时自动清理内存防崩溃",
                    "Clean memory automatically if overall RAM usage exceeds 90%",
                )
                .get(language),
                setting_switch(
                    cx,
                    SystemBool::MemoryProtection,
                    system.memory_protection,
                    "settings-memory",
                ),
            ),
            setting_row(
                text("最小化到托盘", "Minimize to Tray").get(language),
                text(
                    "窗口最小化时隐藏到系统托盘区",
                    "Hide window to system tray when minimized",
                )
                .get(language),
                setting_switch(
                    cx,
                    SystemBool::MinimizeToTray,
                    system.minimize_to_tray,
                    "settings-tray",
                ),
            ),
            setting_row(
                text("开机自动启动", "Start on Boot").get(language),
                text(
                    "跟随 Windows 系统开机自启 AALC",
                    "Launch AALC automatically when Windows boots",
                )
                .get(language),
                setting_switch(
                    cx,
                    SystemBool::Autostart,
                    system.autostart,
                    "settings-autostart",
                ),
            ),
        ]));
    settings_card(
        text("系统与防护", "System & Protection").get(language),
        body,
    )
}

pub fn experimental_card(
    _app: &mut AhabApp,
    cx: &mut Context<AhabApp>,
    system: &crate::model::SystemSettingsConfig,
    language: Language,
) -> Div {
    let body = div()
        .flex()
        .flex_col()
        .gap(px(12.))
        .px_4()
        .pb_4()
        .child(settings_list(vec![
            setting_row(
                text("运行时阻止休眠", "Prevent System Sleep").get(language),
                text(
                    "任务执行期间阻止系统与显示器进入休眠，任务结束后自动恢复",
                    "Prevent sleep and display turn-off during task execution",
                )
                .get(language),
                setting_switch(
                    cx,
                    SystemBool::KeepScreenAwake,
                    system.experimental_keep_screen_awake,
                    "settings-awake",
                ),
            ),
            setting_row(
                text("显示器 HDR 检测提示", "Display HDR Detection").get(language),
                text(
                    "检测到游戏处于 HDR 显示器时提示潜在图像识别问题",
                    "Warn about potential visual recognition issues on HDR monitors",
                )
                .get(language),
                setting_switch(
                    cx,
                    SystemBool::HdrWarning,
                    system.experimental_hdr_warning,
                    "settings-hdr",
                ),
            ),
            setting_row(
                text("模板高斯模糊", "Template Gaussian Blur").get(language),
                text(
                    "抗 Scrcpy H.264 压缩噪点，Scrcpy 建议开启，MuMu 可关闭省 CPU",
                    "Anti H.264 blur for Scrcpy, on for Scrcpy, off to save CPU on MuMu",
                )
                .get(language),
                setting_switch(
                    cx,
                    SystemBool::TemplateBlur,
                    system.enable_template_blur,
                    "settings-template-blur",
                ),
            ),
        ]));
    settings_card(
        text("实验性功能", "Experimental Features").get(language),
        body,
    )
}
