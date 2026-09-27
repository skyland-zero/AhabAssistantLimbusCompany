use super::*;
use crate::components::{Icon, IconName};

pub(super) const RIGHT_PANEL_DEFAULT_WIDTH: f32 = 280.0;
pub(super) const RIGHT_PANEL_MIN_WIDTH: f32 = 280.0;
pub(super) const RIGHT_PANEL_MAX_WIDTH: f32 = 800.0;
pub(super) const SPLITTER_WIDTH: f32 = 4.0;
pub(super) const SPLITTER_COLLAPSED_WIDTH: f32 = 16.0;
pub(super) const SPLITTER_COLLAPSE_THRESHOLD: f32 = 160.0;
pub(super) const MIN_LEFT_PANEL_WIDTH: f32 = 460.0;

#[allow(dead_code)]
pub(super) struct SplitterDragGhost;

impl Render for SplitterDragGhost {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div().w(px(1.0)).h(px(1.0)).bg(rgb(ACCENT))
    }
}

pub(super) fn action_icon(name: impl Into<Icon>, size: f32, color: u32) -> Icon {
    crate::components::svg_icon(name, size, color)
}

pub(super) fn brand_action_icon(name: impl Into<Icon>, size: f32) -> Icon {
    crate::components::svg_icon_colored(
        name,
        size,
        palette_rgb(current_render_palette().brand_foreground),
    )
}

/// Icon shown on a task card header, chosen by the spec's short label.
///
/// Kept as a label lookup rather than storing an `IconName` on every
/// `TaskCardSpec`, because the label is also the stable identity used by the
/// layout tests.
pub(super) fn task_icon_name(label: &str) -> IconName {
    match label {
        "SET" => IconName::SlidersHorizontal,
        "CAL" => IconName::CalendarCheck,
        "GFT" => IconName::Gift,
        "ZAP" => IconName::Zap,
        "CMP" => IconName::Compass,
        "RAD" => IconName::Radio,
        _ => IconName::SlidersHorizontal,
    }
}
