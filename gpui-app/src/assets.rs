#![allow(dead_code)]

//! Asset identity, embedding, and dev/release resolution.
//!
//! Pages should request an [`Asset`] instead of joining paths. Fixed assets are
//! embedded from the canonical `gpui-app/resources/assets` tree so a release
//! cannot depend on the checkout layout. [`AssetResolver::resolve`]
//! additionally finds copied files for GPUI APIs that require a filesystem path
//! (and for future large assets). No absolute application path is encoded in
//! this module.

use std::{io, path::PathBuf, sync::Arc};

use crate::model::Language;

const APP_ROOT: &str = env!("CARGO_MANIFEST_DIR");

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum SinnerAsset {
    DonQuixote,
    Faust,
    Gregor,
    Heathcliff,
    HongLu,
    Ishmael,
    Meursault,
    Outis,
    Rodion,
    Ryoshu,
    Sinclair,
    YiSang,
}

impl SinnerAsset {
    pub const ALL: [Self; 12] = [
        Self::DonQuixote,
        Self::Faust,
        Self::Gregor,
        Self::Heathcliff,
        Self::HongLu,
        Self::Ishmael,
        Self::Meursault,
        Self::Outis,
        Self::Rodion,
        Self::Ryoshu,
        Self::Sinclair,
        Self::YiSang,
    ];

    pub const fn file_name(self) -> &'static str {
        match self {
            Self::DonQuixote => "don_quixote.png",
            Self::Faust => "faust.png",
            Self::Gregor => "gregor.png",
            Self::Heathcliff => "heathcliff.png",
            Self::HongLu => "hong_lu.png",
            Self::Ishmael => "ishmael.png",
            Self::Meursault => "meursault.png",
            Self::Outis => "outis.png",
            Self::Rodion => "rodion.png",
            Self::Ryoshu => "ryoshu.png",
            Self::Sinclair => "sinclair.png",
            Self::YiSang => "yi_sang.png",
        }
    }

    pub fn from_id(id: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|asset| asset.file_name().strip_suffix(".png") == Some(id))
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum StatusEffectAsset {
    Bleed,
    Blunt,
    Burn,
    Charge,
    General,
    Pierce,
    Poise,
    Rupture,
    Sinking,
    Slash,
    Tremor,
}

impl StatusEffectAsset {
    pub const ALL: [Self; 11] = [
        Self::Bleed,
        Self::Blunt,
        Self::Burn,
        Self::Charge,
        Self::General,
        Self::Pierce,
        Self::Poise,
        Self::Rupture,
        Self::Sinking,
        Self::Slash,
        Self::Tremor,
    ];

    pub const fn file_name(self) -> &'static str {
        match self {
            Self::Bleed => "bleed.png",
            Self::Blunt => "blunt.png",
            Self::Burn => "burn.png",
            Self::Charge => "charge.png",
            Self::General => "general.png",
            Self::Pierce => "pierce.png",
            Self::Poise => "poise.png",
            Self::Rupture => "rupture.png",
            Self::Sinking => "sinking.png",
            Self::Slash => "slash.png",
            Self::Tremor => "tremor.png",
        }
    }

    pub fn from_id(id: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|asset| asset.file_name().strip_suffix(".png") == Some(id))
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Asset {
    Logo,
    TitleBanner,
    Sinner(SinnerAsset),
    StatusEffect(StatusEffectAsset),
    Help(Language),
}

impl Asset {
    pub const fn help_for(language: Language) -> Self {
        Self::Help(language)
    }

    /// Relative path used by release resource bundles. This is always made of
    /// known file names, so a caller cannot inject `..` into the resolver.
    pub fn relative_path(self) -> PathBuf {
        match self {
            Self::Logo => PathBuf::from("logo.png"),
            Self::TitleBanner => PathBuf::from("limbus_title_banner.png"),
            Self::Sinner(sinner) => PathBuf::from("sinners").join(sinner.file_name()),
            Self::StatusEffect(effect) => PathBuf::from("status_effects").join(effect.file_name()),
            Self::Help(Language::ZhCn) => PathBuf::from("content").join("help-zh.md"),
            Self::Help(Language::EnUs) => PathBuf::from("content").join("help-en.md"),
        }
    }

    /// Embedded bytes for all currently shipped fixed assets.
    pub fn embedded(self) -> &'static [u8] {
        match self {
            Self::Logo => include_bytes!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/resources/assets/logo.png"
            )),
            Self::TitleBanner => include_bytes!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/resources/assets/limbus_title_banner.png"
            )),
            Self::Sinner(SinnerAsset::DonQuixote) => include_bytes!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/resources/assets/sinners/don_quixote.png"
            )),
            Self::Sinner(SinnerAsset::Faust) => include_bytes!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/resources/assets/sinners/faust.png"
            )),
            Self::Sinner(SinnerAsset::Gregor) => include_bytes!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/resources/assets/sinners/gregor.png"
            )),
            Self::Sinner(SinnerAsset::Heathcliff) => include_bytes!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/resources/assets/sinners/heathcliff.png"
            )),
            Self::Sinner(SinnerAsset::HongLu) => include_bytes!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/resources/assets/sinners/hong_lu.png"
            )),
            Self::Sinner(SinnerAsset::Ishmael) => include_bytes!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/resources/assets/sinners/ishmael.png"
            )),
            Self::Sinner(SinnerAsset::Meursault) => include_bytes!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/resources/assets/sinners/meursault.png"
            )),
            Self::Sinner(SinnerAsset::Outis) => include_bytes!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/resources/assets/sinners/outis.png"
            )),
            Self::Sinner(SinnerAsset::Rodion) => include_bytes!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/resources/assets/sinners/rodion.png"
            )),
            Self::Sinner(SinnerAsset::Ryoshu) => include_bytes!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/resources/assets/sinners/ryoshu.png"
            )),
            Self::Sinner(SinnerAsset::Sinclair) => include_bytes!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/resources/assets/sinners/sinclair.png"
            )),
            Self::Sinner(SinnerAsset::YiSang) => include_bytes!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/resources/assets/sinners/yi_sang.png"
            )),
            Self::StatusEffect(StatusEffectAsset::Bleed) => include_bytes!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/resources/assets/status_effects/bleed.png"
            )),
            Self::StatusEffect(StatusEffectAsset::Blunt) => include_bytes!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/resources/assets/status_effects/blunt.png"
            )),
            Self::StatusEffect(StatusEffectAsset::Burn) => include_bytes!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/resources/assets/status_effects/burn.png"
            )),
            Self::StatusEffect(StatusEffectAsset::Charge) => include_bytes!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/resources/assets/status_effects/charge.png"
            )),
            Self::StatusEffect(StatusEffectAsset::General) => include_bytes!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/resources/assets/status_effects/general.png"
            )),
            Self::StatusEffect(StatusEffectAsset::Pierce) => include_bytes!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/resources/assets/status_effects/pierce.png"
            )),
            Self::StatusEffect(StatusEffectAsset::Poise) => include_bytes!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/resources/assets/status_effects/poise.png"
            )),
            Self::StatusEffect(StatusEffectAsset::Rupture) => include_bytes!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/resources/assets/status_effects/rupture.png"
            )),
            Self::StatusEffect(StatusEffectAsset::Sinking) => include_bytes!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/resources/assets/status_effects/sinking.png"
            )),
            Self::StatusEffect(StatusEffectAsset::Slash) => include_bytes!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/resources/assets/status_effects/slash.png"
            )),
            Self::StatusEffect(StatusEffectAsset::Tremor) => include_bytes!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/resources/assets/status_effects/tremor.png"
            )),
            Self::Help(Language::ZhCn) => include_bytes!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/resources/assets/content/help-zh.md"
            )),
            Self::Help(Language::EnUs) => include_bytes!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/resources/assets/content/help-en.md"
            )),
        }
    }
}

/// Resolve assets from an optional override, the checked-out GPUI resources
/// in debug builds, and conventional release resource directories near the
/// executable.
#[derive(Clone, Debug)]
pub struct AssetResolver {
    roots: Vec<PathBuf>,
}

impl Default for AssetResolver {
    fn default() -> Self {
        Self::new()
    }
}

impl AssetResolver {
    pub fn new() -> Self {
        Self::from_roots(default_roots())
    }

    pub fn from_root(root: impl Into<PathBuf>) -> Self {
        Self::from_roots([root.into()])
    }

    pub fn from_roots(roots: impl IntoIterator<Item = PathBuf>) -> Self {
        Self {
            roots: roots.into_iter().collect(),
        }
    }

    pub fn roots(&self) -> &[PathBuf] {
        &self.roots
    }

    /// Find a copied file for APIs that need a path. Embedded assets remain the
    /// preferred path for fixed images and documents.
    pub fn resolve(&self, asset: Asset) -> Option<PathBuf> {
        let relative = asset.relative_path();
        self.roots
            .iter()
            .map(|root| root.join(&relative))
            .find(|candidate| candidate.is_file())
    }

    pub fn path(&self, asset: Asset) -> Option<PathBuf> {
        self.resolve(asset)
    }

    pub fn load(&self, asset: Asset) -> io::Result<Vec<u8>> {
        if let Some(bytes) = embedded_if_available(asset) {
            return Ok(bytes.to_vec());
        }
        let Some(path) = self.resolve(asset) else {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                format!("asset not found: {:?}", asset.relative_path()),
            ));
        };
        std::fs::read(path)
    }
}

fn embedded_if_available(asset: Asset) -> Option<&'static [u8]> {
    // All assets currently shipped by GPUI are fixed and embedded. Keeping
    // this helper separate allows a future large, downloaded asset to use only
    // the release resolver without changing callers.
    Some(asset.embedded())
}

fn default_roots() -> Vec<PathBuf> {
    let mut roots = Vec::new();
    if let Some(root) = std::env::var_os("AHAB_ASSET_DIR") {
        roots.push(PathBuf::from(root));
    }

    let manifest = PathBuf::from(APP_ROOT);
    if cfg!(debug_assertions) {
        roots.push(manifest.join("resources").join("assets"));
    }

    if let Ok(executable) = std::env::current_exe()
        && let Some(directory) = executable.parent()
    {
        roots.push(directory.join("resources"));
        roots.push(directory.join("resources").join("assets"));
        roots.push(directory.join("assets"));
        roots.push(directory.to_owned());
    }
    if let Ok(current) = std::env::current_dir() {
        roots.push(current.join("resources"));
        roots.push(current.join("assets"));
    }
    roots
}

/// Build an image source directly from embedded bytes. This keeps title-bar,
/// avatar, and status images working in a release build without requiring the
/// source checkout or a separately copied resource directory.
pub fn image_source(asset: Asset) -> gpui::ImageSource {
    Arc::new(gpui::Image::from_bytes(
        gpui::ImageFormat::Png,
        asset.embedded().to_vec(),
    ))
    .into()
}

/// Resolve a filesystem path for GPUI APIs that cannot consume embedded bytes.
/// The returned fallback is relative to the configured application resource
/// directory and never contains a checkout-specific absolute path.
pub fn path(asset: Asset) -> PathBuf {
    AssetResolver::new()
        .resolve(asset)
        .unwrap_or_else(|| asset.relative_path())
}

pub fn logo() -> Asset {
    Asset::Logo
}

pub fn title_banner() -> Asset {
    Asset::TitleBanner
}

pub fn sinner(asset: SinnerAsset) -> Asset {
    Asset::Sinner(asset)
}

pub fn status_effect(asset: StatusEffectAsset) -> Asset {
    Asset::StatusEffect(asset)
}

pub fn help(language: Language) -> Asset {
    Asset::Help(language)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_canonical_assets_are_embedded_and_have_relative_paths() {
        assert!(!Asset::Logo.embedded().is_empty());
        assert!(!Asset::TitleBanner.embedded().is_empty());
        for sinner in SinnerAsset::ALL {
            let asset = Asset::Sinner(sinner);
            assert!(!asset.embedded().is_empty());
            assert!(!asset.relative_path().is_absolute());
        }
        for effect in StatusEffectAsset::ALL {
            let asset = Asset::StatusEffect(effect);
            assert!(!asset.embedded().is_empty());
            assert!(!asset.relative_path().is_absolute());
        }
    }

    #[test]
    fn localized_help_uses_the_canonical_language_values() {
        let zh = Asset::help_for(Language::ZhCn);
        let en = Asset::help_for(Language::EnUs);
        assert!(String::from_utf8_lossy(zh.embedded()).contains("#"));
        assert!(String::from_utf8_lossy(en.embedded()).contains("#"));
        assert_ne!(zh.relative_path(), en.relative_path());
    }

    #[test]
    fn resolver_can_use_a_release_style_root_without_absolute_constants() {
        let root = std::env::temp_dir().join(format!("ahab-assets-{}", std::process::id()));
        let nested = root.join("sinners");
        std::fs::create_dir_all(&nested).unwrap();
        std::fs::write(nested.join("faust.png"), b"test").unwrap();
        let resolver = AssetResolver::from_root(root.clone());
        assert_eq!(
            resolver.load(Asset::Sinner(SinnerAsset::Faust)).unwrap(),
            Asset::Sinner(SinnerAsset::Faust).embedded().to_vec()
        );
        let _ = std::fs::remove_dir_all(root);
    }
}
