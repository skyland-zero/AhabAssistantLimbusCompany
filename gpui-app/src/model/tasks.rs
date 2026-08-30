#![allow(non_snake_case)]

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum FixedTaskId {
    SetWindows,
    DailyTask,
    GetReward,
    BuyEnkephalin,
    Mirror,
    #[serde(rename = "resonate_with_Ahab")]
    ResonateWithAhab,
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ExecutionState {
    #[default]
    Idle,
    Running,
    Paused,
    Stopping,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[allow(clippy::enum_variant_names)]
#[serde(rename_all = "snake_case")]
pub enum AfterExitAction {
    ExitGame,
    ExitEmulator,
    ExitAalc,
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AfterPowerAction {
    #[default]
    None,
    Sleep,
    Hibernate,
    Lock,
    Shutdown,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct AfterCompletionConfig {
    pub actions: Vec<AfterExitAction>,
    pub powerAction: AfterPowerAction,
    pub keepAfterCompletion: bool,
}

impl Default for AfterCompletionConfig {
    fn default() -> Self {
        Self {
            actions: Vec::new(),
            powerAction: AfterPowerAction::None,
            keepAfterCompletion: false,
        }
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[allow(non_snake_case)]
pub struct SetWindowsConfig {
    pub set_win_size: u16,
    pub set_win_position: String,
    pub set_reduce_miscontact: bool,
    pub screenshot_interval: f32,
    pub mouse_action_interval: f32,
    pub mouse_down_duration: f32,
    pub use_post_message: bool,
}

impl Default for SetWindowsConfig {
    fn default() -> Self {
        Self {
            set_win_size: 1080,
            set_win_position: "free".into(),
            set_reduce_miscontact: true,
            screenshot_interval: 0.5,
            mouse_action_interval: 0.3,
            mouse_down_duration: 0.1,
            use_post_message: false,
        }
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[allow(non_snake_case)]
pub struct DailyTaskConfig {
    pub set_EXP_count: u8,
    pub set_thread_count: u8,
    pub daily_teams: u8,
    pub use_continuous_combat: bool,
    pub use_continuous_combat_select: u8,
    pub targeted_teaming_EXP: bool,
    pub EXP_day_1_2: u8,
    pub EXP_day_3_4: u8,
    pub EXP_day_5_6: u8,
    pub EXP_day_7: u8,
    pub targeted_teaming_thread: bool,
    pub thread_day_1: u8,
    pub thread_day_2: u8,
    pub thread_day_3: u8,
    pub thread_day_4: u8,
    pub thread_day_5: u8,
    pub thread_day_6: u8,
    pub thread_day_7: u8,
}

impl Default for DailyTaskConfig {
    fn default() -> Self {
        Self {
            set_EXP_count: 1,
            set_thread_count: 3,
            daily_teams: 1,
            use_continuous_combat: false,
            use_continuous_combat_select: 1,
            targeted_teaming_EXP: false,
            EXP_day_1_2: 1,
            EXP_day_3_4: 1,
            EXP_day_5_6: 1,
            EXP_day_7: 1,
            targeted_teaming_thread: false,
            thread_day_1: 1,
            thread_day_2: 1,
            thread_day_3: 1,
            thread_day_4: 1,
            thread_day_5: 1,
            thread_day_6: 1,
            thread_day_7: 1,
        }
    }
}
impl DailyTaskConfig {
    pub fn mock_default() -> Self {
        Self::default()
    }
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
pub struct GetRewardConfig {
    pub set_get_prize: u8,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[allow(non_snake_case)]
pub struct BuyEnkephalinConfig {
    pub set_lunacy_to_enkephalin: u8,
    pub Dr_Grandet_mode: bool,
    pub skip_enkephalin: bool,
}
impl Default for BuyEnkephalinConfig {
    fn default() -> Self {
        Self {
            set_lunacy_to_enkephalin: 2,
            Dr_Grandet_mode: false,
            skip_enkephalin: false,
        }
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[allow(non_snake_case)]
pub struct MirrorConfig {
    pub set_mirror_count: u8,
    pub infinite_dungeons: bool,
    pub hard_mirror: bool,
    pub no_weekly_bonuses: bool,
    pub floor_3_exit: bool,
    pub save_rewards: bool,
    pub hard_mirror_single_bonuses: bool,
    pub select_event_pack: bool,
    pub skip_event_pack: bool,
    pub re_claim_rewards: bool,
    pub not_skip_whitegossypium: bool,
    pub fight_to_last_man: bool,
    pub mirror_keyboard_navigation: bool,
    pub mirror_keyboard_simple_pathfinding: bool,
}

impl Default for MirrorConfig {
    fn default() -> Self {
        Self {
            set_mirror_count: 1,
            infinite_dungeons: false,
            hard_mirror: false,
            no_weekly_bonuses: false,
            floor_3_exit: false,
            save_rewards: false,
            hard_mirror_single_bonuses: false,
            select_event_pack: false,
            skip_event_pack: false,
            re_claim_rewards: false,
            not_skip_whitegossypium: false,
            fight_to_last_man: false,
            mirror_keyboard_navigation: false,
            mirror_keyboard_simple_pathfinding: false,
        }
    }
}
impl MirrorConfig {
    pub fn mock_default() -> Self {
        Self::default()
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[allow(non_snake_case)]
pub struct EnabledTasks {
    pub daily_task: bool,
    pub get_reward: bool,
    pub buy_enkephalin: bool,
    pub mirror: bool,
    pub resonate_with_Ahab: bool,
}
impl Default for EnabledTasks {
    fn default() -> Self {
        Self {
            daily_task: true,
            get_reward: true,
            buy_enkephalin: false,
            mirror: true,
            resonate_with_Ahab: true,
        }
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[allow(non_snake_case)]
pub struct TasksConfig {
    #[serde(default = "schema_version")]
    pub schemaVersion: u32,
    pub enabledTasks: EnabledTasks,
    pub set_windows: SetWindowsConfig,
    pub daily_task: DailyTaskConfig,
    pub get_reward: GetRewardConfig,
    pub buy_enkephalin: BuyEnkephalinConfig,
    pub mirror: MirrorConfig,
    pub resonate_with_Ahab: ResonateWithAhabConfig,
    pub afterCompletion: AfterCompletionConfig,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct ResonateWithAhabConfig {
    pub enabled: bool,
}
impl Default for ResonateWithAhabConfig {
    fn default() -> Self {
        Self { enabled: true }
    }
}
impl Default for TasksConfig {
    fn default() -> Self {
        Self {
            schemaVersion: schema_version(),
            enabledTasks: Default::default(),
            set_windows: Default::default(),
            daily_task: DailyTaskConfig::mock_default(),
            get_reward: Default::default(),
            buy_enkephalin: Default::default(),
            mirror: MirrorConfig::mock_default(),
            resonate_with_Ahab: Default::default(),
            afterCompletion: Default::default(),
        }
    }
}

fn schema_version() -> u32 {
    1
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
pub struct ExecutionStatusPayload {
    pub state: ExecutionState,
    pub currentTaskId: Option<FixedTaskId>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct MirrorProgressPayload {
    pub current: u32,
    pub total: u32,
    pub isHard: bool,
    pub isInfinite: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn defaults_match_mock_contract() {
        let cfg = TasksConfig::default();
        assert_eq!(cfg.set_windows.set_win_size, 1080);
        assert_eq!(cfg.set_windows.set_win_position, "free");
        assert_eq!(cfg.daily_task.set_EXP_count, 1);
        assert_eq!(cfg.daily_task.set_thread_count, 3);
        assert!(!cfg.daily_task.use_continuous_combat);
        assert_eq!(cfg.mirror.set_mirror_count, 1);
        assert!(cfg.enabledTasks.resonate_with_Ahab);
        assert!(cfg.afterCompletion.actions.is_empty());
        assert!(!cfg.afterCompletion.keepAfterCompletion);
    }
    #[test]
    fn task_config_round_trips_json() {
        let cfg = TasksConfig::default();
        let json = serde_json::to_string(&cfg).unwrap();
        assert_eq!(serde_json::from_str::<TasksConfig>(&json).unwrap(), cfg);
    }
}
