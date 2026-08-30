//! Native, entity-backed text input used by the GPUI forms.
//!
//! GPUI deliberately leaves text editing policy to the application. This
//! control keeps the entity model, editing commands, IME bridge, and custom
//! element renderer in separate files while preserving the original public
//! `TextInput` API.

#![allow(dead_code)]

mod editing;
mod element;
mod input_handler;

use std::ops::Range;

use gpui::{
    App, Bounds, ClipboardItem, Context, FocusHandle, KeyBinding, ShapedLine, SharedString, Window,
    actions,
};

use super::style::{ColorToken, Palette, current_render_palette};

fn paint_color(token: ColorToken) -> gpui::Rgba {
    gpui::rgba(token.rgba_hex())
}

// These actions are bound once by main.rs and listened to by every input
// entity. Keeping them in the reusable control makes keyboard behavior the
// same in the team editor and in future settings/help inputs.
actions!(
    text_input,
    [
        Backspace,
        Delete,
        Left,
        Right,
        SelectLeft,
        SelectRight,
        SelectAll,
        Home,
        End,
        Paste,
        Cut,
        Copy,
    ]
);

pub struct TextInput {
    focus_handle: FocusHandle,
    content: SharedString,
    placeholder: SharedString,
    selected_range: Range<usize>,
    selection_reversed: bool,
    marked_range: Option<Range<usize>>,
    last_layout: Option<ShapedLine>,
    last_bounds: Option<Bounds<gpui::Pixels>>,
    is_selecting: bool,
    palette: Palette,
    disabled: bool,
    masked: bool,
}

impl TextInput {
    pub fn new(
        content: impl Into<SharedString>,
        placeholder: impl Into<SharedString>,
        cx: &mut Context<Self>,
    ) -> Self {
        Self::new_with_palette(content, placeholder, current_render_palette(), cx)
    }

    /// Construct an input using the palette derived from the root settings.
    /// The palette is copied into the entity so a root theme update can call
    /// [`Self::set_palette`] and immediately repaint this seam.
    pub fn new_with_palette(
        content: impl Into<SharedString>,
        placeholder: impl Into<SharedString>,
        palette: Palette,
        cx: &mut Context<Self>,
    ) -> Self {
        Self::new_with_palette_and_mask(content, placeholder, palette, false, cx)
    }

    /// Construct an input that stores the real value but paints asterisks.
    /// The mask keeps one byte per source byte so cursor and selection offsets
    /// remain valid for the ASCII credentials used by settings forms.
    pub fn new_masked_with_palette(
        content: impl Into<SharedString>,
        placeholder: impl Into<SharedString>,
        palette: Palette,
        cx: &mut Context<Self>,
    ) -> Self {
        Self::new_with_palette_and_mask(content, placeholder, palette, true, cx)
    }

    fn new_with_palette_and_mask(
        content: impl Into<SharedString>,
        placeholder: impl Into<SharedString>,
        palette: Palette,
        masked: bool,
        cx: &mut Context<Self>,
    ) -> Self {
        let content = content.into();
        let end = content.len();
        Self {
            focus_handle: cx.focus_handle(),
            content,
            placeholder: placeholder.into(),
            selected_range: end..end,
            selection_reversed: false,
            marked_range: None,
            last_layout: None,
            last_bounds: None,
            is_selecting: false,
            palette,
            disabled: false,
            masked,
        }
    }

    pub fn text(&self) -> String {
        self.content.to_string()
    }

    pub fn set_text(&mut self, text: impl Into<SharedString>) {
        self.content = text.into();
        let end = self.content.len();
        self.selected_range = end..end;
        self.selection_reversed = false;
        self.marked_range = None;
        self.last_layout = None;
        self.last_bounds = None;
    }

    pub fn set_palette(&mut self, palette: Palette) {
        self.palette = palette;
        self.last_layout = None;
    }

    pub fn palette(&self) -> Palette {
        self.palette
    }

    pub fn set_disabled(&mut self, disabled: bool) {
        self.disabled = disabled;
        self.is_selecting = false;
    }

    pub fn is_disabled(&self) -> bool {
        self.disabled
    }
}

// Keep this import available to callers that want to bind the standard input
// actions without knowing the macro-generated module details.
pub fn key_bindings() -> [KeyBinding; 12] {
    [
        KeyBinding::new("backspace", Backspace, None),
        KeyBinding::new("delete", Delete, None),
        KeyBinding::new("left", Left, None),
        KeyBinding::new("right", Right, None),
        KeyBinding::new("shift-left", SelectLeft, None),
        KeyBinding::new("shift-right", SelectRight, None),
        KeyBinding::new("cmd-a", SelectAll, None),
        KeyBinding::new("cmd-v", Paste, None),
        KeyBinding::new("cmd-c", Copy, None),
        KeyBinding::new("cmd-x", Cut, None),
        KeyBinding::new("home", Home, None),
        KeyBinding::new("end", End, None),
    ]
}

#[cfg(test)]
mod tests {
    use super::key_bindings;

    #[test]
    fn standard_key_bindings_cover_the_input_actions() {
        assert_eq!(key_bindings().len(), 12);
    }
}
