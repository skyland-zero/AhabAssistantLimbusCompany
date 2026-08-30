use std::time::Duration;

use gpui::{Animation, AnimationExt, Context, Render, Window, div, prelude::*, px, rgba};

use crate::components::style::{Palette, palette_rgb};

/// The execution status dot is its own reactive boundary. Its repeating
/// animation can therefore invalidate only this small subtree instead of the
/// root application view.
pub(crate) struct StatusDot {
    busy: bool,
    paused: bool,
    palette: Palette,
}

impl StatusDot {
    pub(crate) fn new(busy: bool, paused: bool, palette: Palette) -> Self {
        Self {
            busy,
            paused,
            palette,
        }
    }

    pub(crate) fn sync_snapshot(&mut self, busy: bool, paused: bool, palette: Palette) {
        self.busy = busy;
        self.paused = paused;
        self.palette = palette;
    }
}

impl Render for StatusDot {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let dot_color = if self.busy {
            if self.paused {
                palette_rgb(self.palette.warning)
            } else {
                palette_rgb(self.palette.success)
            }
        } else {
            rgba(0)
        };
        let status_dot = div()
            .w(px(6.))
            .h(px(6.))
            .rounded_full()
            .bg(dot_color)
            .flex_none();

        if self.busy {
            status_dot
                .with_animation(
                    "titlebar-console-status-breathe",
                    Animation::new(Duration::from_millis(1400))
                        .repeat()
                        .with_max_fps(12.0),
                    |dot, progress| {
                        let opacity =
                            0.40 + 0.60 * (0.5 + 0.5 * (progress * std::f32::consts::TAU).sin());
                        dot.opacity(opacity)
                    },
                )
                .into_any_element()
        } else {
            status_dot.into_any_element()
        }
    }
}
