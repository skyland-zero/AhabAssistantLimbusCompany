#![allow(dead_code)]

//! The window's `AssetSource`: the icons this app draws plus GPUI Kit's bundle.
//!
//! GPUI resolves an `IconName` by loading its SVG through the window's
//! `AssetSource`, and a path the source does not know about renders as an empty
//! square with no error. `gpui_component_assets::Assets` only embeds the ~100
//! icons GPUI Kit's own widgets need, so pointing the window at it alone made
//! every page icon disappear while the surrounding text kept rendering.
//!
//! `icon_assets!` embeds exactly the icons listed below and returns `Ok(None)`
//! for everything else, so this module composes the two: the app's icons win,
//! and anything the app does not name falls through to the component bundle.
//! That keeps the binary to the icons the UI actually uses instead of the full
//! 1830-icon catalog.
//!
//! # Keeping the list honest
//!
//! Adding `IconName::Foo` to a page without adding `Foo` to the list below is a
//! silent regression - the icon simply stops drawing. `every_referenced_icon_is_embedded`
//! scans the sources for `IconName::` identifiers and compares them against the
//! paths this source actually embeds, so the list cannot drift.

use std::borrow::Cow;

use gpui::{AssetSource, Result, SharedString};
use gpui_component_assets::{Assets as ComponentAssets, IconName, icon_assets};

// The icons the app names. `every_referenced_icon_is_embedded` proves this list
// matches the sources.
icon_assets!(
    AppIcons,
    [
        ArrowUpDown,
        CalendarCheck,
        Camera,
        Check,
        ChevronDown,
        ChevronUp,
        CircleAlert,
        CircleQuestionMark,
        ClipboardPaste,
        Close,
        Compass,
        Copy,
        Crosshair,
        ExternalLink,
        Gift,
        House,
        Loader,
        LoaderCircle,
        Minus,
        Monitor,
        MonitorPlay,
        Moon,
        Package,
        Palette,
        Pause,
        Pencil,
        Pill,
        Play,
        Plus,
        Radio,
        RefreshCw,
        RotateCcw,
        RotateCcwClock,
        RotateCw,
        ScrollText,
        SearchCheck,
        Settings,
        SlidersHorizontal,
        Smartphone,
        Sparkles,
        Square,
        SquareCheck,
        Sun,
        Trash,
        TriangleAlert,
        Users,
        WindowRestore,
        Wrench,
        X,
        Zap,
    ]
);

/// The composed source installed via `Application::with_assets`.
#[derive(Clone, Copy, Debug, Default)]
pub struct AppAssets;

impl AssetSource for AppAssets {
    fn load(&self, path: &str) -> Result<Option<Cow<'static, [u8]>>> {
        match AppIcons.load(path)? {
            Some(data) => Ok(Some(data)),
            None => ComponentAssets.load(path),
        }
    }

    fn list(&self, path: &str) -> Result<Vec<SharedString>> {
        let mut paths = AppIcons.list(path)?;
        paths.extend(ComponentAssets.list(path)?);
        paths.dedup();
        Ok(paths)
    }
}

/// Resolve a name the way GPUI will at paint time.
///
/// Exposed so tests can assert a name is reachable through the composed source
/// rather than merely being a well-formed path.
pub fn resolve(name: IconName) -> Option<Cow<'static, [u8]>> {
    AppAssets.load(&name.path()).ok().flatten()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every `IconName::X` written in the sources must be embedded here.
    ///
    /// The failure this guards is silent: an unembedded icon renders as an
    /// empty square, with no error and nothing in a log. Only comparing the
    /// referenced identifiers against the source's own path list catches it.
    #[test]
    fn every_referenced_icon_is_embedded() {
        let embedded: std::collections::BTreeSet<String> = AppIcons
            .list("icons/")
            .expect("the embedded source must list its icons")
            .iter()
            .filter_map(|path| path.rsplit('/').next())
            .filter_map(|file| file.strip_suffix(".svg"))
            .map(kebab_to_pascal)
            .collect();

        let mut missing = std::collections::BTreeSet::new();
        let mut referenced = 0;
        for (path, source) in crate_sources() {
            for ident in referenced_icon_idents(&source) {
                referenced += 1;
                if !embedded.contains(&ident) {
                    missing.insert(format!("{path}: IconName::{ident}"));
                }
            }
        }

        assert!(
            referenced > 0,
            "the source scan found no IconName references"
        );
        assert!(
            missing.is_empty(),
            "named in the UI but not embedded in `AppIcons`:\n{}",
            missing.into_iter().collect::<Vec<_>>().join("\n")
        );
    }

    /// Read every Rust source in the crate, relative to `src/`.
    ///
    /// Skips this module: its own `IconName::` mentions are the test fixtures
    /// and the component-bundle check, which are deliberately outside the
    /// embedded list.
    fn crate_sources() -> Vec<(String, String)> {
        fn walk(dir: &std::path::Path, out: &mut Vec<(String, String)>) {
            let Ok(entries) = std::fs::read_dir(dir) else {
                return;
            };
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    walk(&path, out);
                } else if path.extension().is_some_and(|e| e == "rs") {
                    let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
                        continue;
                    };
                    if name == "kit_assets.rs" {
                        continue;
                    }
                    let Ok(source) = std::fs::read_to_string(&path) else {
                        continue;
                    };
                    out.push((path.display().to_string(), source));
                }
            }
        }
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
        let mut out = Vec::new();
        walk(&root, &mut out);
        out
    }

    /// The component bundle must stay reachable, or GPUI Kit's own widgets lose
    /// their icons even though the app's are present.
    #[test]
    fn component_bundle_stays_reachable() {
        // An icon the app does not list, so it can only come from the bundle.
        let component_only = IconName::ArrowLeft;
        assert!(
            AppIcons.load(&component_only.path()).unwrap().is_none(),
            "ArrowLeft is expected to come from the component bundle, not AppIcons"
        );
        assert!(
            resolve(component_only).is_some(),
            "the component bundle must still resolve its own icons"
        );
    }

    /// Extract the identifiers from `IconName::Ident` mentions in a source.
    fn referenced_icon_idents(source: &str) -> Vec<String> {
        source
            .match_indices("IconName::")
            .map(|(index, _)| {
                let rest = &source[index + "IconName::".len()..];
                let end = rest
                    .find(|c: char| !c.is_alphanumeric())
                    .unwrap_or(rest.len());
                rest[..end].to_owned()
            })
            .collect()
    }

    /// `rotate-ccw-clock` -> `RotateCcwClock`, matching the generated variant.
    fn kebab_to_pascal(stem: &str) -> String {
        stem.split('-')
            .filter(|word| !word.is_empty())
            .map(|word| {
                let mut chars = word.chars();
                match chars.next() {
                    Some(first) => first.to_ascii_uppercase().to_string() + chars.as_str(),
                    None => String::new(),
                }
            })
            .collect()
    }

    #[test]
    fn kebab_conversion_matches_the_catalog() {
        assert_eq!(kebab_to_pascal("rotate-ccw-clock"), "RotateCcwClock");
        assert_eq!(
            kebab_to_pascal("circle-question-mark"),
            "CircleQuestionMark"
        );
        assert_eq!(kebab_to_pascal("x"), "X");
        assert_eq!(kebab_to_pascal("square-check"), "SquareCheck");
    }
}
