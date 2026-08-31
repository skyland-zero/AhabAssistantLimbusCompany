//! Reusable state-free controls. Pages own the model and event handlers;
//! these modules only render the shared visual primitives.

use super::*;

mod inputs;
mod select;
mod slider;
mod switch;
mod tabs;

#[allow(unused_imports)]
pub use inputs::{clamp_number, number_stepper, text_input, text_input_with_palette};
#[allow(unused_imports)]
pub use select::{select, select_option, select_popup, select_trigger, select_with_palette};
#[allow(unused_imports)]
pub use slider::{normalize_slider, slider, slider_with_palette};
#[allow(unused_imports)]
pub use switch::{switch, switch_accent, switch_with_palette};
#[allow(unused_imports)]
pub use tabs::{tab_surface_with_palette, tabs, tabs_with_palette};
