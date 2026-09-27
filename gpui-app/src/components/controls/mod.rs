//! Reusable state-free controls. Pages own the model and event handlers;
//! these modules only render the shared visual primitives.

use super::style::ShapeExt;
use super::*;

mod inputs;
mod select;
mod slider;
mod switch;
mod tabs;

#[allow(unused_imports)]
pub use inputs::{clamp_number, number_stepper, text_input, text_input_with_palette};
#[allow(unused_imports)]
pub use select::{
    select, select_keyboard_index, select_option, select_options_state, select_popup,
    select_trigger, select_with_palette,
};
#[allow(unused_imports)]
pub use slider::{normalize_slider, slider, slider_with_palette};
#[allow(unused_imports)]
pub use switch::{Switch, switch, switch_accent, switch_with_palette};
#[allow(unused_imports)]
pub use tabs::{Tab, TabBar, segmented_tab_bar};
