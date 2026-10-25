#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]

// `gpui-kit` re-exports the GPUI snapshot at its crate root. Alias the facade
// so the existing `gpui::` imports keep resolving through the single
// dependency instead of requiring a mechanical rename in every module.
extern crate gpui_kit as gpui;

mod app;
mod app_inputs;
mod assets;
mod components;
mod i18n;
mod ipc;
mod kit_assets;
mod model;
mod pages;
mod shell;
mod state;
mod theme;

use gpui::{
    App, AppContext, Bounds, KeyBinding, TextRenderingMode, TitlebarOptions, WindowBounds,
    WindowDecorations, WindowOptions, px, size,
};
use gpui_kit::application;

use app::AhabApp;

gpui_kit::actions!(app_actions, [Quit]);

#[cfg(target_os = "windows")]
fn configure_windows_window(window: &gpui::Window) {
    use raw_window_handle::{HasWindowHandle, RawWindowHandle};

    if let Ok(handle) = <gpui::Window as HasWindowHandle>::window_handle(window)
        && let RawWindowHandle::Win32(win32_handle) = handle.as_raw()
    {
        let hwnd = win32_handle.hwnd.get() as *mut std::ffi::c_void;

        unsafe extern "system" {
            fn DwmSetWindowAttribute(
                hwnd: *mut std::ffi::c_void,
                dwAttribute: u32,
                pvAttribute: *const std::ffi::c_void,
                cbAttribute: u32,
            ) -> i32;
        }

        // On Windows 11 (Build 22000+), request native small rounded corners (4px).
        // On Windows 10, this is safely ignored by DWM with zero side-effects.
        const DWMWA_WINDOW_CORNER_PREFERENCE: u32 = 33;
        const DWMWCP_ROUNDSMALL: u32 = 3;
        let preference = DWMWCP_ROUNDSMALL;
        unsafe {
            let _ = DwmSetWindowAttribute(
                hwnd,
                DWMWA_WINDOW_CORNER_PREFERENCE,
                &preference as *const _ as *const _,
                std::mem::size_of::<u32>() as u32,
            );
        }
    }
}

fn main() {
    #[cfg(target_os = "windows")]
    let _instance = match shell::acquire_instance() {
        Ok(Some(instance)) => instance,
        Ok(None) => return,
        Err(error) => {
            eprintln!("{error}");
            return;
        }
    };
    #[cfg(target_os = "windows")]
    shell::start_tray();

    // `kit_assets::AppAssets` supplies the Lucide SVGs both the app and GPUI
    // Kit's widgets draw. The app's own artwork stays embedded via
    // `include_bytes!` in `assets`, so this source is additive.
    application()
        .with_assets(kit_assets::AppAssets)
        .run(|cx: &mut App| {
            cx.set_app_identity("com.kiyi671.ahab-gpui-app", shell::NATIVE_APP_TITLE);
            cx.set_text_rendering_mode(TextRenderingMode::Grayscale);
            // GPUI Kit owns widget state (overlays, menus, input focus rings), so
            // it has to be installed before the first window opens.
            gpui_kit::init(cx);
            cx.bind_keys([KeyBinding::new("cmd-q", Quit, None)]);

            let bounds = Bounds::centered(None, size(px(820.), px(680.)), cx);
            match gpui_kit::open_window(
                WindowOptions {
                    // `appears_transparent` hides the native Windows titlebar while
                    // retaining the platform window controls and resize frame.
                    titlebar: Some(TitlebarOptions {
                        title: Some(shell::NATIVE_APP_TITLE.into()),
                        appears_transparent: true,
                        ..Default::default()
                    }),
                    app_id: Some("com.kiyi671.ahab-gpui-app".into()),
                    focus: true,
                    window_bounds: Some(WindowBounds::Windowed(bounds)),
                    window_min_size: Some(size(px(800.), px(560.))),
                    // Client-side decorations are the GPUI equivalent of a
                    // borderless/self-drawn window on platforms that support them.
                    window_decorations: Some(WindowDecorations::Client),
                    ..Default::default()
                },
                cx,
                |window, cx| {
                    #[cfg(target_os = "windows")]
                    configure_windows_window(window);

                    // `gpui_kit::open_window` wraps this view in a Base `Root`,
                    // which hosts the dialog, sheet, notification and tooltip
                    // layers. The app keeps owning its own window content.
                    let view = cx.new(|cx| {
                        let mut app = AhabApp::new();
                        app.attach_window(window, cx);
                        app.start_event_pump(window, cx);
                        app.start_stats_ticker(cx);
                        cx.on_next_frame(window, |view, _window, cx| {
                            view.start_backend_bootstrap(cx);
                        });
                        app
                    });
                    let quit_handle = view.downgrade();
                    cx.on_action(move |_: &Quit, cx| {
                        if let Some(view) = quit_handle.upgrade() {
                            view.update(cx, |view, cx| view.request_app_exit(cx));
                        }
                    });
                    view
                },
            ) {
                Ok(_) => {}
                Err(error) => {
                    eprintln!("failed to open GPUI window: {error}");
                    return;
                }
            }

            cx.activate(true);
        });
}
