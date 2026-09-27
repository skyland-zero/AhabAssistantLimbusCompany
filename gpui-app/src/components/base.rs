use gpui::{ElementId, SharedString};
use gpui_component::Disableable as _;
use gpui_component::button::{Button, ButtonVariants as _};
use gpui_component::tag::Tag;

use super::style::FONT_SM;
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

impl ButtonVariant {
    /// Project the app's variant vocabulary onto GPUI Kit's.
    ///
    /// `Icon` maps to `Ghost` because the app draws icon buttons on a card
    /// surface, so "same colour as the parent until hover" is the same visual
    /// intent; GPUI Kit then sizes the padding for a label-less button itself.
    fn kit(self) -> gpui_component::button::ButtonVariant {
        use gpui_component::button::ButtonVariant as Kit;
        match self {
            Self::Default => Kit::Primary,
            Self::Outline => Kit::Default,
            Self::Secondary => Kit::Secondary,
            Self::Ghost | Self::Icon => Kit::Ghost,
            Self::Destructive => Kit::Danger,
            Self::Link => Kit::Link,
        }
    }

    /// GPUI Kit splits the variant and the outline flag; the app folds both
    /// into one name, so the border is re-applied here.
    const fn outlined(self) -> bool {
        matches!(self, Self::Outline)
    }

    const fn is_icon(self) -> bool {
        matches!(self, Self::Icon)
    }
}

/// A clickable button surface.
///
/// `id` is required because GPUI Kit keys the button's focus handle by it;
/// two buttons sharing an id would share one focus handle and break Tab order.
/// Add `.on_click(...)` at the call site when the action is known; this keeps
/// the primitive independent of application state.
pub fn button(
    id: impl Into<ElementId>,
    label: impl Into<SharedString>,
    variant: ButtonVariant,
) -> Button {
    button_inner(id.into(), label.into(), variant, ControlState::default())
}

/// [`button`] with an explicit disabled/loading state.
pub fn button_with_state(
    id: impl Into<ElementId>,
    label: impl Into<SharedString>,
    variant: ButtonVariant,
    state: ControlState,
) -> Button {
    button_inner(id.into(), label.into(), variant, state)
}

fn button_inner(
    id: ElementId,
    label: SharedString,
    variant: ButtonVariant,
    state: ControlState,
) -> Button {
    // The skins own the control geometry, so the app keeps its compact sizing
    // and the theme bridge supplies every colour. GPUI Kit re-applies the
    // caller's style last (`refine_style(&instance_style)`), so these win over
    // its own size defaults.
    let palette = current_render_palette();
    let mut control = Button::new(id)
        .label(label)
        .with_variant(variant.kit())
        .h(px(FONT_SM + CONTROL_HEIGHT_PADDING))
        .px_3()
        .py_0()
        .text_size(px(FONT_SM))
        .rounded(px(palette.shape.radius_md as f32));
    if variant.outlined() {
        control = control.outline();
    }
    if variant.is_icon() {
        control = control.px_2().compact();
    }
    control.disabled(state.is_inert()).loading(state.loading)
}

/// Vertical padding added to the label size to reach the app's control height.
/// Keeps the previous `py_2` rhythm without pinning a second magic number at
/// every call site.
const CONTROL_HEIGHT_PADDING: f32 = 9.0;

/// A compact label chip.
///
/// GPUI Kit's same-named `Badge` is a count/dot overlay, so the chip maps to
/// `Tag` instead. `Tag::custom` carries the palette's already contrast-tested
/// background/foreground pair through unchanged.
pub fn badge(label: impl Into<String>, tone: BadgeTone) -> Tag {
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
) -> Tag {
    let (background, foreground) = match tone {
        BadgeTone::Neutral => (palette.muted, palette.muted_foreground),
        BadgeTone::Accent => (palette.brand_light, palette.brand),
        BadgeTone::Success => (palette.success_light, palette.success),
        BadgeTone::Warning => (palette.warning_light, palette.warning),
        BadgeTone::Info => (palette.brand_light, palette.brand),
        BadgeTone::Danger => (palette.danger_light, palette.danger),
    };

    let control = Tag::custom(
        palette_hsla(background),
        palette_hsla(foreground),
        palette_hsla(palette.border),
    )
    .child(label.into())
    .text_size(px(FONT_SM))
    .rounded(px(palette.shape.radius_sm as f32));
    if state.disabled {
        control.opacity(0.5)
    } else {
        control
    }
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
    // Positioned ancestor for the absolute decoration layers below.
    let mut surface = div().min_w_0().p_4().relative();
    surface = shape_rounded(surface, palette.shape.radius_lg);
    surface = surface
        .bg(paint_color(palette.card))
        .text_color(paint_color(palette.card_foreground))
        .focus_visible(move |style| style.border_color(paint_color(focus_ring)));

    // The browser token was transparent globally, which left dark-mode cards
    // without any outline. Every skin that defines a visible border gets one.
    if !palette.border.is_transparent() {
        surface = surface.border_1().border_color(paint_color(palette.border));
    }
    if state.focused {
        surface = surface.border_1().border_color(paint_color(palette.ring));
    }
    if !palette.glow.is_transparent() {
        surface = apply_card_shadow(
            surface,
            palette.shape.shadow,
            Some(palette_hsla(palette.glow)),
        );
    } else {
        surface = apply_card_shadow(surface, palette.shape.shadow, None);
    }

    if state.interactive && !state.disabled {
        let hover = paint_color(palette.secondary);
        surface = surface.cursor_pointer().hover(move |style| style.bg(hover));
    }
    if state.disabled {
        surface = surface.opacity(0.5);
    }
    let mut surface = surface.child(child);
    match palette.decor {
        Decor::LimbusFrame => {
            for bracket in frame_corner_brackets(palette) {
                surface = surface.child(bracket);
            }
        }
        Decor::Glass => {
            // A one-pixel highlight along the top edge is what makes a
            // translucent panel read as glass rather than as a flat tint.
            if !palette.hilite.is_transparent() {
                surface = surface.child(
                    div()
                        .absolute()
                        .top_0()
                        .left_0()
                        .right_0()
                        .h(px(1.))
                        .rounded_t(px(palette.shape.radius_lg as f32))
                        .bg(paint_color(palette.hilite)),
                );
            }
        }
        Decor::Plain | Decor::Archive | Decor::Mist => {}
    }
    surface
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::components::style::{AccentId, ColorScheme, SkinId};

    #[test]
    fn every_skin_and_accent_can_build_a_card_and_header() {
        for skin in SkinId::ALL {
            for scheme in [ColorScheme::Light, ColorScheme::Dark] {
                let palette = Palette::for_skin(scheme, AccentId::Crimson, skin);
                crate::components::style::set_current_render_palette(palette);
                let _ = card_header("Header", &palette);
                let _ = card(div().child("body"));
                let _ = rule(&palette);
                let _ = button("test-ok", "Ok", ButtonVariant::Default);
                let _ = button("test-delete", "Delete", ButtonVariant::Destructive);
                let _ = badge("tag", BadgeTone::Danger);
            }
        }
    }

    #[test]
    fn brackets_are_only_meaningful_for_the_frame_skin() {
        let limbus = Palette::for_skin(ColorScheme::Dark, AccentId::LimbusBrass, SkinId::Limbus);
        assert!(limbus.uses_frame_decor());
        let glass = Palette::for_skin(ColorScheme::Dark, AccentId::Crimson, SkinId::Glass);
        assert!(!glass.uses_frame_decor());
        assert_eq!(frame_corner_brackets(&limbus).len(), 4);
    }
}
