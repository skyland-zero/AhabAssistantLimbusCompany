//! App identity and platform facts.
//!
//! The smallest export of the crate: it proves the Rust ↔ TSX seam works
//! (`App.tsx` reads the version it shows in the title bar from here) and gives
//! the ported modules a place to land in P1.

use nsh_core::prelude::*;

/// App identity and platform facts.
#[module]
pub struct Runtime;

#[module]
impl Runtime {
    /// The backend crate's version.
    ///
    /// Kept in step with `nsh.json` `"version"` by the release script; the
    /// value here is what the running binary was built with.
    pub fn app_version() -> String {
        env!("CARGO_PKG_VERSION").to_owned()
    }

    /// The operating system the app runs on: `windows`, `macos` or `linux`.
    pub fn platform_os() -> String {
        std::env::consts::OS.to_owned()
    }

    /// On Windows 10/11, opts the top-level frameless window into DWM desktop
    /// alpha compositing (`DwmEnableBlurBehindWindow`), eliminating solid black
    /// corners outside the card's rounded radius. Returns true if applied.
    pub fn enable_transparency() -> bool {
        crate::window::enable_transparency_opt_in()
    }

    /// On Windows, sets or clears a GDI rounded clipping region on the top-level
    /// window to eliminate DirectComposition edge artifacts on Windows 10.
    /// Pass 0.0 or negative to restore the standard rectangular window (e.g. when maximized).
    pub fn apply_rounded_corners(radius_pt: f64) -> bool {
        crate::window::apply_rounded_window_corners(radius_pt as f32)
    }
}

#[cfg(test)]
mod tests {
    use super::Runtime;

    #[test]
    fn version_is_not_empty() {
        assert!(!Runtime::app_version().is_empty());
    }

    #[test]
    fn platform_is_one_of_the_desktops() {
        assert!(["windows", "macos", "linux"].contains(&Runtime::platform_os().as_str()));
    }
}
