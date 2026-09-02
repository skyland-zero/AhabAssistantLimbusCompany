use super::*;

use super::super::contract::{event, method};

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

    fn emit<T: Serialize>(&mut self, name: &str, payload: &T) {
        if let Ok(value) = serde_json::to_value(payload) {
            self.events.push(EventEnvelope::new(name, value));
        }
    }

    fn emit_stats(&mut self) {
        let stats = self.stats.clone();
        self.emit(event::EXECUTION_STATS, &stats);
    }

    pub(super) fn handle(&mut self, request: RpcRequest) -> RpcResponse {
        let id = request.id;
        let result: Result<Value, RpcError> = (|| match request.method.as_str() {
            method::APP_PING => Ok(json!("pong")),
            method::APP_VERSION => Ok(json!({
                "schemaVersion": 2,
                "ui": env!("CARGO_PKG_VERSION"),
                "backend": "mock-1.0.0"
            })),
            method::APP_CHECK_UPDATE => Ok(json!({
                "schemaVersion": 2,
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
            method::EXECUTION_GET_STATE => Ok(serde_json::to_value(&self.execution).unwrap()),
            method::EXECUTION_START => {
                if self.execution.state != ExecutionState::Idle {
                    Err(RpcError::new(-32010, "execution is not idle"))
                } else if !has_executable_task(&self.tasks) {
                    // A direct IPC caller must observe the same no-op behavior as
                    // HomeState: window settings and Ahab resonance are not
                    // executable tasks on their own.
                    Ok(json!({
                        "accepted": false,
                        "runId": null,
                        "state": "idle",
                        "reason": "没有选择可执行任务"
                    }))
                } else {
                    let task = request
                        .params
                        .and_then(|value| value.get("taskId").cloned())
                        .and_then(|value| serde_json::from_value(value).ok())
                        .or_else(|| first_executable_task(&self.tasks));
                    self.execution = ExecutionStatusPayload {
                        state: ExecutionState::Running,
                        currentTaskId: task,
                    };
                    let infinite =
                        self.tasks.enabledTasks.mirror && self.tasks.mirror.infinite_dungeons;
                    self.stats = ExecutionStatsPayload {
                        schemaVersion: 1,
                        currentRun: CurrentRunStats {
                            runId: Some("mock-run".into()),
                            state: ExecutionState::Running,
                            currentTaskId: self.execution.currentTaskId,
                            startedAt: Some(0),
                            targets: StatCounts {
                                exp: if self.tasks.enabledTasks.daily_task {
                                    self.tasks.daily_task.set_EXP_count.into()
                                } else {
                                    0
                                },
                                thread: if self.tasks.enabledTasks.daily_task {
                                    self.tasks.daily_task.set_thread_count.into()
                                } else {
                                    0
                                },
                                mirror: if self.tasks.enabledTasks.mirror && !infinite {
                                    self.tasks.mirror.set_mirror_count.into()
                                } else {
                                    0
                                },
                            },
                            completed: StatCounts::default(),
                            isMirrorInfinite: infinite,
                            updatedAt: Some(0),
                        },
                        lastMirror: self.stats.lastMirror.clone(),
                        mirrorHistory: self.stats.mirrorHistory.clone(),
                        today: self.stats.today.clone(),
                        week: self.stats.week.clone(),
                        updatedAt: 0,
                    };
                    self.emit_stats();
                    let status = self.execution.clone();
                    self.emit(event::EXECUTION_STATUS, &status);
                    if self.tasks.enabledTasks.mirror {
                        self.emit(
                            event::EXECUTION_MIRROR_PROGRESS,
                            &MirrorProgressPayload {
                                current: 1,
                                total: if self.tasks.mirror.infinite_dungeons {
                                    9999
                                } else {
                                    u32::from(self.tasks.mirror.set_mirror_count)
                                },
                                isHard: self.tasks.mirror.hard_mirror,
                                isInfinite: self.tasks.mirror.infinite_dungeons,
                            },
                        );
                    }
                    Ok(json!({
                        "accepted": true,
                        "runId": "mock-run",
                        "state": "running"
                    }))
                }
            }
            method::EXECUTION_STOP => {
                self.execution = ExecutionStatusPayload::default();
                self.stats.currentRun.state = ExecutionState::Idle;
                self.stats.currentRun.currentTaskId = None;
                self.emit_stats();
                let status = self.execution.clone();
                self.emit(event::EXECUTION_STATUS, &status);
                Ok(json!({
                    "accepted": true,
                    "runId": null,
                    "state": "idle"
                }))
            }
            method::EXECUTION_PAUSE => {
                if self.execution.state != ExecutionState::Running {
                    Err(RpcError::new(-32011, "execution is not running"))
                } else {
                    self.execution.state = ExecutionState::Paused;
                    self.stats.currentRun.state = ExecutionState::Paused;
                    self.emit_stats();
                    let status = self.execution.clone();
                    self.emit(event::EXECUTION_STATUS, &status);
                    Ok(json!({
                        "accepted": true,
                        "runId": "mock-run",
                        "state": "paused"
                    }))
                }
            }
            method::EXECUTION_RESUME => {
                if self.execution.state != ExecutionState::Paused {
                    Err(RpcError::new(-32012, "execution is not paused"))
                } else {
                    self.execution.state = ExecutionState::Running;
                    self.stats.currentRun.state = ExecutionState::Running;
                    self.emit_stats();
                    let status = self.execution.clone();
                    self.emit(event::EXECUTION_STATUS, &status);
                    Ok(json!({
                        "accepted": true,
                        "runId": "mock-run",
                        "state": "running"
                    }))
                }
            }
            method::TEAM_LIST => Ok(serde_json::to_value(&self.teams).unwrap()),
            method::TEAM_PRESET_LIST => Ok(serde_json::to_value(builtin_team_presets()).unwrap()),
            method::TEAM_SAVE => {
                let raw: Value = Self::params(request.params, "team.save requires an object")?;
                let raw_object = raw
                    .as_object()
                    .ok_or_else(|| RpcError::invalid_params("team.save requires an object"))?;
                let team_id = raw_object
                    .get("id")
                    .and_then(Value::as_str)
                    .unwrap_or_default();
                if !team_id.is_empty() && raw_object.contains_key("teamNumber") {
                    Err(RpcError::invalid_params(
                        "team.save.teamNumber is only valid when creating a team",
                    ))
                } else {
                    let mut team: TeamDetail = if team_id.is_empty() {
                        Self::params(Some(raw.clone()), "team.save requires TeamDetail")?
                    } else {
                        let existing = self
                            .teams
                            .iter()
                            .find(|entry| entry.id == team_id)
                            .cloned()
                            .ok_or_else(|| RpcError::invalid_params("team.save team not found"))?;
                        let mut merged = serde_json::to_value(existing).unwrap();
                        let merged_object = merged.as_object_mut().unwrap();
                        merged_object.extend(raw_object.clone());
                        Self::params(Some(merged), "team.save requires TeamDetail")?
                    };
                    if team.name.trim().is_empty() {
                        Err(RpcError::invalid_params("team name required"))
                    } else {
                        if team.id.is_empty() {
                            let requested_number = raw_object
                                .get("teamNumber")
                                .map(|value| {
                                    value.as_u64().ok_or_else(|| {
                                        RpcError::invalid_params(
                                            "team.save.teamNumber requires a positive integer",
                                        )
                                    })
                                })
                                .transpose()?;
                            let team_number = requested_number.unwrap_or(self.next_team_id);
                            if team_number == 0 {
                                return Err(RpcError::invalid_params(
                                    "team.save.teamNumber requires a positive integer",
                                ));
                            }
                            let requested_id = format!("team-{team_number}");
                            if self.teams.iter().any(|entry| entry.id == requested_id) {
                                return Err(RpcError::invalid_params("team number already in use"));
                            }
                            team.id = requested_id;
                            self.next_team_id = self.next_team_id.max(team_number + 1);
                        }
                        if team.purpose == TeamPurpose::Luxcavation {
                            team.enabled = false;
                            team.mirrorConfig = None;
                        } else if team.mirrorConfig.is_none()
                            && let Some(existing) =
                                self.teams.iter().find(|entry| entry.id == team.id)
                        {
                            team.mirrorConfig = existing.mirrorConfig.clone();
                        }
                        if let Some(existing) =
                            self.teams.iter_mut().find(|entry| entry.id == team.id)
                        {
                            *existing = team.clone();
                        } else {
                            self.teams.push(team.clone());
                        }
                        Ok(json!(team))
                    }
                }
            }
            method::TEAM_DELETE => {
                let id_value: Value = Self::params(request.params, "team.delete requires {id}")?;
                let team_id = id_value
                    .get("id")
                    .and_then(Value::as_str)
                    .ok_or_else(|| RpcError::invalid_params("team.delete requires a string id"))?;
                self.teams.retain(|team| team.id != team_id);
                Ok(json!(true))
            }
            method::SINNER_LIST => Ok(serde_json::to_value(&self.sinners).unwrap()),
            method::THEME_PACK_LIST => Ok(serde_json::to_value(&self.packs).unwrap()),
            method::THEME_PACK_UPDATE_ALL => {
                let value: Value =
                    Self::params(request.params, "themePack.updateAll requires {packs}")?;
                let packs: Vec<ThemePack> =
                    Self::params(value.get("packs").cloned(), "themePack.updateAll packs")?;
                self.packs.packs = packs;
                Ok(json!(true))
            }
            method::THEME_PACK_RESET_WEIGHTS => {
                self.packs.packs.iter_mut().for_each(|pack| pack.weight = 1);
                Ok(json!(self.packs.clone()))
            }
            method::RESOURCE_STATUS => Ok(serde_json::to_value(&self.resources).unwrap()),
            method::RESOURCE_CHECK_UPDATE => {
                for resource in &mut self.resources {
                    resource.remoteVersion = Some("v2025.06.2".into());
                }
                // Checking updates returns the same resource collection the
                // UI renders, matching the canonical `resource.status`
                // response shape and avoiding a second incompatible payload.
                Ok(serde_json::to_value(&self.resources).unwrap())
            }
            method::RESOURCE_SYNC_START => {
                let scope = request
                    .params
                    .as_ref()
                    .and_then(|value| value.get("scope"))
                    .and_then(Value::as_str)
                    .unwrap_or("all")
                    .to_owned();
                self.emit(
                    event::RESOURCE_SYNC_PROGRESS,
                    &SyncProgressPayload {
                        scope: scope.clone(),
                        progress: 100,
                    },
                );
                for resource in &mut self.resources {
                    if scope == "all" || resource.id == scope {
                        resource.localVersion = "v2025.06.2".into();
                        resource.lastSyncAt = Some(0);
                    }
                }
                Ok(json!(true))
            }
            method::TOOL_START | method::TOOL_STOP => {
                let value: Value = Self::params(request.params, "tool call requires {id}")?;
                let tool: ToolId =
                    Self::params(value.get("id").cloned(), "tool call requires a valid id")?;
                let running = request.method == method::TOOL_START;
                self.tools.insert(tool, running);
                self.emit(
                    event::TOOL_STATUS,
                    &ToolStatusPayload {
                        toolId: tool,
                        running,
                    },
                );
                Ok(json!(true))
            }
            method::TOOL_SCREENSHOT => Ok(json!({"path":"AALC/screenshots/mock.png"})),
            method::TOOL_RESOLUTION_SET => Ok(
                json!({"accepted": true, "size": "1920x1080", "density": 240, "reconnected": false}),
            ),
            method::TOOL_RESOLUTION_RESET => Ok(json!({"accepted": true, "reconnected": false})),
            method::HOTKEY_GET => Ok(serde_json::to_value(&self.hotkey).unwrap()),
            method::HOTKEY_SET => {
                self.hotkey = merge_json(&self.hotkey, request.params, "hotkey.set")?;
                Ok(json!(true))
            }
            method::SYSTEM_SETTINGS_GET => Ok(serde_json::to_value(&self.system_settings).unwrap()),
            method::SYSTEM_SETTINGS_SET => {
                self.system_settings =
                    merge_json(&self.system_settings, request.params, "systemSettings.set")?;
                Ok(json!(true))
            }
            method::NOTIFICATION_TEST => {
                let value: Value =
                    Self::params(request.params, "notification.test requires {spt}")?;
                let spt = match value.get("spt") {
                    Some(Value::String(value)) => value.trim(),
                    Some(_) => return Err(RpcError::invalid_params("SPT 必须是字符串")),
                    None => self.system_settings.wxpusher_spt.trim(),
                };
                if spt.is_empty() {
                    return Err(RpcError::invalid_params("SPT 未配置"));
                }
                if spt.len() <= 4 || !spt.starts_with("SPT_") {
                    return Err(RpcError::invalid_params("SPT 格式无效"));
                }
                Ok(json!({"accepted": true}))
            }
            method::PREVIEW_SET_ENABLED => {
                let value: Value =
                    Self::params(request.params, "preview.setEnabled requires {enabled}")?;
                let enabled = value
                    .get("enabled")
                    .and_then(Value::as_bool)
                    .ok_or_else(|| {
                        RpcError::invalid_params("preview.setEnabled.enabled must be a boolean")
                    })?;
                self.preview_enabled = enabled;
                Ok(json!({
                    "enabled": enabled,
                    "running": enabled && self.device_status == ConnectionStatus::Connected
                }))
            }
            method::DEVICE_LIST => Ok(serde_json::to_value(&self.devices).unwrap()),
            method::DEVICE_CONNECT => {
                let value: Value = Self::params(request.params, "device.connect requires {id}")?;
                let device_id = value
                    .get("id")
                    .and_then(Value::as_str)
                    .ok_or_else(|| RpcError::invalid_params("device.connect requires a string id"))?
                    .to_owned();
                self.device_status = ConnectionStatus::Connected;
                self.emit(
                    event::DEVICE_STATUS,
                    &DeviceStatusPayload {
                        deviceId: Some(device_id),
                        status: self.device_status,
                    },
                );
                Ok(json!(true))
            }
            method::DEVICE_DISCONNECT => {
                self.device_status = ConnectionStatus::Disconnected;
                self.emit(
                    event::DEVICE_STATUS,
                    &DeviceStatusPayload {
                        deviceId: None,
                        status: self.device_status,
                    },
                );
                Ok(json!(true))
            }
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
        skill_replacement_mode: 1,
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
