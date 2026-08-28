use super::*;

impl Default for MockState {
    fn default() -> Self {
        Self {
            next_team_id: 4,
            sequence: crate::ipc::contract::RequestSequence::default(),
            tasks: TasksConfig::default(),
            execution: ExecutionStatusPayload::default(),
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
                    enabled: true,
                    mirrorConfig: Some(TeamMirrorConfig::default()),
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
            tools: HashMap::new(),
            events: Vec::new(),
        }
    }
}
