use gpui::AnyElement;

use super::*;

use crate::app::AhabApp;
use gpui_component::Sizable as _;
use gpui_component::empty::{Empty, EmptyDescription, EmptyHeader, EmptyMedia, EmptyTitle};
use gpui_component::skeleton::Skeleton;
use gpui_component::spinner::Spinner;

// NOTE: the `dialog` / `dialog_overlay` helpers that used to live here were
// removed as dead code. Pages build their overlays inline (a scrim `div()` plus
// a `card`), so there was no call site left. Moving those overlays onto GPUI
// Kit's `Root::open_dialog` is the migration that would replace them; it is not
// done yet, and until it is, the inline overlays are the only implementation.

// NOTE: these scroll areas are deliberately kept. GPUI already draws a native
// scrollbar and `.scrollbar_width(...)` / `.track_scroll(...)` already style
// and track it, so GPUI Kit's `Scrollable` would be cosmetic. It also owns its
// `ScrollHandle` in the window's keyed state, which would take the handle away
// from the help page - that page drives the table of contents with
// `scroll_to_top_of_item()` and tracks the active section through
// `top_item()`, so handing the handle to the element would be a functional
// regression in exchange for a different scrollbar look.

pub fn scroll_area(child: impl IntoElement) -> Stateful<Div> {
    scroll_area_base(
        "scroll-area",
        child,
        &current_render_palette(),
        ControlState::default(),
    )
}

/// Scroll container with a caller-provided stable GPUI id for repeated lists.
/// GPUI owns the complete native wheel and trackpad scrolling path.
pub fn scroll_area_with_id(
    _app: &mut AhabApp,
    id: &'static str,
    child: impl IntoElement,
) -> Stateful<Div> {
    scroll_area_base(
        id,
        child,
        &current_render_palette(),
        ControlState::default(),
    )
}

pub fn scroll_area_with_handle_children(
    _app: &mut AhabApp,
    id: &'static str,
    children: impl IntoIterator<Item = Div>,
    handle: gpui::ScrollHandle,
) -> Stateful<Div> {
    scroll_area_base_without_child(id, &current_render_palette(), ControlState::default())
        .children(children)
        .track_scroll(&handle)
}

fn scroll_area_base(
    id: &'static str,
    child: impl IntoElement,
    palette: &Palette,
    state: ControlState,
) -> Stateful<Div> {
    scroll_area_base_without_child(id, palette, state).child(child)
}

fn scroll_area_base_without_child(
    id: &'static str,
    palette: &Palette,
    state: ControlState,
) -> Stateful<Div> {
    let focus_ring = palette.ring;
    let mut area = div()
        .id(id)
        .min_w_0()
        .overflow_y_scroll()
        .focus_visible(move |style| style.border_color(paint_color(focus_ring)));
    if state.disabled {
        area = area.opacity(0.5);
    }
    area
}

pub fn empty_state(title: impl Into<String>, detail: impl Into<String>) -> Div {
    let palette = current_render_palette();
    let mut header = EmptyHeader::new()
        .title(EmptyTitle::new().child(title.into()))
        .description(EmptyDescription::new().child(detail.into()));
    if let Some(marker) = skin_empty_marker(&palette) {
        header = header.media(EmptyMedia::new().child(marker));
    }
    // `Empty` sets a dashed border *style* but no border width, so nothing is
    // painted and the skins keep their own border language (Limbus is square
    // and borderless, Archive uses hairlines).
    div().child(Empty::new().header(header))
}

/// The per-skin marker that stands in for an illustration.
///
/// Each decoration language gets its own: Limbus stamps its wax seal, Archive
/// leaves a ruled blank, Glass a raised chip, Mist a thin bar, and the flat
/// skins nothing at all. No skin invents text or geometry that would shift the
/// surrounding layout.
fn skin_empty_marker(palette: &Palette) -> Option<AnyElement> {
    let marker = match palette.decor {
        Decor::LimbusFrame => gpui::img(crate::assets::image_source(crate::assets::theme(
            crate::assets::ThemeAsset::Seal,
        )))
        .w(px(72.))
        .h(px(72.))
        .opacity(0.85)
        .into_any_element(),
        Decor::Archive => div()
            .w(px(64.))
            .h(px(2.))
            .bg(paint_color(palette.decor_line))
            .opacity(0.5)
            .into_any_element(),
        Decor::Glass => div()
            .w(px(44.))
            .h(px(44.))
            .skin_rounded(palette, true)
            .border_1()
            .border_color(paint_color(palette.hilite))
            .bg(paint_color(palette.secondary))
            .into_any_element(),
        Decor::Mist => div()
            .w(px(28.))
            .h(px(2.))
            .bg(paint_color(palette.decor_line))
            .opacity(0.7)
            .into_any_element(),
        Decor::Plain => return None,
    };
    Some(marker)
}

/// A spinner with its label.
///
/// GPUI Kit's `Spinner` drives its rotation through `with_animation`, so the
/// indicator is a real animation instead of a static glyph. The return type
/// stays `Div` so call sites are unaffected.
pub fn loading(label: impl Into<String>) -> Div {
    let palette = current_render_palette();
    div()
        .flex()
        .items_center()
        .gap_2()
        .text_color(paint_color(palette.muted_foreground))
        .child(
            Spinner::new()
                .with_size(px(14.))
                .color(palette_hsla(palette.brand)),
        )
        .child(label.into())
}

/// A pulsing placeholder.
///
/// GPUI Kit's `Skeleton` owns the animation and reads `theme.skeleton`, which
/// the bridge derives from `Palette::muted`, so the skin still controls the
/// fill. Only the size and radius are overridden.
pub fn skeleton(width: gpui::Pixels, height: gpui::Pixels) -> Div {
    let palette = current_render_palette();
    div().w(width).h(height).child(
        Skeleton::new()
            .w_full()
            .h_full()
            .rounded(px(palette.shape.radius_sm as f32)),
    )
}

/// The design-system lane intentionally keeps palette construction independent
/// from the root `theme` module, so old pages can compile before the root is
/// rewired. This helper makes the canonical accent parser available to callers
/// that only import components.
pub fn parse_accent_id(id: &str) -> AccentId {
    AccentId::parse(id)
}
