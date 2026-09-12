use super::*;

use super::super::contract::{event, method};

mod device;
mod execution;
mod settings;
mod teams;

impl MockState {
    pub fn call(&mut self, method_name: &str, params: Option<Value>) -> RpcResponse {
        let request = RpcRequest::new(self.sequence.next(), method_name, params);
        self.handle(request)
    }

    pub fn take_events(&mut self) -> Vec<EventEnvelope> {
        std::mem::take(&mut self.events)
    }

    fn params<T: serde::de::DeserializeOwned>(
        params: Option<Value>,
        description: &str,
    ) -> Result<T, RpcError> {
        serde_json::from_value(params.unwrap_or(Value::Null))
            .map_err(|error| RpcError::invalid_params(format!("{description}: {error}")))
    }

    pub(super) fn emit<T: Serialize>(&mut self, name: &str, payload: &T) {
        if let Ok(value) = serde_json::to_value(payload) {
            self.event_sequence = self.event_sequence.wrapping_add(1);
            self.events
                .push(EventEnvelope::new(name, value).with_sequence(self.event_sequence.max(1)));
        }
    }

    fn emit_stats(&mut self) {
        let stats = self.stats.clone();
        self.emit(event::EXECUTION_STATS, &stats);
    }

    fn advance_execution_revision(&mut self) {
        self.execution.stateRevision = self.execution.stateRevision.saturating_add(1);
    }

    fn execution_result(&self, accepted: bool, reason: Option<&str>) -> Value {
        let mut result = serde_json::to_value(&self.execution).unwrap_or_else(|_| json!({}));
        if let Some(object) = result.as_object_mut() {
            object.insert("accepted".into(), json!(accepted));
            if let Some(reason) = reason {
                object.insert("reason".into(), json!(reason));
            }
        }
        result
    }

    fn requested_run_id(params: Option<&Value>) -> Option<&str> {
        params
            .and_then(Value::as_object)
            .and_then(|params| params.get("runId"))
            .and_then(Value::as_str)
    }

    fn validate_run_id(&self, params: Option<&Value>) -> Result<(), RpcError> {
        let Some(requested) = Self::requested_run_id(params) else {
            return Ok(());
        };
        if self.execution.runId.as_deref() == Some(requested) {
            Ok(())
        } else {
            Err(RpcError::with_data(
                -32013,
                "STALE_RUN",
                json!({"code": "STALE_RUN", "runId": requested}),
            ))
        }
    }

    pub(super) fn handle(&mut self, request: RpcRequest) -> RpcResponse {
        let id = request.id;
        let result: Result<Value, RpcError> = (|| match request.method.as_str() {
            method::APP_PING => Ok(json!("pong")),
            method::APP_VERSION => Ok(json!({
                "schemaVersion": 3,
                "ui": env!("CARGO_PKG_VERSION"),
                "backend": "mock-1.0.0"
            })),
            method::APP_CHECK_UPDATE => Ok(json!({
                "schemaVersion": 3,
                "status": "up_to_date",
                "updateAvailable": false,
                "latest": env!("CARGO_PKG_VERSION")
            })),
            method::STATS_GET_SUMMARY => Ok(serde_json::to_value(&self.stats).unwrap()),
            method::STATS_GET_DAILY_SUMMARY => Ok(serde_json::to_value(DailyStatsPayload {
                schemaVersion: 1,
                dateFrom: String::new(),
                dateTo: String::new(),
                days: Vec::new(),
                updatedAt: 0,
            })
            .unwrap()),
            method::TASKS_GET_CONFIG => Ok(serde_json::to_value(&self.tasks).unwrap()),
            method::TASKS_SET_CONFIG => {
                self.tasks = Self::params(request.params, "tasks.setConfig requires TasksConfig")?;
                Ok(json!(true))
            }
            method::EXECUTION_GET_STATE
            | method::EXECUTION_START
            | method::EXECUTION_STOP
            | method::EXECUTION_PAUSE
            | method::EXECUTION_RESUME => self.handle_execution(&request),
            method::TEAM_LIST
            | method::TEAM_STATS_GET
            | method::TEAM_STATS_CLEAR
            | method::TEAM_PRESET_LIST
            | method::TEAM_SAVE
            | method::TEAM_DELETE
            | method::SINNER_LIST => self.handle_teams(&request),
            method::THEME_PACK_LIST
            | method::THEME_PACK_UPDATE_ALL
            | method::THEME_PACK_RESET_WEIGHTS
            | method::RESOURCE_STATUS
            | method::RESOURCE_CHECK_UPDATE
            | method::RESOURCE_SYNC_START
            | method::HOTKEY_GET
            | method::HOTKEY_SET
            | method::SYSTEM_SETTINGS_GET
            | method::SYSTEM_SETTINGS_SET
            | method::NOTIFICATION_TEST
            | method::PREVIEW_SET_ENABLED => self.handle_settings(&request),
            method::TOOL_START
            | method::TOOL_STOP
            | method::TOOL_SCREENSHOT
            | method::TOOL_RESOLUTION_SET
            | method::TOOL_RESOLUTION_RESET
            | method::DEVICE_LIST
            | method::DEVICE_CONNECT
            | method::DEVICE_DISCONNECT => self.handle_device(&request),
            unknown => Err(RpcError::method_not_found(unknown)),
        })();
        match result {
            Ok(value) => RpcResponse::success(id, value),
            Err(error) => RpcResponse::failure(id, error),
        }
    }
}

fn builtin_team_presets() -> Vec<TeamPreset> {
    let normal_solo_config = TeamMirrorConfig {
        team_system: 4,
        shop_strategy: 1,
        reward_cards: true,
        reward_cards_select: 3,
        opening_items: true,
        opening_items_select: 0,
        opening_items_system: 4,
        do_not_buy: true,
        do_not_fuse: true,
        do_not_enhance: true,
        do_not_heal: true,
        do_not_sell: true,
        ignore_shop: vec![true; 5],
        max_keyword_refresh: 2,
        max_normal_refresh: 3,
        defense_for_solo: true,
        skill_replacement: false,
        skill_replacement_select: 0,
        skill_replacement_mode: 0,
        use_starlight: true,
        opening_bonus: vec![1, 1, 1, 1, 0, 0, 0, 0, 0, 0],
        use_team_code: true,
        team_code: "H4sIAAAAAAAACnMxcUwvD8x2DAh0dgQBc0dPEOVS4ZgOop0iIcKm5WBhVxeIsH8xWNjJORCiuhIiHJAPUe0GEXZ0tLUFAH9Z+5NgAAAA".into(),
        observe_ego_gift: true,
        observe_ego_gift_selected: vec![SPIDERWEB_ENTANGLED_IN_RED_GIFT_ID.to_owned()],
        mirror_route_profile: "hos_ryoshu_solo_route".into(),
        ..TeamMirrorConfig::default()
    };
    let mut hard_solo_config = normal_solo_config.clone();
    hard_solo_config.do_not_buy = false;
    hard_solo_config.do_not_fuse = false;
    hard_solo_config.do_not_enhance = false;
    hard_solo_config.skill_replacement = true;
    hard_solo_config.ignore_shop = vec![false; 5];
    hard_solo_config.opening_bonus = vec![3; 10];

    let normal_solo_sinners = vec![
        "ryoshu".into(),
        "don_quixote".into(),
        "heathcliff".into(),
        "faust".into(),
        "outis".into(),
        "yi_sang".into(),
        "ishmael".into(),
    ];
    let solo_sinners = vec![
        "ryoshu".into(),
        "yi_sang".into(),
        "rodion".into(),
        "meursault".into(),
        "gregor".into(),
        "heathcliff".into(),
        "outis".into(),
        "hong_lu".into(),
        "faust".into(),
        "ishmael".into(),
        "don_quixote".into(),
        "sinclair".into(),
    ];
    let normal_solo_team = TeamDetail {
        schemaVersion: 1,
        id: String::new(),
        name: "小指良伪单通（普牢）".into(),
        sinners: normal_solo_sinners,
        purpose: TeamPurpose::Mirror,
        accessoryScheme: "poise".into(),
        enabled: false,
        mirrorConfig: Some(normal_solo_config),
    };
    let hard_solo_team = TeamDetail {
        schemaVersion: 1,
        id: String::new(),
        name: "小指良伪单通（困牢）".into(),
        sinners: solo_sinners,
        purpose: TeamPurpose::Mirror,
        accessoryScheme: "poise".into(),
        enabled: false,
        mirrorConfig: Some(hard_solo_config),
    };

    let spider_config = TeamMirrorConfig {
        team_system: 4,
        use_team_code: true,
        team_code: "H4sIAAAAAAAACg3MQRJAMAxA0UthZ/HTFGE6ZKx6gtS4ALfnHeDpq0QERyQVCXqs5hV9aOE3hTTLBb2bn0ZeuNwr+52yiguPlW1GB6LZn9QzpxZ0eLUJGMcPt8GoUGAAAAA=".into(),
        observe_ego_gift: true,
        observe_ego_gift_selected: vec![SPIDERWEB_ENTANGLED_IN_RED_GIFT_ID.to_owned()],
        mirror_route_profile: "spiderweb_family_route".into(),
        ..TeamMirrorConfig::default()
    };
    let spider_team = TeamDetail {
        schemaVersion: 1,
        id: String::new(),
        name: "蜘蛛巢全家桶".into(),
        sinners: vec![
            "ryoshu".into(),
            "yi_sang".into(),
            "rodion".into(),
            "don_quixote".into(),
            "hong_lu".into(),
            "outis".into(),
            "heathcliff".into(),
            "faust".into(),
            "ishmael".into(),
            "sinclair".into(),
        ],
        purpose: TeamPurpose::Mirror,
        accessoryScheme: "poise".into(),
        enabled: false,
        mirrorConfig: Some(spider_config),
    };

    vec![
        TeamPreset {
            presetId: "hos_ryoshu_solo_normal".into(),
            routeId: "hos_ryoshu_solo_route".into(),
            name: LocalizedText {
                zhCn: "小指良伪单通（普牢）".into(),
                enUs: "Ryoshu Pseudo-Solo (Normal)".into(),
            },
            description: LocalizedText {
                zhCn: "锁定 7 人编队（良秀首位，中指/环指父辈前置促成斩杀目标裂变），后备席留空实现 1 回合启动。普通镜牢全流程零商店纯 P 速刷（不买、不合、不强化），极大压缩现实过图时间。".into(),
                enUs: "7-sinner lineup (Ryoshu first, Middle/Ring patriarchs placed early to split kill targets) with empty bench for 1-turn startup. Pure-P zero-shop speedrun for normal Mirror Dungeons (no buy, fuse, or enhance) to minimize real-world run time.".into(),
            },
            floorHint: LocalizedText {
                zhCn: "适用于普通镜牢，执行 1–5 层".into(),
                enUs: "For normal Mirror Dungeons; runs floors 1–5.".into(),
            },
            routeName: LocalizedText {
                zhCn: "House of Spiders 良秀伪单通路线".into(),
                enUs: "House of Spiders Ryoshu route".into(),
            },
            team: normal_solo_team,
        },
        TeamPreset {
            presetId: "hos_ryoshu_solo_hard".into(),
            routeId: "hos_ryoshu_solo_route".into(),
            name: LocalizedText {
                zhCn: "小指良伪单通（困牢）".into(),
                enUs: "Ryoshu Pseudo-Solo (Hard)".into(),
            },
            description: LocalizedText {
                zhCn: "良秀首位，李箱与罗佳随后；4–6 号位安排优先牺牲人格。困难镜牢开局使用 House of Spiders 攻略的十个 ++ 星光。".into(),
                enUs: "Ryoshu starts first, followed by Yi Sang and Rodion; slots 4–6 are prioritized sacrifices. Uses ten level-++ starting bonuses from the House of Spiders guide for hard Mirror Dungeons.".into(),
            },
            floorHint: LocalizedText {
                zhCn: "适用于困难镜牢，执行 1–15 层并开启平行叠加".into(),
                enUs: "For hard Mirror Dungeons; runs floors 1–15 with Parallel Superposition.".into(),
            },
            routeName: LocalizedText {
                zhCn: "House of Spiders 良秀伪单通路线".into(),
                enUs: "House of Spiders Ryoshu route".into(),
            },
            team: hard_solo_team,
        },
        TeamPreset {
            presetId: "spiderweb_family".into(),
            routeId: "spiderweb_family_route".into(),
            name: LocalizedText {
                zhCn: "蜘蛛巢全家桶".into(),
                enUs: "House of Spiders Full Roster".into(),
            },
            description: LocalizedText {
                zhCn: "保留现有蜘蛛巢专属 Gift Search 的全家桶编队。".into(),
                enUs: "The full House of Spiders roster with the existing exclusive Gift Search.".into(),
            },
            floorHint: LocalizedText {
                zhCn: "沿用当前镜牢流程".into(),
                enUs: "Uses the current mirror-dungeon flow.".into(),
            },
            routeName: LocalizedText {
                zhCn: "蜘蛛巢默认路线".into(),
                enUs: "Spiderweb default route".into(),
            },
            team: spider_team,
        },
    ]
}

fn has_executable_task(tasks: &TasksConfig) -> bool {
    first_executable_task(tasks).is_some()
}

fn first_executable_task(tasks: &TasksConfig) -> Option<FixedTaskId> {
    let enabled = &tasks.enabledTasks;
    if enabled.daily_task {
        Some(FixedTaskId::DailyTask)
    } else if enabled.get_reward {
        Some(FixedTaskId::GetReward)
    } else if enabled.buy_enkephalin {
        Some(FixedTaskId::BuyEnkephalin)
    } else if enabled.mirror {
        Some(FixedTaskId::Mirror)
    } else {
        None
    }
}

fn merge_json<T: Serialize + serde::de::DeserializeOwned>(
    current: &T,
    patch: Option<Value>,
    label: &str,
) -> Result<T, RpcError> {
    let mut value = serde_json::to_value(current)
        .map_err(|error| RpcError::invalid_params(format!("{label}: {error}")))?;
    let patch =
        patch.ok_or_else(|| RpcError::invalid_params(format!("{label} requires an object")))?;
    let (Some(base), Some(changes)) = (value.as_object_mut(), patch.as_object()) else {
        return Err(RpcError::invalid_params(format!(
            "{label} requires an object"
        )));
    };
    base.extend(changes.clone());
    serde_json::from_value(value)
        .map_err(|error| RpcError::invalid_params(format!("{label}: {error}")))
}
