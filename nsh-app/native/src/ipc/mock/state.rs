//! The mock backend's deterministic starting state.

use super::*;

impl Default for MockState {
    fn default() -> Self {
        // Six recorded mirror runs: two teams, both difficulties, one failed
        // run, and every phase of the sidecar's record filled in, so the
        // console's history, the statistics page and the dialogues all have
        // data to show in mock runs and tests.
        let mirror_history = vec![
            mock_mirror(json!({
                "completedAt": "2026-08-31T21:04:00+08:00",
                "runId": "mock-run-6",
                "totalSeconds": 1520.4,
                "battleSeconds": 980.4,
                "eventSeconds": 150.0,
                "shopSeconds": 80.0,
                "findRoadSeconds": 240.0,
                "themePackSeconds": 30.0,
                "rewardCardSeconds": 15.0,
                "egoGiftSeconds": 10.0,
                "settlementSeconds": 10.0,
                "otherSeconds": 5.0,
                "eventCount": 4,
                "mode": "normal",
                "floorCount": 10,
                "routeId": "spiderweb_family_route",
                "routeName": "蜘蛛巢默认路线",
                "routeNameEn": "Spiderweb default route",
                "team": mock_team("team-2", "编队 2 (烧伤)", 2, "burn")
            })),
            mock_mirror(json!({
                "completedAt": "2026-08-31T19:12:00+08:00",
                "runId": "mock-run-5",
                "totalSeconds": 1980.5,
                "battleSeconds": 1310.5,
                "eventSeconds": 200.0,
                "shopSeconds": 110.0,
                "findRoadSeconds": 260.0,
                "themePackSeconds": 40.0,
                "rewardCardSeconds": 20.0,
                "egoGiftSeconds": 15.0,
                "settlementSeconds": 20.0,
                "otherSeconds": 5.0,
                "eventCount": 5,
                "hardMode": true,
                "mode": "hard",
                "floorCount": 15,
                "routeId": "spiderweb_family_route",
                "routeName": "蜘蛛巢默认路线",
                "routeNameEn": "Spiderweb default route",
                "team": mock_team("team-1", "编队 1 (震颤)", 1, "tremor")
            })),
            mock_mirror(json!({
                "completedAt": "2026-08-30T22:41:00+08:00",
                "runId": "mock-run-4",
                "totalSeconds": 1610.2,
                "battleSeconds": 1050.2,
                "eventSeconds": 160.0,
                "shopSeconds": 90.0,
                "findRoadSeconds": 230.0,
                "themePackSeconds": 35.0,
                "rewardCardSeconds": 15.0,
                "egoGiftSeconds": 10.0,
                "settlementSeconds": 15.0,
                "otherSeconds": 5.0,
                "eventCount": 3,
                "mode": "normal",
                "floorCount": 10,
                "routeId": "spiderweb_family_route",
                "routeName": "蜘蛛巢默认路线",
                "routeNameEn": "Spiderweb default route",
                "team": mock_team("team-2", "编队 2 (烧伤)", 2, "burn")
            })),
            mock_mirror(json!({
                "completedAt": "2026-08-30T20:07:00+08:00",
                "runId": "mock-run-3",
                "totalSeconds": 940.8,
                "battleSeconds": 720.8,
                "eventSeconds": 90.0,
                "shopSeconds": 40.0,
                "findRoadSeconds": 70.0,
                "themePackSeconds": 10.0,
                "rewardCardSeconds": 5.0,
                "egoGiftSeconds": 0.0,
                "settlementSeconds": 5.0,
                "otherSeconds": 0.0,
                "eventCount": 2,
                "failed": true,
                "failureReason": "战斗超时",
                "hardMode": true,
                "mode": "hard",
                "floorCount": 8,
                "routeId": "spiderweb_family_route",
                "routeName": "蜘蛛巢默认路线",
                "routeNameEn": "Spiderweb default route",
                "team": mock_team("team-1", "编队 1 (震颤)", 1, "tremor")
            })),
            mock_mirror(json!({
                "completedAt": "2026-08-29T23:15:00+08:00",
                "runId": "mock-run-2",
                "totalSeconds": 2050.1,
                "battleSeconds": 1390.1,
                "eventSeconds": 210.0,
                "shopSeconds": 120.0,
                "findRoadSeconds": 240.0,
                "themePackSeconds": 40.0,
                "rewardCardSeconds": 20.0,
                "egoGiftSeconds": 10.0,
                "settlementSeconds": 15.0,
                "otherSeconds": 5.0,
                "eventCount": 5,
                "hardMode": true,
                "mode": "hard",
                "floorCount": 15,
                "routeId": "spiderweb_family_route",
                "routeName": "蜘蛛巢默认路线",
                "routeNameEn": "Spiderweb default route",
                "team": mock_team("team-1", "编队 1 (震颤)", 1, "tremor")
            })),
            mock_mirror(json!({
                "completedAt": "2026-08-29T20:02:00+08:00",
                "runId": "mock-run-1",
                "totalSeconds": 1875.3,
                "battleSeconds": 1230.3,
                "eventSeconds": 190.0,
                "shopSeconds": 100.0,
                "findRoadSeconds": 250.0,
                "themePackSeconds": 30.0,
                "rewardCardSeconds": 20.0,
                "egoGiftSeconds": 20.0,
                "settlementSeconds": 25.0,
                "otherSeconds": 10.0,
                "eventCount": 4,
                "hardMode": true,
                "mode": "hard",
                "floorCount": 15,
                "routeId": "spiderweb_family_route",
                "routeName": "蜘蛛巢默认路线",
                "routeNameEn": "Spiderweb default route",
                "team": mock_team("team-2", "编队 2 (烧伤)", 2, "burn")
            })),
        ];
        Self {
            next_team_id: 4,
            next_run_id: 0,
            last_start_client_request_id: None,
            last_start_response: None,
            sequence: crate::ipc::contract::RequestSequence::default(),
            event_sequence: 0,
            tasks: TasksConfig::default(),
            execution: ExecutionStatusPayload::default(),
            stats: ExecutionStatsPayload {
                schemaVersion: 1,
                lastMirror: mirror_history.first().cloned(),
                mirrorHistory: mirror_history,
                today: StatCounts {
                    exp: 2,
                    thread: 1,
                    mirror: 3,
                },
                week: StatCounts {
                    exp: 6,
                    thread: 4,
                    mirror: 6,
                },
                ..ExecutionStatsPayload::default()
            },
            team_queue: vec!["team-1".into()],
            select_team_by_order: false,
            team_stats: HashMap::from([(
                "team-1".to_owned(),
                TeamStats {
                    schemaVersion: 1,
                    teamId: "team-1".into(),
                    teamNumber: 1,
                    totalCount: 6,
                    hard: TeamStatsBucket {
                        count: 3,
                        averageSeconds: 120.5,
                        last5AverageSeconds: 118.2,
                        last10AverageSeconds: 121.4,
                    },
                    normal: TeamStatsBucket {
                        count: 3,
                        averageSeconds: 95.0,
                        last5AverageSeconds: 94.8,
                        last10AverageSeconds: 96.1,
                    },
                },
            )]),
            teams: vec![
                TeamDetail {
                    schemaVersion: 1,
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
                    teamCode: String::new(),
                    queuePosition: 1,
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
                    schemaVersion: 1,
                    id: "team-2".into(),
                    name: "编队 2 (烧伤)".into(),
                    sinners: vec!["heathcliff".into(), "rodion".into(), "gregor".into()],
                    purpose: TeamPurpose::Luxcavation,
                    accessoryScheme: "burn".into(),
                    enabled: false,
                    teamCode: String::new(),
                    queuePosition: 0,
                    mirrorConfig: None,
                },
                TeamDetail {
                    schemaVersion: 1,
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
                    teamCode: String::new(),
                    queuePosition: 0,
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
                    id: "pc:limbus".into(),
                    name: "Limbus Company".into(),
                    detail: Some("1920×1080 · 窗口化".into()),
                },
                DeviceInfo {
                    id: "mumu:0".into(),
                    name: "MuMu 模拟器".into(),
                    detail: Some("1280×720 · 端口 16384".into()),
                },
            ],
            device_status: ConnectionStatus::Disconnected,
            preview_enabled: true,
            tools: HashMap::new(),
            events: Vec::new(),
            completions: std::collections::VecDeque::new(),
        }
    }
}

/// One mirror record of the mock's history, decoded through the model so the
/// fixture fails loudly when the contract's shape drifts.
fn mock_mirror(value: Value) -> MirrorCompletionStats {
    serde_json::from_value(value).expect("the mock's mirror fixture is valid")
}

/// The team snapshot a mock mirror record was fought with.
fn mock_team(id: &str, name: &str, number: u32, system: &str) -> Value {
    json!({
        "id": id,
        "name": name,
        "number": number,
        "sinners": [],
        "sinnerNames": [],
        "sinnerNamesEn": [],
        "system": system,
        "accessoryScheme": system,
    })
}
