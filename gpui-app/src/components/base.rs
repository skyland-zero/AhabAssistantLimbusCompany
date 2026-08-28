use super::*;

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
        ButtonVariant::Ghost => (palette.card, palette.foreground, palette.muted),
        ButtonVariant::Destructive => (palette.danger, palette.brand_foreground, palette.danger),
        ButtonVariant::Icon => (palette.card, palette.foreground, palette.secondary),
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
    let label: String = label.into();
    if !label.is_empty() {
        control = control.child(label);
    }
    control
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
