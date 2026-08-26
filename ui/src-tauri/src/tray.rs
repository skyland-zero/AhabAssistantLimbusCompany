use std::sync::atomic::{AtomicBool, Ordering};
use tauri::{
    menu::{Menu, MenuItem, PredefinedMenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, Emitter, Manager, Wry,
};

static MINIMIZE_TO_TRAY: AtomicBool = AtomicBool::new(true);

pub fn set_minimize_to_tray(enabled: bool) {
    MINIMIZE_TO_TRAY.store(enabled, Ordering::SeqCst);
}

pub fn get_minimize_to_tray() -> bool {
    MINIMIZE_TO_TRAY.load(Ordering::SeqCst)
}

pub fn init_tray(app: &AppHandle) -> Result<(), Box<dyn std::error::Error>> {
    // 1. 创建托盘菜单项
    let show_i = MenuItem::with_id(app, "show", "打开主窗口", true, None::<&str>)?;
    let sep1 = PredefinedMenuItem::separator(app)?;
    let start_i = MenuItem::with_id(app, "start", "开始任务 (Link Start!)", true, None::<&str>)?;
    let stop_i = MenuItem::with_id(app, "stop", "停止任务 (Stop!)", true, None::<&str>)?;
    let sep2 = PredefinedMenuItem::separator(app)?;
    let quit_i = MenuItem::with_id(app, "quit", "退出 AALC", true, None::<&str>)?;

    let menu = Menu::with_items(app, &[&show_i, &sep1, &start_i, &stop_i, &sep2, &quit_i])?;

    // 2. 获取程序图标
    let icon = app.default_window_icon().cloned().unwrap();

    // 3. 构建托盘图标并绑定事件
    TrayIconBuilder::<Wry>::new()
        .icon(icon)
        .tooltip("Ahab Assistant · Limbus Company")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "show" => show_main_window(app),
            "start" => {
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.emit("tray-start-tasks", ());
                }
            }
            "stop" => {
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.emit("tray-stop-tasks", ());
                }
            }
            "quit" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            // 左键单击直接恢复并前置主窗口
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                show_main_window(tray.app_handle());
            }
        })
        .build(app)?;

    Ok(())
}

pub fn show_main_window(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
    }
}
