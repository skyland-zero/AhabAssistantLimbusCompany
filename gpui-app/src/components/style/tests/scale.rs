//! Source-level guards that keep the type scale and skin geometry
//! authoritative.
//!
//! The spacing/type tokens used to be decorative: 17 ad-hoc text sizes had
//! accumulated and the `FONT_*` constants were referenced once. These tests
//! make the scale authoritative by failing on any new off-scale literal,
//! which is far cheaper than migrating every call site to a constant.

use super::support::*;
use crate::components::style::{FONT_2XS, FONT_LG, FONT_MD, FONT_SM, FONT_XL, FONT_XS};

#[test]
fn type_scale_is_enforced_across_sources() {
    let scale = [FONT_2XS, FONT_XS, FONT_SM, FONT_MD, FONT_LG, FONT_XL];
    let mut offenders = Vec::new();
    for (path, source) in source_files("src") {
        let text = path.to_string_lossy().replace('\\', "/");
        if text.contains("components/style/") {
            // The scale itself lives here.
            continue;
        }
        for literal in literals_after(&source, "text_size(px(") {
            let parsed: f32 = literal.parse().unwrap_or(-1.0);
            if !scale.iter().any(|value| (value - parsed).abs() < 0.01) {
                offenders.push(format!("{}: text_size(px({literal}))", path.display()));
            }
        }
    }
    assert!(
        offenders.is_empty(),
        "off-scale text sizes (use a FONT_* token):\n{}",
        offenders.join("\n")
    );
}

/// Page roots must stay transparent so the root window `Div` and the skin's
/// artwork layer are visible. An opaque page root hid the limbus nebula plate
/// completely: it was painted every frame and never seen.
#[test]
fn page_roots_do_not_repaint_the_window_background() {
    // Only the exact expression `page_root()` used to carry is banned; small
    // elements legitimately paint `palette.background` (the skin preview
    // miniatures do).
    let opaque_root_patterns = [".bg(palette_rgb(current_render_palette().background))"];
    let mut offenders = Vec::new();
    for (path, source) in source_files("src/pages") {
        for (index, line) in source.lines().enumerate() {
            if line.trim_start().starts_with("//") {
                continue;
            }
            for needle in opaque_root_patterns {
                if line.contains(needle) {
                    offenders.push(format!("{}:{}: {}", path.display(), index + 1, line.trim()));
                }
            }
        }
    }
    assert!(
        offenders.is_empty(),
        "page roots repaint the window background and hide the skin artwork:\n{}",
        offenders.join("\n")
    );
    // Home's page root used the tagged compatibility constant instead.
    let home = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/pages/home/mod.rs"),
    )
    .unwrap_or_default();
    assert!(
        !home.contains("bg(rgb(BACKGROUND))\n        .child(left_panel)"),
        "the home page root must not paint an opaque background"
    );
}

#[test]
fn controls_do_not_hardcode_semantic_radii() {
    // Components must go through `ShapeExt` so a skin switch can square off
    // every control. `rounded_full` (pills) and `rounded_none` stay legal
    // because they are geometry-independent.
    let banned = [
        ".rounded_sm()",
        ".rounded_md()",
        ".rounded_lg()",
        ".rounded_xl()",
    ];
    let mut offenders = Vec::new();
    for (path, source) in source_files("src/components") {
        let text = path.to_string_lossy().replace('\\', "/");
        if text.contains("components/style/") {
            // The trait and its documentation live here.
            continue;
        }
        for (index, line) in source.lines().enumerate() {
            if line.trim_start().starts_with("//") {
                continue;
            }
            for needle in banned {
                if line.contains(needle) {
                    offenders.push(format!("{}:{}: {}", path.display(), index + 1, line.trim()));
                }
            }
        }
    }
    assert!(
        offenders.is_empty(),
        "components hardcode a skin radius (use .skin_rounded(..)):\n{}",
        offenders.join("\n")
    );
}
