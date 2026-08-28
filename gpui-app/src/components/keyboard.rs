use gpui::KeyDownEvent;

/// Return whether a focused action should be triggered by keyboard input.
pub fn is_activation_key(event: &KeyDownEvent) -> bool {
    matches!(
        event.keystroke.key.to_ascii_lowercase().as_str(),
        "enter" | "space"
    )
}
