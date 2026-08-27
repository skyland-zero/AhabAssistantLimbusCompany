mod app;
mod assets;
mod components;
mod i18n;
mod ipc;
mod model;
mod pages;
mod shell;
mod state;
mod theme;

use gpui::{
    App, AppContext, Bounds, KeyBinding, TitlebarOptions, WindowBounds, WindowDecorations,
    WindowOptions, actions, px, size,
};
use gpui_platform::application;

use app::AhabApp;

actions!(app_actions, [Quit]);

fn main() {
    application().run(|cx: &mut App| {
        cx.on_action(|_: &Quit, cx| cx.quit());
        cx.bind_keys([KeyBinding::new("cmd-q", Quit, None)]);
        cx.bind_keys(components::text_input::key_bindings());

        let bounds = Bounds::centered(None, size(px(900.), px(680.)), cx);
        match cx.open_window(
            WindowOptions {
                // `appears_transparent` hides the native Windows titlebar while
                // retaining the platform window controls and resize frame.
                titlebar: Some(TitlebarOptions {
                    title: Some(
                        crate::i18n::text(crate::model::Language::ZhCn, crate::i18n::Key::AppName)
                            .into(),
                    ),
                    appears_transparent: true,
                    ..Default::default()
                }),
                focus: true,
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                window_min_size: Some(size(px(800.), px(560.))),
                // Client-side decorations are the GPUI equivalent of a
                // borderless/self-drawn window on platforms that support them.
                window_decorations: Some(WindowDecorations::Client),
                ..Default::default()
            },
            |_, cx| {
                cx.new(|cx| {
                    let mut app = AhabApp::new();
                    app.start_event_pump(cx);
                    app
                })
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
