use super::*;

use crate::app::AhabApp;

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

pub fn scroll_area_with_handle(
    _app: &mut AhabApp,
    id: &'static str,
    child: impl IntoElement,
    handle: gpui::ScrollHandle,
) -> Stateful<Div> {
    scroll_area_base(
        id,
        child,
        &current_render_palette(),
        ControlState::default(),
    )
    .track_scroll(&handle)
}

/// Scroll container whose direct children are tracked individually by GPUI.
///
/// `ScrollHandle::scroll_to_top_of_item` indexes only direct children of the
/// tracked element. Pages that need stable section anchors should use this
/// constructor instead of wrapping all sections in one extra `Div`.
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

pub fn scroll_area_with_palette(
    id: &'static str,
    child: impl IntoElement,
    palette: &Palette,
    state: ControlState,
) -> Stateful<Div> {
    scroll_area_base(id, child, palette, state)
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
    // Limbus skin stamps the blood seal above the copy, like the wax marks
    // on in-game notices. Modern skin keeps the plain centered copy.
    let mut root = div()
        .flex()
        .flex_col()
        .items_center()
        .justify_center()
        .gap_2()
        .p_6()
        .text_color(paint_color(palette.muted_foreground));
    if palette.skin.is_limbus() {
        root = root.child(
            gpui::img(crate::assets::image_source(crate::assets::theme(
                crate::assets::ThemeAsset::LimbusSeal,
            )))
            .w(px(72.))
            .h(px(72.))
            .opacity(0.85),
        );
    }
    root.child(
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
