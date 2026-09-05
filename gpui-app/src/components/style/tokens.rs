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
/// `Default` is the current flat modern look. `Limbus` keeps the same
/// layout and components but swaps surfaces for the cold-black / dark-red /
/// mustard-gold边狱 language and squares off control corners.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum SkinId {
    #[default]
    Default,
    Limbus,
}

impl SkinId {
    pub const ALL: [Self; 2] = [Self::Default, Self::Limbus];

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Default => "default",
            Self::Limbus => "limbus",
        }
    }

    pub const fn name_zh(self) -> &'static str {
        match self {
            Self::Default => "现代",
            Self::Limbus => "边狱",
        }
    }

    pub const fn name_en(self) -> &'static str {
        match self {
            Self::Default => "Modern",
            Self::Limbus => "Limbus",
        }
    }

    pub const fn is_limbus(self) -> bool {
        matches!(self, Self::Limbus)
    }

    /// Unknown values fall back to the modern skin so old settings files
    /// keep rendering exactly as before.
    pub fn parse(value: &str) -> Self {
        match value {
            "limbus" => Self::Limbus,
            "default" => Self::Default,
            _ => Self::Default,
        }
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
}

impl AccentTokens {
    pub const fn for_scheme(accent: AccentId, scheme: ColorScheme) -> Self {
        match (accent, scheme) {
            (AccentId::Crimson, ColorScheme::Light) => Self::new(0xc8354f, 0xa92b42, 0xfbe9ec),
            (AccentId::Crimson, ColorScheme::Dark) => Self::new(0xe05a72, 0xc8354f, 0x3a1e24),
            (AccentId::Blue, ColorScheme::Light) => Self::new(0x2563eb, 0x1d4ed8, 0xdbeafe),
            (AccentId::Blue, ColorScheme::Dark) => Self::new(0x60a5fa, 0x3b82f6, 0x1e293b),
            (AccentId::Amber, ColorScheme::Light) => Self::new(0xd97706, 0xb45309, 0xfef3c7),
            (AccentId::Amber, ColorScheme::Dark) => Self::new(0xfbbf24, 0xf59e0b, 0x3b2f14),
            (AccentId::Emerald, ColorScheme::Light) => Self::new(0x059669, 0x047857, 0xd1fae5),
            (AccentId::Emerald, ColorScheme::Dark) => Self::new(0x34d399, 0x10b981, 0x14332a),
            (AccentId::Violet, ColorScheme::Light) => Self::new(0x7c3aed, 0x6d28d9, 0xede9fe),
            (AccentId::Violet, ColorScheme::Dark) => Self::new(0xa78bfa, 0x8b5cf6, 0x2b2247),
            (AccentId::LimbusBrass, ColorScheme::Light) => {
                Self::new_with_foreground(0x7a5517, 0x5e3f10, 0xf4e5c2, 0xfafafa)
            }
            (AccentId::LimbusBrass, ColorScheme::Dark) => {
                Self::new_with_foreground(0xd1aa52, 0xad8434, 0x4a381d, 0x17120a)
            }
        }
    }

    const fn new(brand: u32, brand_hover: u32, brand_light: u32) -> Self {
        Self::new_with_foreground(brand, brand_hover, brand_light, 0xfafafa)
    }

    const fn new_with_foreground(
        brand: u32,
        brand_hover: u32,
        brand_light: u32,
        brand_foreground: u32,
    ) -> Self {
        Self {
            brand: ColorToken::rgb(brand),
            brand_hover: ColorToken::rgb(brand_hover),
            brand_light: ColorToken::rgb(brand_light),
            brand_foreground: ColorToken::rgb(brand_foreground),
        }
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
