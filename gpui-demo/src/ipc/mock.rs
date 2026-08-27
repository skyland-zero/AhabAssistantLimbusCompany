use super::{
    RpcClient,
    contract::{EventEnvelope, RpcError, RpcRequest, RpcResponse, event, method},
};
use serde::Serialize;
use serde_json::{Value, json};
use std::collections::HashMap;

// Keep this client synchronous and deterministic: it is suitable for UI work
// without a Python sidecar, while its RpcClient boundary can later be backed by
// websocket.rs without changing callers.
pub struct MockClient {
    next_team_id: u64,
    sequence: super::contract::RequestSequence,
    tasks: TasksConfig,
    execution: ExecutionStatusPayload,
    teams: Vec<TeamDetail>,
    sinners: Vec<SinnerInfo>,
    packs: ThemePackState,
    resources: Vec<ResourceGroup>,
    hotkey: HotkeyConfig,
    system_settings: SystemSettingsConfig,
    devices: Vec<DeviceInfo>,
    device_status: ConnectionStatus,
    tools: HashMap<ToolId, bool>,
    events: Vec<EventEnvelope>,
}

// A compact re-export avoids making ipc users know which model submodule owns a payload.
mod model_types {
    pub use crate::model::*;
}
use model_types::*;

impl Default for MockClient {
    fn default() -> Self {
        Self {
            next_team_id: 4,
            sequence: super::contract::RequestSequence::default(),
            tasks: TasksConfig::default(),
            execution: ExecutionStatusPayload::default(),
            teams: vec![
                TeamDetail {
                    id: "team-1".into(),
                    name: "编队 1 (震颤)".into(),
                    sinners: vec![
                        "faust".into(),
                        "ishmael".into(),
                        "ryoshu".into(),
                        "hong_lu".into(),
                    ],
                    purpose: TeamPurpose::Mirror,
                    accessoryScheme: "tremor".into(),
                    enabled: true,
                    mirrorConfig: Some(TeamMirrorConfig {
                        team_system: 2,
                        discard_systems: DiscardSystems {
                            sinking: true,
                            poise: true,
                            ..Default::default()
                        },
                        opening_bonus: vec![2, 2, 1, 1, 0, 0, 0, 0, 0, 0],
                        ..Default::default()
                    }),
                },
                TeamDetail {
                    id: "team-2".into(),
                    name: "编队 2 (烧伤)".into(),
                    sinners: vec!["heathcliff".into(), "rodion".into(), "gregor".into()],
                    purpose: TeamPurpose::Luxcavation,
                    accessoryScheme: "burn".into(),
                    enabled: true,
                    mirrorConfig: Some(TeamMirrorConfig::default()),
                },
                TeamDetail {
                    id: "team-3".into(),
                    name: "编队 3 (呼吸)".into(),
                    sinners: vec![
                        "yi_sang".into(),
                        "don_quixote".into(),
                        "meursault".into(),
                        "sinclair".into(),
                        "outis".into(),
                    ],
                    purpose: TeamPurpose::General,
                    accessoryScheme: "poise".into(),
                    enabled: false,
                    mirrorConfig: Some(TeamMirrorConfig {
                        team_system: 5,
                        ..Default::default()
                    }),
                },
            ],
            sinners: vec![
                SinnerInfo {
                    id: "yi_sang".into(),
                    name: "李箱".into(),
                },
                SinnerInfo {
                    id: "faust".into(),
                    name: "浮士德".into(),
                },
                SinnerInfo {
                    id: "don_quixote".into(),
                    name: "堂吉诃德".into(),
                },
                SinnerInfo {
                    id: "ryoshu".into(),
                    name: "良秀".into(),
                },
                SinnerInfo {
                    id: "meursault".into(),
                    name: "默尔索".into(),
                },
                SinnerInfo {
                    id: "hong_lu".into(),
                    name: "鸿璐".into(),
                },
                SinnerInfo {
                    id: "heathcliff".into(),
                    name: "希斯克利夫".into(),
                },
                SinnerInfo {
                    id: "ishmael".into(),
                    name: "以实玛利".into(),
                },
                SinnerInfo {
                    id: "rodion".into(),
                    name: "罗佳".into(),
                },
                SinnerInfo {
                    id: "sinclair".into(),
                    name: "辛克莱".into(),
                },
                SinnerInfo {
                    id: "outis".into(),
                    name: "奥提斯".into(),
                },
                SinnerInfo {
                    id: "gregor".into(),
                    name: "格雷戈尔".into(),
                },
            ],
            packs: ThemePackState {
                hardMirrorActive: true,
                packs: vec![
                    ThemePack {
                        id: "pk-1".into(),
                        name: "黑云会".into(),
                        weight: 5,
                        enabled: true,
                        tier: "T1".into(),
                    },
                    ThemePack {
                        id: "pk-2".into(),
                        name: "拇指".into(),
                        weight: 3,
                        enabled: true,
                        tier: "T2".into(),
                    },
                    ThemePack {
                        id: "pk-3".into(),
                        name: "利刃兄弟会".into(),
                        weight: 4,
                        enabled: true,
                        tier: "T2".into(),
                    },
                    ThemePack {
                        id: "pk-4".into(),
                        name: "厄伍商会".into(),
                        weight: 2,
                        enabled: false,
                        tier: "T3".into(),
                    },
                    ThemePack {
                        id: "pk-5".into(),
                        name: "二十区福利机构".into(),
                        weight: 6,
                        enabled: true,
                        tier: "T1".into(),
                    },
                    ThemePack {
                        id: "pk-6".into(),
                        name: "技术科学解放者联盟".into(),
                        weight: 1,
                        enabled: false,
                        tier: "T4".into(),
                    },
                    ThemePack {
                        id: "pk-7".into(),
                        name: "公司总部".into(),
                        weight: 3,
                        enabled: true,
                        tier: "T3".into(),
                    },
                    ThemePack {
                        id: "pk-8".into(),
                        name: "残响乐团".into(),
                        weight: 7,
                        enabled: true,
                        tier: "T1".into(),
                    },
                ],
            },
            resources: vec![
                ResourceGroup {
                    id: "templates".into(),
                    name: "模板资源".into(),
                    localVersion: "v2025.06.1".into(),
                    remoteVersion: None,
                    lastSyncAt: Some(0),
                },
                ResourceGroup {
                    id: "models".into(),
                    name: "ONNX 模型".into(),
                    localVersion: "v1.2.0".into(),
                    remoteVersion: None,
                    lastSyncAt: None,
                },
            ],
            hotkey: HotkeyConfig::mock_default(),
            system_settings: SystemSettingsConfig::default(),
            devices: vec![
                DeviceInfo {
                    id: "win-limbus".into(),
                    name: "Limbus Company".into(),
                    detail: Some("1920×1080 · 窗口化".into()),
                },
                DeviceInfo {
                    id: "mumu-instance".into(),
                    name: "MuMu 模拟器".into(),
                    detail: Some("1280×720 · 端口 16384".into()),
                },
            ],
            device_status: ConnectionStatus::Disconnected,
            tools: HashMap::new(),
            events: Vec::new(),
        }
    }
}

impl MockClient {
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
    fn handle(&mut self, request: RpcRequest) -> RpcResponse {
        let id = request.id;
        let result: Result<Value, RpcError> = (|| match request.method.as_str() {
            method::APP_PING => Ok(json!("pong")),
            method::APP_VERSION => {
                Ok(json!({"ui": env!("CARGO_PKG_VERSION"), "backend": "mock-1.0.0"}))
            }
            method::APP_CHECK_UPDATE => Ok(json!(UpdateInfo {
                updateAvailable: false,
                latest: env!("CARGO_PKG_VERSION").into()
            })),
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
                    Ok(json!(true))
                } else {
                    let task = request
                        .params
                        .and_then(|value| value.get("taskId").cloned())
                        .and_then(|value| serde_json::from_value(value).ok());
                    self.execution = ExecutionStatusPayload {
                        state: ExecutionState::Running,
                        currentTaskId: task,
                    };
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
                    Ok(json!(true))
                }
            }
            method::EXECUTION_STOP => {
                self.execution = ExecutionStatusPayload::default();
                let status = self.execution.clone();
                self.emit(event::EXECUTION_STATUS, &status);
                Ok(json!(true))
            }
            method::EXECUTION_PAUSE => {
                if self.execution.state != ExecutionState::Running {
                    Err(RpcError::new(-32011, "execution is not running"))
                } else {
                    self.execution.state = ExecutionState::Paused;
                    let status = self.execution.clone();
                    self.emit(event::EXECUTION_STATUS, &status);
                    Ok(json!(true))
                }
            }
            method::EXECUTION_RESUME => {
                if self.execution.state != ExecutionState::Paused {
                    Err(RpcError::new(-32012, "execution is not paused"))
                } else {
                    self.execution.state = ExecutionState::Running;
                    let status = self.execution.clone();
                    self.emit(event::EXECUTION_STATUS, &status);
                    Ok(json!(true))
                }
            }
            method::TEAM_LIST => Ok(serde_json::to_value(&self.teams).unwrap()),
            method::TEAM_SAVE => {
                let mut team: TeamDetail =
                    Self::params(request.params, "team.save requires TeamDetail")?;
                if team.name.trim().is_empty() {
                    Err(RpcError::invalid_params("team name required"))
                } else {
                    if team.id.is_empty() {
                        team.id = format!("team-{}", self.next_team_id);
                        self.next_team_id += 1;
                    }
                    if let Some(existing) = self.teams.iter_mut().find(|entry| entry.id == team.id)
                    {
                        *existing = team;
                    } else {
                        self.teams.push(team);
                    }
                    Ok(json!(true))
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

fn has_executable_task(tasks: &TasksConfig) -> bool {
    let enabled = &tasks.enabledTasks;
    enabled.daily_task || enabled.get_reward || enabled.buy_enkephalin || enabled.mirror
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

impl RpcClient for MockClient {
    fn send(&mut self, request: RpcRequest) -> RpcResponse {
        self.handle(request)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn unknown_method_is_a_structured_error() {
        let response = MockClient::default().call("not.implemented", None);
        assert_eq!(response.error.unwrap().code, -32601);
    }
    #[test]
    fn execution_has_deterministic_state_transitions() {
        let mut client = MockClient::default();
        assert_eq!(
            client
                .call(method::EXECUTION_GET_STATE, None)
                .result
                .unwrap()["state"],
            "idle"
        );
        assert!(!client.call(method::EXECUTION_START, None).is_error());
        assert!(!client.call(method::EXECUTION_PAUSE, None).is_error());
        assert!(!client.call(method::EXECUTION_RESUME, None).is_error());
        assert!(!client.call(method::EXECUTION_STOP, None).is_error());
        assert_eq!(
            client
                .call(method::EXECUTION_GET_STATE, None)
                .result
                .unwrap()["state"],
            "idle"
        );
        assert_eq!(client.take_events().len(), 5);
    }
    #[test]
    fn execution_start_stays_idle_when_only_non_executable_settings_are_enabled() {
        for ahab_enabled in [false, true] {
            let mut client = MockClient::default();
            let mut config = TasksConfig::default();
            config.enabledTasks.daily_task = false;
            config.enabledTasks.get_reward = false;
            config.enabledTasks.buy_enkephalin = false;
            config.enabledTasks.mirror = false;
            config.enabledTasks.resonate_with_Ahab = ahab_enabled;

            assert!(
                !client
                    .call(
                        method::TASKS_SET_CONFIG,
                        Some(serde_json::to_value(config).unwrap()),
                    )
                    .is_error()
            );
            assert!(!client.call(method::EXECUTION_START, None).is_error());
            assert_eq!(
                client
                    .call(method::EXECUTION_GET_STATE, None)
                    .result
                    .unwrap()["state"],
                "idle"
            );
            assert!(client.take_events().is_empty());
        }
    }

    #[test]
    fn device_starts_disconnected_until_explicit_connect() {
        let mut client = MockClient::default();
        assert_eq!(client.device_status, ConnectionStatus::Disconnected);
        let devices = client.call(method::DEVICE_LIST, None).result.unwrap();
        assert_eq!(devices.as_array().unwrap().len(), 2);
        assert_eq!(devices[1]["id"], "mumu-instance");
        assert_eq!(devices[1]["name"], "MuMu 模拟器");
        assert!(
            !client
                .call(method::DEVICE_CONNECT, Some(json!({ "id": "win-limbus" })))
                .is_error()
        );
        let device_event = client.take_events().pop().unwrap();
        assert_eq!(device_event.event, event::DEVICE_STATUS);
        assert_eq!(device_event.payload["status"], "connected");
    }

    #[test]
    fn config_and_team_calls_round_trip() {
        let mut client = MockClient::default();
        let config = client.call(method::TASKS_GET_CONFIG, None).result.unwrap();
        assert_eq!(config["set_windows"]["set_win_size"], 1080);
        let team = json!({"id":"", "name":"test", "sinners":[], "purpose":"general", "accessoryScheme":"", "enabled":true, "mirrorConfig":null});
        assert!(!client.call(method::TEAM_SAVE, Some(team)).is_error());
        assert_eq!(
            client
                .call(method::TEAM_LIST, None)
                .result
                .unwrap()
                .as_array()
                .unwrap()
                .len(),
            4
        );
    }
}
