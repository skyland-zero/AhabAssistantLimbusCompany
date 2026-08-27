mod help;
mod home;
mod resources;
mod settings;
mod teams;
mod theme_packs;
mod toolbox;

use gpui::{Context, Div};

use crate::app::{AhabApp, Page};

pub fn render(page: Page, app: &mut AhabApp, cx: &mut Context<AhabApp>) -> Div {
    match page {
        Page::Home => home::render(app, cx),
        Page::Teams => teams::render(app, cx),
        Page::ThemePacks => theme_packs::render(app, cx),
        Page::Toolbox => toolbox::render(app, cx),
        Page::Resources => resources::render(app, cx),
        Page::Settings => settings::render(app, cx),
        Page::Help => help::render(app, cx),
    }
}

pub fn render_overlay(page: Page, app: &mut AhabApp, cx: &mut Context<AhabApp>) -> Div {
    match page {
        Page::Teams => teams::render_overlay(app, cx),
        _ => gpui::div(),
    }
}
