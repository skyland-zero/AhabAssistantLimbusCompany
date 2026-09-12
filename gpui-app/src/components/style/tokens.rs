/// A color token encoded as RGBA (`0xRRGGBBAA`), which maps directly to
/// [`gpui::rgba`]. Alpha is retained for the transparent CSS border and
/// selection colors even though the compatibility constants below are RGB.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ColorToken(pub u32);

impl ColorToken {
    pub const fn rgb(hex: u32) -> Self {
        Self((hex & 0x00ff_ffff) << 8 | 0xff)
    }

    pub const fn rgba(hex: u32) -> Self {
        Self(hex)
    }

    pub const fn rgba_hex(self) -> u32 {
        self.0
    }

    pub const fn rgb_hex(self) -> u32 {
        (self.0 >> 8) & 0x00ff_ffff
    }

    pub const fn alpha(self) -> u8 {
        self.0 as u8
    }

    pub const fn is_transparent(self) -> bool {
        self.alpha() == 0
    }
}

/// Light and dark token sets from `index.css`.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum ColorScheme {
    #[default]
    Light,
    Dark,
}

/// The visual skin applied on top of scheme + accent.
///
/// All five skins share the same component tree and layout; only surfaces,
/// geometry, decoration and motion language change. That keeps a skin switch
/// from moving content or invalidating a screenshot baseline's structure.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum SkinId {
    /// The flat, rounded shadcn/Radix look the web client established.
    #[default]
    Default,
    /// Layered translucent surfaces, hairline highlights, large radii and
    /// real elevation. Reads as frosted glass without depending on any
    /// platform blur support.
    Glass,
    /// Paper and ink: warm near-white ground, visible hairlines, no shadows,
    /// whitespace instead of elevation.
    Archive,
    /// Cold black (or parchment) with dark red surfaces, brass frame language,
    /// square corners and stencilled card headers.
    Limbus,
    /// Near-black with a red vignette and film grain; the accent only appears
    /// on active and focused states.
    Mist,
}

impl SkinId {
    pub const ALL: [Self; 5] = [
        Self::Default,
        Self::Glass,
        Self::Archive,
        Self::Limbus,
        Self::Mist,
    ];

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Default => "default",
            Self::Glass => "glass",
            Self::Archive => "archive",
            Self::Limbus => "limbus",
            Self::Mist => "mist",
        }
    }

    pub const fn name_zh(self) -> &'static str {
        match self {
            Self::Default => "现代",
            Self::Glass => "玻璃",
            Self::Archive => "档案",
            Self::Limbus => "边狱",
            Self::Mist => "血雾",
        }
    }

    pub const fn name_en(self) -> &'static str {
        match self {
            Self::Default => "Modern",
            Self::Glass => "Glass",
            Self::Archive => "Archive",
            Self::Limbus => "Limbus",
            Self::Mist => "Mist",
        }
    }

    /// One-line description used by the appearance page's preview grid.
    pub const fn blurb_zh(self) -> &'static str {
        match self {
            Self::Default => "扁平平铺 · 圆角 · 轻描边",
            Self::Glass => "分层半透 · 大圆角 · 真投影",
            Self::Archive => "纸墨发丝线 · 靠留白分层",
            Self::Limbus => "冷黑暗红 · 黄铜角框 · 直角",
            Self::Mist => "近黑血雾 · 暗角颗粒 · 锐角",
        }
    }

    pub const fn blurb_en(self) -> &'static str {
        match self {
            Self::Default => "Flat, rounded, light outline",
            Self::Glass => "Layered translucency, elevation",
            Self::Archive => "Paper hairlines, whitespace",
            Self::Limbus => "Cold black, dark red, brass frame",
            Self::Mist => "Near-black haze, red vignette",
        }
    }

    pub const fn decor(self) -> Decor {
        match self {
            Self::Default => Decor::Plain,
            Self::Glass => Decor::Glass,
            Self::Archive => Decor::Archive,
            Self::Limbus => Decor::LimbusFrame,
            Self::Mist => Decor::Mist,
        }
    }

    pub const fn shape(self) -> Shape {
        Shape::for_skin(self)
    }

    pub const fn is_limbus(self) -> bool {
        matches!(self, Self::Limbus)
    }

    pub const fn is_glass(self) -> bool {
        matches!(self, Self::Glass)
    }

    pub const fn is_archive(self) -> bool {
        matches!(self, Self::Archive)
    }

    pub const fn is_mist(self) -> bool {
        matches!(self, Self::Mist)
    }

    /// Unknown values fall back to the modern skin so old settings files
    /// keep rendering exactly as before.
    pub fn parse(value: &str) -> Self {
        match value {
            "default" | "modern" => Self::Default,
            "glass" | "aero" => Self::Glass,
            "archive" | "paper" => Self::Archive,
            "limbus" => Self::Limbus,
            "mist" | "blood" => Self::Mist,
            _ => Self::Default,
        }
    }
}

/// The decoration language a skin paints on top of the flat surfaces.
///
/// Renderers branch on this instead of on `SkinId`, so a skin can change its
/// colour scheme without every call site learning a new predicate.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum Decor {
    /// No decoration; surfaces are plain fills.
    #[default]
    Plain,
    /// Top highlight hairline plus an accent-tinted glow.
    Glass,
    /// Hairline rules and section dividers.
    Archive,
    /// Gold corner brackets, stencilled headers and a wax seal.
    LimbusFrame,
    /// Full-window red vignette with a low-opacity grain layer.
    Mist,
}

/// How much elevation a skin gives to card-level surfaces.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum ShadowLevel {
    /// No shadow at all; separation comes from borders and whitespace.
    #[default]
    None,
    /// A restrained two-layer shadow for flat themes.
    Subtle,
    /// A clearly visible card shadow.
    Raised,
    /// A large soft shadow for translucent, floating surfaces.
    Floating,
}

/// Per-skin geometry. Pages ask for a semantic size (`sm`/`md`/`lg`/`xl`)
/// and the skin decides the actual radius, so a skin switch can square off or
/// round every control without editing page code.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Shape {
    pub radius_sm: u32,
    pub radius_md: u32,
    pub radius_lg: u32,
    pub radius_xl: u32,
    /// Switches stay pill shaped in every skin: the shape is load-bearing for
    /// the on/off affordance, so squaring it off would hurt usability.
    pub pill: bool,
    pub border_width: u32,
    pub shadow: ShadowLevel,
}

impl Shape {
    pub const fn for_skin(skin: SkinId) -> Self {
        match skin {
            SkinId::Default => Self::new(4, 6, 8, 12, ShadowLevel::Subtle),
            SkinId::Glass => Self::new(6, 10, 14, 18, ShadowLevel::Floating),
            SkinId::Archive => Self::new(2, 4, 6, 8, ShadowLevel::None),
            SkinId::Limbus => Self::new(0, 0, 0, 0, ShadowLevel::None),
            SkinId::Mist => Self::new(0, 2, 4, 6, ShadowLevel::Raised),
        }
    }

    const fn new(sm: u32, md: u32, lg: u32, xl: u32, shadow: ShadowLevel) -> Self {
        Self {
            radius_sm: sm,
            radius_md: md,
            radius_lg: lg,
            radius_xl: xl,
            pill: true,
            border_width: 1,
            shadow,
        }
    }

    /// Radius for a control-level surface (`large == false` is a button or a
    /// tab, `large == true` is a card).
    pub const fn radius(self, large: bool) -> u32 {
        if large {
            self.radius_lg
        } else {
            self.radius_md
        }
    }
}

impl Default for Shape {
    fn default() -> Self {
        Self::for_skin(SkinId::Default)
    }
}

/// The six accent identifiers accepted by the persisted UI settings.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum AccentId {
    #[default]
    Crimson,
    Blue,
    Amber,
    Emerald,
    Violet,
    LimbusBrass,
}

impl AccentId {
    pub const ALL: [Self; 6] = [
        Self::Crimson,
        Self::Blue,
        Self::Amber,
        Self::Emerald,
        Self::Violet,
        Self::LimbusBrass,
    ];

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Crimson => "crimson",
            Self::Blue => "blue",
            Self::Amber => "amber",
            Self::Emerald => "emerald",
            Self::Violet => "violet",
            Self::LimbusBrass => "limbus-brass",
        }
    }

    pub const fn name_zh(self) -> &'static str {
        match self {
            Self::Crimson => "赤红",
            Self::Blue => "深蓝",
            Self::Amber => "琥珀",
            Self::Emerald => "翠绿",
            Self::Violet => "紫罗兰",
            Self::LimbusBrass => "边狱黄铜",
        }
    }

    pub const fn name_en(self) -> &'static str {
        match self {
            Self::Crimson => "Crimson",
            Self::Blue => "Blue",
            Self::Amber => "Amber",
            Self::Emerald => "Emerald",
            Self::Violet => "Violet",
            Self::LimbusBrass => "Limbus Brass",
        }
    }

    /// Unknown values intentionally fall back to crimson, matching the web
    /// theme's `ACCENT_PRESETS[0]` fallback and keeping old settings readable.
    pub fn parse(value: &str) -> Self {
        match value {
            "blue" => Self::Blue,
            "amber" => Self::Amber,
            "emerald" => Self::Emerald,
            "violet" => Self::Violet,
            "limbus-brass" => Self::LimbusBrass,
            "crimson" => Self::Crimson,
            _ => Self::Crimson,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AccentTokens {
    pub brand: ColorToken,
    pub brand_hover: ColorToken,
    pub brand_light: ColorToken,
    pub brand_foreground: ColorToken,
    /// Frame / rule tone derived from the accent.
    ///
    /// The `Default` skin ignores it; the framed skins (limbus, mist, archive)
    /// use it for corner brackets, card-header rules and focus rings so an
    /// arbitrary accent can never clash with a hardcoded brass frame.
    pub decor: ColorToken,
}

impl AccentTokens {
    /// Derive the accent ramp for one scheme.
    ///
    /// `brand_foreground` is picked per accent/scheme to reach at least 4.5:1
    /// against `brand` (WCAG AA for normal-size label text), the same way the
    /// `primary` / `primary_foreground` pair already works. Light schemes use
    /// pale ink where the brand is dark enough; the bright accents of a dark
    /// scheme take dark ink instead of white, because white on a bright accent
    /// only reached 1.6:1 - 3.4:1.
    pub const fn for_scheme(accent: AccentId, scheme: ColorScheme) -> Self {
        match (accent, scheme) {
            (AccentId::Crimson, ColorScheme::Light) => {
                Self::new(0xc8354f, 0xa92b42, 0xfbe9ec, 0xfafafa, 0xa92b42)
            }
            (AccentId::Crimson, ColorScheme::Dark) => {
                Self::new(0xe05a72, 0xc8354f, 0x3a1e24, 0x2a0c12, 0xe05a72)
            }
            (AccentId::Blue, ColorScheme::Light) => {
                Self::new(0x2563eb, 0x1d4ed8, 0xdbeafe, 0xfafafa, 0x1d4ed8)
            }
            (AccentId::Blue, ColorScheme::Dark) => {
                Self::new(0x60a5fa, 0x3b82f6, 0x1e293b, 0x0a1a30, 0x60a5fa)
            }
            (AccentId::Amber, ColorScheme::Light) => {
                // `decor` is darkened from the brand so it stays readable as a
                // rule / bracket tone on the light skins' warm grounds.
                Self::new(0xd97706, 0xb45309, 0xfef3c7, 0x241705, 0x9a4708)
            }
            (AccentId::Amber, ColorScheme::Dark) => {
                Self::new(0xfbbf24, 0xf59e0b, 0x3b2f14, 0x2a1c04, 0xfbbf24)
            }
            (AccentId::Emerald, ColorScheme::Light) => {
                Self::new(0x059669, 0x047857, 0xd1fae5, 0x041a10, 0x047857)
            }
            (AccentId::Emerald, ColorScheme::Dark) => {
                Self::new(0x34d399, 0x10b981, 0x14332a, 0x04170f, 0x34d399)
            }
            (AccentId::Violet, ColorScheme::Light) => {
                Self::new(0x7c3aed, 0x6d28d9, 0xede9fe, 0xfafafa, 0x6d28d9)
            }
            (AccentId::Violet, ColorScheme::Dark) => {
                Self::new(0xa78bfa, 0x8b5cf6, 0x2b2247, 0x150a2e, 0xa78bfa)
            }
            (AccentId::LimbusBrass, ColorScheme::Light) => {
                Self::new(0x7a5517, 0x5e3f10, 0xf4e5c2, 0xfafafa, 0x8a6413)
            }
            (AccentId::LimbusBrass, ColorScheme::Dark) => {
                // `decor` keeps the exact pre-existing mustard gold so the
                // limbus skin's brass frame is byte-for-byte unchanged.
                Self::new(0xd1aa52, 0xad8434, 0x4a381d, 0x17120a, 0xd8a800)
            }
        }
    }

    const fn new(
        brand: u32,
        brand_hover: u32,
        brand_light: u32,
        brand_foreground: u32,
        decor: u32,
    ) -> Self {
        Self {
            brand: ColorToken::rgb(brand),
            brand_hover: ColorToken::rgb(brand_hover),
            brand_light: ColorToken::rgb(brand_light),
            brand_foreground: ColorToken::rgb(brand_foreground),
            decor: ColorToken::rgb(decor),
        }
    }

    /// Whether this accent is the skin-specific signature colour, used by the
    /// settings page to nudge users towards the intended pairing.
    pub const fn is_signature(accent: AccentId) -> bool {
        matches!(accent, AccentId::LimbusBrass)
    }
}

/// Metadata for the six supported accent presets.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AccentPreset {
    pub id: &'static str,
    pub name: &'static str,
    pub name_en: &'static str,
    pub light: AccentTokens,
    pub dark: AccentTokens,
}

/// Accent metadata retained as a static table for settings controls.
pub const ACCENT_PRESETS: [AccentPreset; 6] = [
    AccentPreset {
        id: "crimson",
        name: "赤红",
        name_en: "Crimson",
        light: AccentTokens::for_scheme(AccentId::Crimson, ColorScheme::Light),
        dark: AccentTokens::for_scheme(AccentId::Crimson, ColorScheme::Dark),
    },
    AccentPreset {
        id: "blue",
        name: "深蓝",
        name_en: "Blue",
        light: AccentTokens::for_scheme(AccentId::Blue, ColorScheme::Light),
        dark: AccentTokens::for_scheme(AccentId::Blue, ColorScheme::Dark),
    },
    AccentPreset {
        id: "amber",
        name: "琥珀",
        name_en: "Amber",
        light: AccentTokens::for_scheme(AccentId::Amber, ColorScheme::Light),
        dark: AccentTokens::for_scheme(AccentId::Amber, ColorScheme::Dark),
    },
    AccentPreset {
        id: "emerald",
        name: "翠绿",
        name_en: "Emerald",
        light: AccentTokens::for_scheme(AccentId::Emerald, ColorScheme::Light),
        dark: AccentTokens::for_scheme(AccentId::Emerald, ColorScheme::Dark),
    },
    AccentPreset {
        id: "violet",
        name: "紫罗兰",
        name_en: "Violet",
        light: AccentTokens::for_scheme(AccentId::Violet, ColorScheme::Light),
        dark: AccentTokens::for_scheme(AccentId::Violet, ColorScheme::Dark),
    },
    AccentPreset {
        id: "limbus-brass",
        name: "边狱黄铜",
        name_en: "Limbus Brass",
        light: AccentTokens::for_scheme(AccentId::LimbusBrass, ColorScheme::Light),
        dark: AccentTokens::for_scheme(AccentId::LimbusBrass, ColorScheme::Dark),
    },
];
