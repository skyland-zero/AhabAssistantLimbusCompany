mod tray;

#[tauri::command]
fn set_minimize_to_tray(enabled: bool) {
    tray::set_minimize_to_tray(enabled);
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            tray::init_tray(app.handle())?;
            Ok(())
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                if tray::get_minimize_to_tray() {
                    let _ = window.hide();
                    api.prevent_close();
                }
            }
        })
        .invoke_handler(tauri::generate_handler![set_minimize_to_tray])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
