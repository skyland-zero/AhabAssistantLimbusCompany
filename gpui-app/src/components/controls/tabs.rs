//! Segmented tab bars.
//!
//! GPUI Kit's `TabBar` is a *controlled* component: it takes `selected_index`
//! and reports clicks through `on_click`, so the page keeps owning which tab is
//! active. That is why it is used directly instead of a wrapper: AGENTS.md
//! section 4 requires the page state to own the selection rather than letting a
//! component keep a second copy.
//!
//! Using it also removes the hand-rolled `on_key_down` + activation-key pair
//! each call site used to carry next to its `on_click`; `TabBar` routes
//! pointer and keyboard activation through the one callback.

pub use gpui_component::tab::{Tab, TabBar};

use gpui::{ElementId, Styled as _, px};

/// A segmented bar with the skin's geometry applied.
///
/// The colour language comes from the projected theme; only the radius is
/// pinned here because the skins own their corner geometry and GPUI Kit's
/// default radius would square off Limbus and Mist only by accident.
pub fn segmented_tab_bar(id: impl Into<ElementId>, selected: usize) -> TabBar {
    let palette = crate::components::current_render_palette();
    TabBar::new(id)
        .segmented()
        .selected_index(selected)
        .rounded(px(palette.shape.radius_md as f32))
}
