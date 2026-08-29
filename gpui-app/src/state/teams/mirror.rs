use super::*;

impl TeamsState {
    pub fn update_mirror(&mut self, update: impl FnOnce(&mut TeamMirrorConfig)) {
        if let Some(editor) = self.editor.as_mut()
            && let Some(config) = editor.team.mirrorConfig.as_mut()
        {
            update(config);
        }
    }

    pub fn toggle_discard_system(&mut self, index: usize) {
        self.update_mirror(|config| {
            let systems = &mut config.discard_systems;
            match index {
                0 => systems.burn = !systems.burn,
                1 => systems.bleed = !systems.bleed,
                2 => systems.tremor = !systems.tremor,
                3 => systems.rupture = !systems.rupture,
                4 => systems.sinking = !systems.sinking,
                5 => systems.poise = !systems.poise,
                6 => systems.charge = !systems.charge,
                7 => systems.slash = !systems.slash,
                8 => systems.pierce = !systems.pierce,
                9 => systems.blunt = !systems.blunt,
                _ => {}
            }
        });
    }

    pub fn set_mirror_bool(&mut self, field: MirrorBool, value: bool) {
        self.update_mirror(|config| match field {
            MirrorBool::DoNotHeal => config.do_not_heal = value,
            MirrorBool::DoNotBuy => config.do_not_buy = value,
            MirrorBool::DoNotFuse => config.do_not_fuse = value,
            MirrorBool::DoNotSell => config.do_not_sell = value,
            MirrorBool::DoNotEnhance => config.do_not_enhance = value,
            MirrorBool::OnlyAggressiveFuse => config.only_aggressive_fuse = value,
            MirrorBool::DoNotSystemFuse => config.do_not_system_fuse = value,
            MirrorBool::OnlySystemFuse => config.only_system_fuse = value,
            MirrorBool::AggressiveAlsoEnhance => config.aggressive_also_enhance = value,
            MirrorBool::AggressiveSaveSystems => config.aggressive_save_systems = value,
            MirrorBool::AfterLevelIv => config.after_level_IV = value,
            MirrorBool::SecondSystem => config.second_system = value,
            MirrorBool::SecondSystemFuseIv => config.second_system_fuse_IV = value,
            MirrorBool::SecondSystemBuy => config.second_system_buy = value,
            MirrorBool::SecondSystemSelectReward => config.second_system_select_reward = value,
            MirrorBool::SecondSystemPowerUp => config.second_system_power_up = value,
            MirrorBool::AvoidSkill3 => {
                config.avoid_skill_3 = value;
                if value {
                    config.prioritize_skill_3 = false;
                }
            }
            MirrorBool::PrioritizeSkill3 => {
                config.prioritize_skill_3 = value;
                if value {
                    config.avoid_skill_3 = false;
                }
            }
            MirrorBool::ReformationEachFloor => config.re_formation_each_floor = value,
            MirrorBool::DefenseFirstRound => {
                config.defense_first_round = value;
                if value {
                    config.defense_for_solo = false;
                }
            }
            MirrorBool::DefenseForSolo => {
                config.defense_for_solo = value;
                if value {
                    config.defense_first_round = false;
                }
            }
            MirrorBool::SkillReplacement => config.skill_replacement = value,
            MirrorBool::UseStarlight => config.use_starlight = value,
            MirrorBool::FixedTeamUse => config.fixed_team_use = value,
            MirrorBool::UseTeamCode => config.use_team_code = value,
            MirrorBool::UseCustomThemeWeight => config.use_custom_theme_pack_weight = value,
            MirrorBool::ObserveEgoGift => config.observe_ego_gift = value,
        });
    }

    pub fn set_mirror_u8(&mut self, field: MirrorU8, value: u8) {
        self.update_mirror(|config| match field {
            MirrorU8::ShopStrategy => config.shop_strategy = value.min(2),
            MirrorU8::AfterLevelIvSelect => config.after_level_IV_select = value.min(3),
            MirrorU8::MaxKeywordRefresh => config.max_keyword_refresh = value.min(10),
            MirrorU8::MaxNormalRefresh => config.max_normal_refresh = value.min(10),
            MirrorU8::SecondSystemSelect => config.second_system_select = value.min(9),
            MirrorU8::SecondSystemStartFloor => config.second_system_setting = value.min(1),
            MirrorU8::DefenseTurns => config.defense_for_solo_turns = value.clamp(1, 5),
            MirrorU8::SkillReplacementMode => config.skill_replacement_mode = value.min(1),
            MirrorU8::FixedTeamUseSelect => config.fixed_team_use_select = value.min(2),
        });
    }

    pub fn toggle_ignore_shop(&mut self, floor: usize) {
        self.update_mirror(|config| {
            if let Some(value) = config.ignore_shop.get_mut(floor) {
                *value = !*value;
            }
        });
    }

    pub fn set_starlight_level(&mut self, index: usize, level: u8) {
        self.update_mirror(|config| {
            if index < 10 {
                if config.opening_bonus.len() < 10 {
                    config.opening_bonus.resize(10, 0);
                }
                config.opening_bonus[index] = level.min(3);
            }
        });
    }

    pub fn set_all_starlight(&mut self, level: u8) {
        self.update_mirror(|config| config.opening_bonus = vec![level.min(3); 10]);
    }

    pub fn starlight_cost(&self) -> u32 {
        let costs = [10_u32, 10, 20, 20, 30, 30, 40, 40, 50, 60];
        self.editor
            .as_ref()
            .and_then(|editor| editor.team.mirrorConfig.as_ref())
            .map(|config| {
                config
                    .opening_bonus
                    .iter()
                    .take(10)
                    .enumerate()
                    .map(|(index, level)| costs[index] * u32::from(*level))
                    .sum()
            })
            .unwrap_or(0)
    }

    pub fn add_observe_gift(&mut self, gift: &str) -> bool {
        let gift = gift.trim();
        if gift.is_empty() {
            return false;
        }
        let mut added = false;
        self.update_mirror(|config| {
            if !config
                .observe_ego_gift_selected
                .iter()
                .any(|item| item == gift)
            {
                config.observe_ego_gift_selected.push(gift.to_owned());
                added = true;
            }
        });
        added
    }

    pub fn remove_observe_gift(&mut self, gift: &str) {
        self.update_mirror(|config| {
            config.observe_ego_gift_selected.retain(|item| item != gift);
        });
    }
}
