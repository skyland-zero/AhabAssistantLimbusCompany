//! Entity-backed text input used by the GPUI forms.
//!
//! This used to be a hand-written control: a custom `Element` to shape and
//! paint the text, an `EntityInputHandler` implementation for the IME bridge,
//! and a separate editing-command module - 884 lines whose behaviour GPUI Kit's
//! `Input` already provides, including UTF-16 selection, IME composition,
//! clipboard, masked entry and the platform's text-input configuration.
//!
//! What remains is a thin, deliberately narrow wrapper. The type keeps the
//! `TextInput` name so `Entity<TextInput>` stays the handle pages pass around,
//! and it exposes only the operations this app actually calls: read the value
//! and replace it. `gpui_component::init` installs the input key bindings, so
//! the app no longer binds them itself.
//!
//! # Palette
//!
//! There is no palette argument any more. `Input` reads its colours from the
//! projected theme, which `components::style::kit` derives from the active
//! `Palette`, so a skin or accent change reaches every input without the app
//! pushing a new palette into each entity.

use gpui::{App, AppContext as _, Context, Entity, IntoElement, Render, Window};
use gpui_component::input::{Input, InputState};

pub struct TextInput {
    state: Entity<InputState>,
}

impl TextInput {
    /// A single-line input holding `content`, showing `placeholder` when empty.
    pub fn new(
        content: impl Into<gpui::SharedString>,
        placeholder: impl Into<gpui::SharedString>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        Self::build(content, placeholder, false, window, cx)
    }

    /// [`Self::new`] with the value masked, for credentials.
    pub fn new_masked(
        content: impl Into<gpui::SharedString>,
        placeholder: impl Into<gpui::SharedString>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        Self::build(content, placeholder, true, window, cx)
    }

    fn build(
        content: impl Into<gpui::SharedString>,
        placeholder: impl Into<gpui::SharedString>,
        masked: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let state = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder(placeholder)
                .default_value(content)
                .masked(masked)
        });
        Self { state }
    }

    /// The current value as an owned string.
    pub fn text(&self, cx: &App) -> String {
        self.state.read(cx).value().to_string()
    }

    /// Replace the whole value, leaving the caret at the end.
    pub fn set_text(
        &mut self,
        text: impl Into<gpui::SharedString>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.state
            .update(cx, |state, cx| state.set_value(text, window, cx));
    }
}

impl Render for TextInput {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        Input::new(&self.state)
    }
}
