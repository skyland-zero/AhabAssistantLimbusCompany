use super::*;

/// The five sections of the mirror-team editor. Keeping this as a model enum
/// makes tab selection testable without constructing a GPUI window.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum TeamEditorTab {
    #[default]
    Basic,
    Shop,
    Combat,
    Starlight,
    Advanced,
}

impl TeamEditorTab {
    pub const ALL: [Self; 5] = [
        Self::Basic,
        Self::Shop,
        Self::Combat,
        Self::Starlight,
        Self::Advanced,
    ];

    pub const fn is_available(self, purpose: TeamPurpose) -> bool {
        !matches!(purpose, TeamPurpose::Luxcavation) || matches!(self, Self::Basic)
    }

    #[allow(dead_code)]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Basic => "基础编成",
            Self::Shop => "商店与合成",
            Self::Combat => "二体系与战斗",
            Self::Starlight => "开局星光",
            Self::Advanced => "观测与高级",
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum TeamFilter {
    #[default]
    All,
    Mirror,
    Luxcavation,
    General,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TeamSelect {
    Purpose,
    FixedTeamUse,
    ShopStrategy,
    AfterLevelIv,
    SecondSystem,
    SecondSystemFloor,
    DefenseTurns,
    SkillReplacementMode,
}

impl TeamFilter {
    pub const ALL: [Self; 4] = [Self::All, Self::Mirror, Self::Luxcavation, Self::General];

    #[allow(dead_code)]
    pub const fn label(self) -> &'static str {
        match self {
            Self::All => "全部",
            Self::Mirror => "镜牢",
            Self::Luxcavation => "经验本",
            Self::General => "通用",
        }
    }

    pub const fn purpose(self) -> Option<TeamPurpose> {
        match self {
            Self::All => None,
            Self::Mirror => Some(TeamPurpose::Mirror),
            Self::Luxcavation => Some(TeamPurpose::Luxcavation),
            Self::General => Some(TeamPurpose::General),
        }
    }
}

pub struct TeamEditorState {
    pub team: TeamDetail,
    pub tab: TeamEditorTab,
    pub json_import_open: bool,
    /// A number selected from an empty UI slot. Existing teams keep their
    /// number in the backend-derived `team-N` id and do not use this field.
    pub requested_team_number: Option<u32>,
}

impl TeamEditorState {
    pub fn new(team: TeamDetail) -> Self {
        Self::new_with_slot(team, None)
    }

    pub fn new_with_slot(team: TeamDetail, requested_team_number: Option<u32>) -> Self {
        Self {
            team,
            tab: TeamEditorTab::Basic,
            json_import_open: false,
            requested_team_number,
        }
    }

    pub fn mirror_config(&self) -> TeamMirrorConfig {
        self.team.mirrorConfig.clone().unwrap_or_default()
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct TeamSlot {
    pub number: u32,
    pub team: Option<TeamDetail>,
}

impl TeamSlot {
    pub fn default_purpose(number: u32) -> TeamPurpose {
        if number == 1 {
            TeamPurpose::Luxcavation
        } else {
            TeamPurpose::Mirror
        }
    }
}

#[derive(Clone, Debug)]
pub enum TeamPresetTarget {
    EmptySlot(u32),
    Existing(TeamDetail),
}

#[derive(Clone, Debug)]
pub struct TeamPresetPickerState {
    pub target: TeamPresetTarget,
}

#[derive(Clone, Debug)]
pub struct TeamPresetOverwriteState {
    pub target: TeamDetail,
    pub preset: TeamPreset,
}

/// State and RPC boundary for the Teams page. The page only mutates this
/// object and calls `cx.notify`; all contract serialization remains here.
pub struct TeamsState {
    pub rpc: RpcGateway,
    pub teams: Vec<TeamDetail>,
    pub sinners: Vec<SinnerInfo>,
    pub presets: Vec<TeamPreset>,
    pub filter: TeamFilter,
    pub editor: Option<TeamEditorState>,
    pub delete_target: Option<TeamDetail>,
    pub preset_picker: Option<TeamPresetPickerState>,
    pub preset_overwrite: Option<TeamPresetOverwriteState>,
    pub open_select: Option<TeamSelect>,
    pub feedback: Option<String>,
    pub saving: bool,
    pub deleting: bool,
    pub team_toggle_in_flight: Option<String>,
}

/// Named bool fields keep the page readable and prevent ad-hoc JSON patches.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MirrorBool {
    DoNotHeal,
    DoNotBuy,
    DoNotFuse,
    DoNotSell,
    DoNotEnhance,
    OnlyAggressiveFuse,
    DoNotSystemFuse,
    OnlySystemFuse,
    AggressiveAlsoEnhance,
    AggressiveSaveSystems,
    AfterLevelIv,
    SecondSystem,
    SecondSystemFuseIv,
    SecondSystemBuy,
    SecondSystemSelectReward,
    SecondSystemPowerUp,
    AvoidSkill3,
    PrioritizeSkill3,
    ReformationEachFloor,
    DefenseFirstRound,
    DefenseForSolo,
    SkillReplacement,
    UseStarlight,
    FixedTeamUse,
    UseTeamCode,
    UseCustomThemeWeight,
    ObserveEgoGift,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MirrorU8 {
    ShopStrategy,
    AfterLevelIvSelect,
    MaxKeywordRefresh,
    MaxNormalRefresh,
    SecondSystemSelect,
    SecondSystemStartFloor,
    DefenseTurns,
    SkillReplacementMode,
    FixedTeamUseSelect,
}

pub const SYSTEM_NAMES: [&str; 10] = [
    "burn", "bleed", "tremor", "rupture", "sinking", "poise", "charge", "slash", "pierce", "blunt",
];

pub const SYSTEM_LABELS: [&str; 10] = [
    "燃烧", "流血", "震颤", "破裂", "沉潜", "呼吸", "充能", "斩击", "突刺", "打击",
];
