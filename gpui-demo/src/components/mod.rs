//! Small, page-agnostic GPUI controls used by the shell and pages.
//!
//! The short functions (`button`, `card`, and friends) are compatibility
//! constructors for the existing pages. New pages should pass a derived
//! [`Palette`] to the `*_with_palette` constructors and use [`ControlState`]
//! for disabled/loading/focus-visible rendering.

pub mod icon;
pub mod style;
pub mod text_input;

pub use style::{current_render_palette, palette_rgb, render_rgb, render_rgba};
pub use text_input::TextInput;

use gpui::{Div, Rgba, Stateful, div, prelude::*, px};

use icon::{Icon, icon};
use style::{
    ACCENT, AccentId, BACKGROUND, BORDER, ColorToken, DANGER, GREEN, Palette, SURFACE,
    SURFACE_HOVER, TEXT, TEXT_MUTED,
};

fn paint_color(token: ColorToken) -> Rgba {
    gpui::rgba(token.rgba_hex())
}

/// Common visual state for controls that do not own their interaction model.
/// The page/entity remains responsible for event handlers; this value only
/// describes the state that should be painted.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ControlState {
    pub disabled: bool,
    pub loading: bool,
    /// Set this when a parent owns focus and wants the same focus-ring
    /// treatment as GPUI's `focus-visible` style.
    pub focused: bool,
}

impl ControlState {
    pub const fn disabled() -> Self {
        Self {
            disabled: true,
            loading: false,
            focused: false,
        }
    }

    pub const fn loading() -> Self {
        Self {
            disabled: false,
            loading: true,
            focused: false,
        }
    }

    pub const fn is_inert(self) -> bool {
        self.disabled || self.loading
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum ButtonVariant {
    #[default]
    Default,
    Outline,
    Secondary,
    Ghost,
    Destructive,
    Icon,
    Link,
}

/// A clickable button surface. Add `.on_click(...)` at the call site when the
/// action is known; this keeps the primitive independent of application state.
pub fn button(label: impl Into<String>, variant: ButtonVariant) -> Div {
    button_with_palette(
        label,
        variant,
        &current_render_palette(),
        ControlState::default(),
    )
}

pub fn button_with_palette(
    label: impl Into<String>,
    variant: ButtonVariant,
    palette: &Palette,
    state: ControlState,
) -> Div {
    let (background, foreground, hover_background) = match variant {
        ButtonVariant::Default => (palette.brand, palette.brand_foreground, palette.brand_hover),
        ButtonVariant::Outline => (palette.card, palette.foreground, palette.brand_light),
        ButtonVariant::Secondary => (
            palette.secondary,
            palette.secondary_foreground,
            palette.accent_surface,
        ),
        ButtonVariant::Ghost => (palette.background, palette.foreground, palette.muted),
        ButtonVariant::Destructive => (palette.danger, palette.brand_foreground, palette.danger),
        ButtonVariant::Icon => (
            palette.secondary,
            palette.foreground,
            palette.accent_surface,
        ),
        ButtonVariant::Link => (palette.background, palette.brand, palette.brand_light),
    };

    let focus_ring = palette.ring;
    let mut control = div()
        .flex()
        .items_center()
        .justify_center()
        .gap_2()
        .px_4()
        .py_2()
        .rounded_md()
        .tab_index(0)
        .border_1()
        .border_color(paint_color(if matches!(variant, ButtonVariant::Outline) {
            palette.input
        } else {
            palette.border
        }))
        .bg(paint_color(background))
        .text_color(paint_color(foreground))
        .focus_visible(move |style| style.border_color(paint_color(focus_ring)));

    if matches!(variant, ButtonVariant::Icon) {
        control = control.px_2().py_2();
    }

    if state.focused {
        control = control.border_color(paint_color(palette.ring));
    }

    if state.is_inert() {
        control = control.opacity(0.5);
    } else {
        let hover_background = paint_color(hover_background);
        control = control
            .cursor_pointer()
            .hover(move |style| style.bg(hover_background));
    }

    if state.loading {
        control = control.child(icon(Icon::LoaderCircle, px(14.), paint_color(foreground)));
    }
    control.child(label.into())
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum BadgeTone {
    #[default]
    Neutral,
    Accent,
    Success,
    Warning,
    Info,
    Danger,
}

pub fn badge(label: impl Into<String>, tone: BadgeTone) -> Div {
    badge_with_palette(
        label,
        tone,
        &current_render_palette(),
        ControlState::default(),
    )
}

pub fn badge_with_palette(
    label: impl Into<String>,
    tone: BadgeTone,
    palette: &Palette,
    state: ControlState,
) -> Div {
    let (background, foreground) = match tone {
        BadgeTone::Neutral => (palette.muted, palette.muted_foreground),
        BadgeTone::Accent => (palette.brand_light, palette.brand),
        BadgeTone::Success => (palette.success_light, palette.success),
        BadgeTone::Warning => (palette.warning_light, palette.warning),
        BadgeTone::Info => (palette.brand_light, palette.brand),
        BadgeTone::Danger => (palette.danger_light, palette.danger),
    };

    let mut control = div()
        .flex()
        .items_center()
        .px_2()
        .py_1()
        .rounded_md()
        .bg(paint_color(background))
        .text_color(paint_color(foreground))
        .text_size(px(11.));
    if state.disabled {
        control = control.opacity(0.5);
    }
    control.child(label.into())
}

/// State used by a card that is also a clickable/focusable surface.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct CardState {
    pub interactive: bool,
    pub disabled: bool,
    pub focused: bool,
}

/// A surface container with the shared radius and padding.
pub fn card(child: impl IntoElement) -> Div {
    card_with_palette(child, &current_render_palette())
}

pub fn card_with_palette(child: impl IntoElement, palette: &Palette) -> Div {
    card_with_state(child, palette, CardState::default())
}

pub fn card_with_state(child: impl IntoElement, palette: &Palette, state: CardState) -> Div {
    let focus_ring = palette.ring;
    let mut surface = div()
        .min_w_0()
        .p_4()
        .rounded_lg()
        .bg(paint_color(palette.card))
        .text_color(paint_color(palette.card_foreground))
        .focus_visible(move |style| style.border_color(paint_color(focus_ring)));

    // The browser token is transparent globally. A focused card still gets a
    // visible ring, while a caller can opt into a subtle interactive hover.
    if !palette.border.is_transparent() {
        surface = surface.border_1().border_color(paint_color(palette.border));
    }
    if state.focused {
        surface = surface.border_1().border_color(paint_color(palette.ring));
    }
    if state.interactive && !state.disabled {
        let hover = paint_color(palette.secondary);
        surface = surface.cursor_pointer().hover(move |style| style.bg(hover));
    }
    if state.disabled {
        surface = surface.opacity(0.5);
    }
    surface.child(child)
}

/// A compact on/off control. State changes are supplied by the caller (usually
/// an Entity); this helper only renders the current state.
pub fn switch(checked: bool) -> Div {
    switch_with_palette(checked, &current_render_palette(), ControlState::default())
}

pub fn switch_with_palette(checked: bool, palette: &Palette, state: ControlState) -> Div {
    let track = if checked {
        palette.brand
    } else {
        palette.input
    };
    let thumb = if checked {
        palette.brand_foreground
    } else {
        palette.card
    };
    let hover_track = if checked {
        palette.brand_hover
    } else {
        palette.accent_surface
    };
    let focus_ring = palette.ring;

    let mut control = div()
        .flex()
        .items_center()
        .w(px(38.))
        .h(px(22.))
        .p(px(3.))
        .rounded_lg()
        .tab_index(0)
        .border_1()
        .border_color(paint_color(palette.input))
        .bg(paint_color(track))
        .focus_visible(move |style| style.border_color(paint_color(focus_ring)));
    if checked {
        control = control.justify_end();
    }
    if state.focused {
        control = control.border_color(paint_color(palette.ring));
    }
    if state.is_inert() {
        control = control.opacity(0.5);
    } else {
        let hover_track = paint_color(hover_track);
        control = control
            .cursor_pointer()
            .hover(move |style| style.bg(hover_track));
    }
    control.child(
        div()
            .w(px(16.))
            .h(px(16.))
            .rounded_lg()
            .bg(paint_color(thumb)),
    )
}

/// Select surface. Menu ownership and keyboard navigation stay with the page
/// entity; this primitive provides the field, focus ring, loading state, and a
/// real Lucide chevron rather than a Unicode glyph.
pub fn select(label: impl Into<String>, options: &[&str], selected: usize) -> Div {
    select_with_palette(
        label,
        options,
        selected,
        &current_render_palette(),
        ControlState::default(),
    )
}

pub fn select_with_palette(
    label: impl Into<String>,
    options: &[&str],
    selected: usize,
    palette: &Palette,
    state: ControlState,
) -> Div {
    let selected = options.get(selected).copied().unwrap_or_default();
    let focus_ring = palette.ring;
    let mut control = div()
        .flex()
        .items_center()
        .justify_between()
        .w_full()
        .px_3()
        .py_2()
        .rounded_md()
        .tab_index(0)
        .border_1()
        .border_color(paint_color(palette.input))
        .bg(paint_color(palette.card))
        .text_color(paint_color(palette.foreground))
        .focus_visible(move |style| style.border_color(paint_color(focus_ring)));
    if state.focused {
        control = control.border_color(paint_color(palette.ring));
    }
    if state.is_inert() {
        control = control.opacity(0.5);
    } else {
        let hover = paint_color(palette.accent_surface);
        control = control.cursor_pointer().hover(move |style| style.bg(hover));
    }

    let mut value = div().flex().items_center().gap_2();
    if state.loading {
        value = value.child(icon(
            Icon::LoaderCircle,
            px(14.),
            paint_color(palette.muted_foreground),
        ));
    }
    value = value.child(selected.to_owned());

    control
        .child(
            div()
                .text_color(paint_color(palette.muted_foreground))
                .child(label.into()),
        )
        .child(div().flex().items_center().gap_2().child(value).child(icon(
            Icon::ChevronDown,
            px(14.),
            paint_color(palette.muted_foreground),
        )))
}

/// Render a bounded slider. Use [`normalize_slider`] for the model value
/// before passing it here. The knob is deliberately static; callers provide
/// pointer/keyboard updates to the canonical model.
pub fn slider(value: f32, min: f32, max: f32) -> Div {
    slider_with_palette(
        value,
        min,
        max,
        &current_render_palette(),
        ControlState::default(),
    )
}

pub fn slider_with_palette(
    value: f32,
    min: f32,
    max: f32,
    palette: &Palette,
    state: ControlState,
) -> Div {
    let normalized = normalize_slider(value, min, max);
    let width = 180.0;
    let knob_left = (width * normalized - 5.0).clamp(0.0, width - 10.0);
    let mut track = div()
        .relative()
        .w(px(width))
        .h(px(6.))
        .rounded_md()
        .tab_index(0)
        .bg(paint_color(palette.input))
        .focus_visible({
            let ring = palette.ring;
            move |style| style.border_color(paint_color(ring))
        })
        .child(
            div()
                .h(px(6.))
                .w(px(width * normalized))
                .rounded_md()
                .bg(paint_color(palette.brand)),
        )
        .child(
            div()
                .absolute()
                .left(px(knob_left))
                .top(px(-2.))
                .w(px(10.))
                .h(px(10.))
                .rounded_lg()
                .bg(paint_color(palette.brand)),
        );
    if state.focused {
        track = track.border_1().border_color(paint_color(palette.ring));
    }
    if state.is_inert() {
        track = track.opacity(0.5);
    }
    track
}

pub fn normalize_slider(value: f32, min: f32, max: f32) -> f32 {
    if !value.is_finite() || !min.is_finite() || !max.is_finite() || max <= min {
        return 0.0;
    }
    ((value - min) / (max - min)).clamp(0.0, 1.0)
}

pub fn tabs(labels: &[&str], selected: usize) -> Div {
    tabs_with_palette(labels, selected, &current_render_palette(), &[])
}

/// Render tabs with per-tab disabled state. An empty `disabled` slice keeps
/// every tab enabled for the legacy constructor.
pub fn tabs_with_palette(
    labels: &[&str],
    selected: usize,
    palette: &Palette,
    disabled: &[bool],
) -> Div {
    let mut root = div().flex().items_center().gap_1();
    for (index, label) in labels.iter().enumerate() {
        let active = index == selected;
        let is_disabled = disabled.get(index).copied().unwrap_or(false);
        let focus_ring = palette.ring;
        let mut tab = div()
            .id(format!("tab-{index}"))
            .tab_index(index as isize)
            .px_3()
            .py_2()
            .rounded_md()
            .border_1()
            .border_color(paint_color(if active {
                palette.brand
            } else {
                palette.border
            }))
            .text_size(px(12.))
            .text_color(paint_color(if active {
                palette.foreground
            } else {
                palette.muted_foreground
            }))
            .bg(paint_color(if active {
                palette.brand_light
            } else {
                palette.card
            }))
            .focus_visible(move |style| style.border_color(paint_color(focus_ring)));
        if is_disabled {
            tab = tab.opacity(0.5);
        } else {
            let hover = paint_color(palette.accent_surface);
            tab = tab.cursor_pointer().hover(move |style| style.bg(hover));
        }
        tab = tab.child((*label).to_owned());
        root = root.child(tab);
    }
    root
}

pub fn dialog(title: impl Into<String>, body: impl IntoElement, actions: impl IntoElement) -> Div {
    dialog_with_palette(
        title,
        body,
        actions,
        &current_render_palette(),
        ControlState::default(),
    )
}

pub fn dialog_with_palette(
    title: impl Into<String>,
    body: impl IntoElement,
    actions: impl IntoElement,
    palette: &Palette,
    state: ControlState,
) -> Div {
    let mut surface = card_with_state(
        div()
            .flex()
            .flex_col()
            .gap_3()
            .child(
                div()
                    .text_size(px(16.))
                    .text_color(paint_color(palette.foreground))
                    .child(title.into()),
            )
            .child(body)
            .child(div().flex().justify_end().gap_2().child(actions)),
        palette,
        CardState {
            interactive: false,
            disabled: state.disabled,
            focused: state.focused,
        },
    );
    surface = surface
        .border_1()
        .border_color(paint_color(if state.focused {
            palette.ring
        } else {
            palette.input
        }));
    surface
}

/// Center a dialog and paint a modal scrim. The owner should close it on Esc,
/// restore focus to the triggering entity, and attach confirmation handlers;
/// GPUI cannot infer those application actions from an ordinary `Div`.
pub fn dialog_overlay(child: impl IntoElement, palette: &Palette) -> Div {
    div()
        .absolute()
        .top_0()
        .left_0()
        .size_full()
        .flex()
        .items_center()
        .justify_center()
        .p_4()
        .bg(gpui::rgba(0x00000080))
        .child(child)
        .text_color(paint_color(palette.foreground))
}

pub fn scroll_area(child: impl IntoElement) -> Stateful<Div> {
    scroll_area_with_palette(
        "scroll-area",
        child,
        &current_render_palette(),
        ControlState::default(),
    )
}

/// Scroll container with a caller-provided stable GPUI id for repeated lists.
/// GPUI owns wheel/trackpad behavior; the thin scrollbar is a platform paint
/// concern, so callers should keep this as the single scroll boundary.
pub fn scroll_area_with_id(id: &'static str, child: impl IntoElement) -> Stateful<Div> {
    scroll_area_with_palette(
        id,
        child,
        &current_render_palette(),
        ControlState::default(),
    )
}

pub fn scroll_area_with_palette(
    id: &'static str,
    child: impl IntoElement,
    palette: &Palette,
    state: ControlState,
) -> Stateful<Div> {
    let focus_ring = palette.ring;
    let mut area = div()
        .id(id)
        .min_w_0()
        .overflow_y_scroll()
        .scrollbar_width(px(6.))
        .focus_visible(move |style| style.border_color(paint_color(focus_ring)))
        .child(child);
    if state.disabled {
        area = area.opacity(0.5);
    }
    area
}

/// A visual seam for simple text labels. The entity-backed [`TextInput`] below
/// is the control to use when IME, selection, and clipboard behavior matters.
pub fn text_input(value: &str, placeholder: &str) -> Div {
    text_input_with_palette(
        value,
        placeholder,
        &current_render_palette(),
        ControlState::default(),
    )
}

pub fn text_input_with_palette(
    value: &str,
    placeholder: &str,
    palette: &Palette,
    state: ControlState,
) -> Div {
    let shown = if value.is_empty() { placeholder } else { value };
    let color = if value.is_empty() {
        palette.muted_foreground
    } else {
        palette.foreground
    };
    let mut control = div()
        .w_full()
        .px_3()
        .py_2()
        .rounded_md()
        .border_1()
        .border_color(paint_color(if state.focused {
            palette.ring
        } else {
            palette.input
        }))
        .bg(paint_color(palette.card))
        .text_color(paint_color(color));
    if state.disabled {
        control = control.opacity(0.5);
    }
    control.child(shown.to_owned())
}

pub fn number_stepper(value: i32, min: i32, max: i32) -> Div {
    let value = clamp_number(value, min, max);
    div()
        .flex()
        .items_center()
        .justify_between()
        .px_2()
        .py_1()
        .rounded_md()
        .border_1()
        .border_color(paint_color(current_render_palette().input))
        .bg(paint_color(current_render_palette().card))
        .text_color(paint_color(current_render_palette().foreground))
        .child("-")
        .child(value.to_string())
        .child("+")
}

pub fn clamp_number(value: i32, min: i32, max: i32) -> i32 {
    if min > max {
        min
    } else {
        value.clamp(min, max)
    }
}

pub fn empty_state(title: impl Into<String>, detail: impl Into<String>) -> Div {
    let palette = current_render_palette();
    div()
        .flex()
        .flex_col()
        .items_center()
        .justify_center()
        .gap_2()
        .p_6()
        .text_color(paint_color(palette.muted_foreground))
        .child(
            div()
                .text_color(paint_color(palette.foreground))
                .child(title.into()),
        )
        .child(detail.into())
}

pub fn loading(label: impl Into<String>) -> Div {
    let palette = current_render_palette();
    div()
        .flex()
        .items_center()
        .gap_2()
        .text_color(paint_color(palette.muted_foreground))
        .child(icon(
            Icon::LoaderCircle,
            px(14.),
            paint_color(palette.brand),
        ))
        .child(label.into())
}

pub fn skeleton(width: gpui::Pixels, height: gpui::Pixels) -> Div {
    div()
        .w(width)
        .h(height)
        .rounded_md()
        .bg(paint_color(current_render_palette().muted))
}

/// The design-system lane intentionally keeps palette construction independent
/// from the root `theme` module, so old pages can compile before the root is
/// rewired. This helper makes the canonical accent parser available to callers
/// that only import components.
pub fn parse_accent_id(id: &str) -> AccentId {
    AccentId::parse(id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slider_normalization_is_bounded() {
        assert_eq!(normalize_slider(-1.0, 0.0, 10.0), 0.0);
        assert_eq!(normalize_slider(5.0, 0.0, 10.0), 0.5);
        assert_eq!(normalize_slider(11.0, 0.0, 10.0), 1.0);
        assert_eq!(normalize_slider(5.0, 1.0, 1.0), 0.0);
    }

    #[test]
    fn number_stepper_normalization_is_bounded() {
        assert_eq!(clamp_number(-2, 0, 5), 0);
        assert_eq!(clamp_number(3, 0, 5), 3);
        assert_eq!(clamp_number(9, 0, 5), 5);
        assert_eq!(clamp_number(9, 5, 0), 5);
    }

    #[test]
    fn controls_have_stable_defaults_and_accents() {
        assert_eq!(ButtonVariant::default(), ButtonVariant::Default);
        assert_eq!(BadgeTone::default(), BadgeTone::Neutral);
        assert!(ControlState::loading().is_inert());
        assert_eq!(parse_accent_id("violet"), AccentId::Violet);
    }

    #[test]
    fn compatibility_palette_is_not_the_old_dark_only_palette() {
        assert_eq!(BACKGROUND, Palette::default().background.rgb_hex());
        assert_eq!(SURFACE, Palette::default().card.rgb_hex());
        assert_eq!(BORDER, Palette::default().input.rgb_hex());
        assert_ne!(BACKGROUND, 0x0f141c);
        assert_eq!(ACCENT, Palette::default().brand.rgb_hex());
        assert_eq!(GREEN, Palette::default().success.rgb_hex());
        assert_eq!(DANGER, Palette::default().danger.rgb_hex());
        assert_eq!(SURFACE_HOVER, Palette::default().secondary.rgb_hex());
        assert_eq!(TEXT, Palette::default().foreground.rgb_hex());
        assert_eq!(TEXT_MUTED, Palette::default().muted_foreground.rgb_hex());
    }
}
