mod help;
mod home;
mod resources;
mod settings;
mod teams;
mod theme_packs;
mod toolbox;

pub(crate) use home::HomeViewRefs;

use gpui::{Context, Div, Window};

use crate::app::{AhabApp, Page};

pub fn render(
    page: Page,
    app: &mut AhabApp,
    window: &mut Window,
    cx: &mut Context<AhabApp>,
) -> Div {
    match page {
        Page::Home => home::render(app, window, cx),
        Page::Teams => teams::render(app, window, cx),
        Page::ThemePacks => theme_packs::render(app, window, cx),
        Page::Toolbox => toolbox::render(app, window, cx),
        Page::Resources => resources::render(app, window, cx),
        Page::Settings => settings::render(app, window, cx),
        Page::Help => help::render(app, window, cx),
    }
}

pub(crate) use home::{AfterCompletionView, DailyDetailsView, mirror_history_body};
pub(crate) use teams::{PresetPickerEntry, TeamEditorView, preset_picker_body};
