mod title_bar;
mod toast;
#[cfg(target_os = "windows")]
mod windows;

pub use title_bar::title_bar;
pub use toast::{Toast, ToastKind, toast_layer};

#[cfg(target_os = "windows")]
pub use windows::{acquire_instance, start_tray};
