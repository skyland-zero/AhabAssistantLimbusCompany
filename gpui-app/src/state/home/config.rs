use super::*;

impl HomeState {
    /// Apply one complete task-config mutation and send the canonical config
    /// through IPC. This is the seam used by all Home controls, so no control
    /// can accidentally update the UI without updating the backend.
    pub fn update_tasks(&mut self, update: impl FnOnce(&mut TasksConfig)) {
        if self.is_busy() {
            return;
        }
        update(&mut self.tasks);
        self.save_tasks();
    }

    pub fn set_after_completion_open(&mut self, open: bool) {
        self.after_completion_open = open;
        self.after_completion_draft = open.then(|| self.tasks.afterCompletion.clone());
        if open {
            self.close_select();
        }
    }

    pub fn toggle_after_completion_draft(&mut self, action: AfterExitAction) {
        if self.is_busy() {
            return;
        }
        if let Some(config) = self.after_completion_draft.as_mut() {
            if let Some(index) = config.actions.iter().position(|item| *item == action) {
                config.actions.remove(index);
            } else {
                config.actions.push(action);
            }
        }
    }

    pub fn set_after_completion_draft_power(&mut self, action: AfterPowerAction) {
        if self.is_busy() {
            return;
        }
        if let Some(config) = self.after_completion_draft.as_mut() {
            config.powerAction = action;
        }
    }

    pub fn apply_after_completion(&mut self, keep: bool) {
        if self.is_busy() {
            return;
        }
        let Some(mut draft) = self.after_completion_draft.take() else {
            return;
        };
        draft.keepAfterCompletion = keep;
        self.update_tasks(|tasks| tasks.afterCompletion = draft);
        self.after_completion_open = false;
    }

    pub fn toggle_mirror_option(&mut self, field: MirrorOption) {
        self.update_tasks(|tasks| match field {
            MirrorOption::Infinite => {
                tasks.mirror.infinite_dungeons = !tasks.mirror.infinite_dungeons
            }
            MirrorOption::NoWeeklyBonuses => {
                tasks.mirror.no_weekly_bonuses = !tasks.mirror.no_weekly_bonuses
            }
            MirrorOption::FloorThreeExit => tasks.mirror.floor_3_exit = !tasks.mirror.floor_3_exit,
            MirrorOption::SaveRewards => tasks.mirror.save_rewards = !tasks.mirror.save_rewards,
            MirrorOption::HardSingleBonuses => {
                tasks.mirror.hard_mirror_single_bonuses = !tasks.mirror.hard_mirror_single_bonuses
            }
            MirrorOption::SelectEventPack => {
                tasks.mirror.select_event_pack = !tasks.mirror.select_event_pack
            }
            MirrorOption::SkipEventPack => {
                tasks.mirror.skip_event_pack = !tasks.mirror.skip_event_pack
            }
            MirrorOption::ReclaimRewards => {
                tasks.mirror.re_claim_rewards = !tasks.mirror.re_claim_rewards
            }
            MirrorOption::NotSkipCotton => {
                tasks.mirror.not_skip_whitegossypium = !tasks.mirror.not_skip_whitegossypium
            }
            MirrorOption::FightToLast => {
                tasks.mirror.fight_to_last_man = !tasks.mirror.fight_to_last_man
            }
            MirrorOption::KeyboardNavigation => {
                tasks.mirror.mirror_keyboard_navigation = !tasks.mirror.mirror_keyboard_navigation
            }
            MirrorOption::SimplePathfinding => {
                tasks.mirror.mirror_keyboard_simple_pathfinding =
                    !tasks.mirror.mirror_keyboard_simple_pathfinding
            }
        });
    }

    pub fn set_hard_mirror_target_floors(&mut self, floors: u32) {
        if self.is_busy() {
            return;
        }
        self.update_tasks(|tasks| tasks.mirror.hard_mirror_target_floors = floors);
    }

    #[allow(dead_code)]
    pub fn toggle_after_exit_action(&mut self, action: AfterExitAction) {
        self.update_tasks(|tasks| {
            if let Some(index) = tasks
                .afterCompletion
                .actions
                .iter()
                .position(|item| *item == action)
            {
                tasks.afterCompletion.actions.remove(index);
            } else {
                tasks.afterCompletion.actions.push(action);
            }
        });
    }

    #[allow(dead_code)]
    pub fn set_after_power_action(&mut self, action: AfterPowerAction) {
        self.update_tasks(|tasks| tasks.afterCompletion.powerAction = action);
    }

    #[allow(dead_code)]
    pub fn set_keep_after_completion(&mut self, keep: bool) {
        self.update_tasks(|tasks| tasks.afterCompletion.keepAfterCompletion = keep);
    }

    pub fn set_all_tasks(&mut self, selected: bool) {
        if self.is_busy() {
            return;
        }
        let enabled = &mut self.tasks.enabledTasks;
        enabled.daily_task = selected;
        enabled.get_reward = selected;
        enabled.buy_enkephalin = selected;
        enabled.mirror = selected;
        self.save_tasks();
        self.log(if selected {
            "已全选可执行任务"
        } else {
            "已清空可执行任务"
        });
    }

    pub fn toggle_task(&mut self, task: FixedTaskId) {
        if self.is_busy() {
            return;
        }
        let enabled = &mut self.tasks.enabledTasks;
        match task {
            FixedTaskId::DailyTask => enabled.daily_task = !enabled.daily_task,
            FixedTaskId::GetReward => enabled.get_reward = !enabled.get_reward,
            FixedTaskId::BuyEnkephalin => enabled.buy_enkephalin = !enabled.buy_enkephalin,
            FixedTaskId::Mirror => enabled.mirror = !enabled.mirror,
            FixedTaskId::ResonateWithAhab => {
                enabled.resonate_with_Ahab = !enabled.resonate_with_Ahab
            }
            FixedTaskId::SetWindows => return,
        }
        self.save_tasks();
    }

    pub fn toggle_detail(&mut self, task: FixedTaskId) {
        if self.is_busy() {
            return;
        }
        match task {
            FixedTaskId::SetWindows => {
                self.tasks.set_windows.set_reduce_miscontact =
                    !self.tasks.set_windows.set_reduce_miscontact;
            }
            FixedTaskId::DailyTask => {
                self.tasks.daily_task.use_continuous_combat =
                    !self.tasks.daily_task.use_continuous_combat;
            }
            FixedTaskId::GetReward => {
                self.tasks.get_reward.set_get_prize =
                    cycle_inclusive(self.tasks.get_reward.set_get_prize, 0, 2);
            }
            FixedTaskId::BuyEnkephalin => {
                self.tasks.buy_enkephalin.Dr_Grandet_mode =
                    !self.tasks.buy_enkephalin.Dr_Grandet_mode;
            }
            FixedTaskId::Mirror => {
                self.tasks.mirror.hard_mirror = !self.tasks.mirror.hard_mirror;
            }
            // The nested field is retained for contract compatibility but is
            // not an execution toggle; the top-level enabledTasks field is
            // the only selectable Ahab task setting.
            FixedTaskId::ResonateWithAhab => return,
        }
        self.save_tasks();
    }

    pub fn adjust_daily_counter(&mut self, counter: DailyCounter, delta: i8) {
        if self.is_busy() {
            return;
        }
        self.update_tasks(|tasks| {
            let (value, min, max) = match counter {
                DailyCounter::Exp => (&mut tasks.daily_task.set_EXP_count, 0, 99),
                DailyCounter::Thread => (&mut tasks.daily_task.set_thread_count, 0, 99),
                DailyCounter::Continuous => {
                    (&mut tasks.daily_task.use_continuous_combat_select, 1, 10)
                }
            };
            let next = if delta < 0 {
                value.saturating_sub((-delta) as u8)
            } else {
                value.saturating_add(delta as u8)
            };
            *value = next.clamp(min, max);
        });
    }

    pub fn adjust_enkephalin_count(&mut self, delta: i8) {
        self.update_tasks(|tasks| {
            let value = &mut tasks.buy_enkephalin.set_lunacy_to_enkephalin;
            *value = adjust_u8(*value, delta, 0, 10);
        });
    }

    pub fn adjust_mirror_count(&mut self, delta: i8) {
        self.update_tasks(|tasks| {
            let value = &mut tasks.mirror.set_mirror_count;
            *value = adjust_u8(*value, delta, 1, 99);
        });
    }

    pub fn set_window_size(&mut self, value: u16) {
        self.update_tasks(|tasks| {
            if matches!(value, 720 | 1080 | 1440) {
                tasks.set_windows.set_win_size = value;
            }
        });
    }

    pub fn set_window_position(&mut self, value: impl Into<String>) {
        let value = match value.into().as_str() {
            // Accept the short-lived numeric GPUI protocol values while
            // writing only the Python screen module's canonical names.
            "0" => "center",
            "1" => "left_top",
            "2" => "right_top",
            "3" => "free",
            value => value,
        }
        .to_owned();
        self.update_tasks(|tasks| {
            if matches!(
                value.as_str(),
                "free" | "left_top" | "right_top" | "left_bottom" | "right_bottom" | "center"
            ) {
                tasks.set_windows.set_win_position = value;
            }
        });
    }

    pub fn set_screenshot_interval(&mut self, value: f32) {
        self.update_tasks(|tasks| {
            if [0.2, 0.5, 1.0]
                .iter()
                .any(|candidate| (*candidate - value).abs() < f32::EPSILON)
            {
                tasks.set_windows.screenshot_interval = value;
            }
        });
    }

    pub fn set_mouse_action_interval(&mut self, value: f32) {
        self.update_tasks(|tasks| {
            if [0.1, 0.3, 0.5]
                .iter()
                .any(|candidate| (*candidate - value).abs() < f32::EPSILON)
            {
                tasks.set_windows.mouse_action_interval = value;
            }
        });
    }

    pub fn set_daily_team(&mut self, index: u8, value: u8) {
        self.update_tasks(|tasks| match index {
            0 => tasks.daily_task.EXP_day_1_2 = value,
            1 => tasks.daily_task.EXP_day_3_4 = value,
            2 => tasks.daily_task.EXP_day_5_6 = value,
            3 => tasks.daily_task.EXP_day_7 = value,
            4 => tasks.daily_task.thread_day_1 = value,
            5 => tasks.daily_task.thread_day_2 = value,
            6 => tasks.daily_task.thread_day_3 = value,
            7 => tasks.daily_task.thread_day_4 = value,
            8 => tasks.daily_task.thread_day_5 = value,
            9 => tasks.daily_task.thread_day_6 = value,
            10 => tasks.daily_task.thread_day_7 = value,
            11 => tasks.daily_task.daily_teams = value,
            _ => {}
        });
    }

    pub fn set_reward_mode(&mut self, value: u8) {
        self.update_tasks(|tasks| {
            if value <= 2 {
                tasks.get_reward.set_get_prize = value;
            }
        });
    }

    #[allow(dead_code)]
    pub fn cycle_number(&mut self, task: FixedTaskId) {
        if self.is_busy() {
            return;
        }
        match task {
            FixedTaskId::SetWindows => {
                self.tasks.set_windows.set_win_size = match self.tasks.set_windows.set_win_size {
                    720 => 1080,
                    1080 => 1440,
                    _ => 720,
                };
            }
            FixedTaskId::DailyTask => {
                self.tasks.daily_task.daily_teams = self.tasks.daily_task.daily_teams % 3 + 1;
            }
            FixedTaskId::GetReward => {
                self.tasks.get_reward.set_get_prize =
                    cycle_inclusive(self.tasks.get_reward.set_get_prize, 0, 2);
            }
            FixedTaskId::BuyEnkephalin => {
                self.tasks.buy_enkephalin.set_lunacy_to_enkephalin =
                    cycle_inclusive(self.tasks.buy_enkephalin.set_lunacy_to_enkephalin, 0, 10);
            }
            FixedTaskId::Mirror => {
                self.tasks.mirror.set_mirror_count =
                    cycle_inclusive(self.tasks.mirror.set_mirror_count, 1, 99);
            }
            FixedTaskId::ResonateWithAhab => return,
        }
        self.save_tasks();
    }
}
