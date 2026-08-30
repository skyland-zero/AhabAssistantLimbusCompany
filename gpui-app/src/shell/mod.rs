mod title_bar;
mod toast;
#[cfg(target_os = "windows")]
mod windows;

pub(crate) const NATIVE_APP_TITLE: &str = "AALC·gpui";

pub use title_bar::title_bar;
pub use toast::{Toast, ToastKind, toast_layer};

#[cfg(target_os = "windows")]
pub use windows::{acquire_instance, is_window_minimized, start_tray};

#[cfg(not(target_os = "windows"))]
pub fn is_window_minimized(_window: &gpui::Window) -> bool {
    false
}
