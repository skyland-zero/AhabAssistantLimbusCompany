use super::*;

use crate::{
    ipc::BackendClient,
    model::UpdateSource,
    state::other::settings::{HotkeyTarget, SystemBool},
};

fn settings() -> settings::SettingsState {
    let state = settings::SettingsState::new(BackendClient::mock());
    assert!(state.error.is_none(), "mock must load: {:?}", state.error);
    state
}

fn theme_packs() -> theme_packs::ThemePacksState {
    let state = theme_packs::ThemePacksState::new(BackendClient::mock());
    assert!(state.error.is_none(), "mock must load: {:?}", state.error);
    state
}

fn resources() -> resources::ResourcesState {
    let state = resources::ResourcesState::new(BackendClient::mock());
    assert!(state.error.is_none(), "mock must load: {:?}", state.error);
    state
}

#[test]
fn settings_load_and_persist_the_visible_patch() {
    let mut state = settings();
    assert_eq!(state.hotkey.startStop, "F10");
    assert_eq!(state.hotkey.pauseResume.as_deref(), Some("F11"));
    assert!(state.system.memory_protection);

    state.set_system_bool(SystemBool::Autostart, true).unwrap();
    assert!(state.system.autostart);
    assert_eq!(state.feedback.as_deref(), Some("设置已保存"));

    state.set_update_source(UpdateSource::MirrorChyan).unwrap();
    assert_eq!(state.system.update_source, UpdateSource::MirrorChyan);

    state.set_cdk("cdk".into());
    state.save_cdk().unwrap();
    assert_eq!(state.system.mirrorchyan_cdk, "cdk");

    state.set_wxpusher_spt("  SPT_demo  ".into());
    assert_eq!(state.system.wxpusher_spt, "SPT_demo");
    state.test_notification("SPT_unsaved".into()).unwrap();
    assert_eq!(state.feedback.as_deref(), Some("测试通知已发送"));
    assert!(state.test_notification("bad".into()).is_err());
}

#[test]
fn hotkey_changes_are_saved_and_cleared() {
    let mut state = settings();
    state.set_hotkeys_enabled(false).unwrap();
    assert!(!state.hotkey.enabled);
    state
        .set_hotkey(HotkeyTarget::StartStop, Some("Ctrl+F10".into()))
        .unwrap();
    assert_eq!(state.hotkey.startStop, "Ctrl+F10");
    state.set_hotkey(HotkeyTarget::StartStop, None).unwrap();
    assert_eq!(state.hotkey.startStop, "");
    state
        .set_hotkey(HotkeyTarget::PauseResume, Some("Ctrl+F11".into()))
        .unwrap();
    assert_eq!(state.hotkey.pauseResume.as_deref(), Some("Ctrl+F11"));
}

#[test]
fn update_check_maps_the_backend_status() {
    let mut state = settings();
    let message = state.check_update().unwrap();
    assert!(message.contains("当前已是最新版本") || message.contains("发现新版本"));
}

#[test]
fn theme_packs_load_sort_and_persist() {
    let mut state = theme_packs();
    assert_eq!(state.data.packs.len(), 8);
    assert_eq!(state.total_weight(), 28);
    assert!(state.data.hardMirrorActive);

    state.toggle_enabled("pk-4").unwrap();
    assert_eq!(state.total_weight(), 30);
    state.toggle_enabled("pk-4").unwrap();
    assert_eq!(state.total_weight(), 28);

    state.set_weight("pk-4", 5).unwrap();
    let pack = state
        .data
        .packs
        .iter()
        .find(|pack| pack.id == "pk-4")
        .unwrap();
    assert_eq!(pack.weight, 2, "disabled packs keep their weight");

    state.set_weight("pk-1", 10).unwrap();
    assert_eq!(state.total_weight(), 33);

    state.set_all_enabled(false).unwrap();
    assert_eq!(state.total_weight(), 0);
    state.set_all_enabled(true).unwrap();
    assert!(state.data.packs.iter().all(|pack| pack.enabled));

    state.set_sort_by_weight(true);
    let sorted = state.sorted_packs();
    assert!(
        sorted
            .windows(2)
            .all(|pair| pair[0].weight >= pair[1].weight)
    );

    state.reset_weights().unwrap();
    assert!(state.data.packs.iter().all(|pack| pack.weight == 1));
    assert_eq!(state.feedback.as_deref(), Some("已恢复默认权重"));
}

#[test]
fn resources_check_and_sync_follow_the_contract() {
    let mut state = resources();
    assert_eq!(state.groups.len(), 2);
    state.check_update().unwrap();
    assert!(
        state
            .groups
            .iter()
            .all(|group| group.remoteVersion.as_deref() == Some("v2025.06.2"))
    );
    assert_eq!(state.feedback.as_deref(), Some("发现资源更新"));

    let progress = state.sync_now().unwrap();
    assert_eq!(progress, 100);
    assert!(state.groups.iter().all(|group| group.lastSyncAt == Some(0)));
    assert!(
        state
            .groups
            .iter()
            .all(|group| group.localVersion == "v2025.06.2")
    );
    state.finish_sync();
    assert!(state.sync_progress.is_none());
    assert_eq!(state.feedback.as_deref(), Some("资源同步完成"));
}

#[test]
fn toolbox_toggles_tools_and_reports_resolution() {
    let mut state = toolbox::ToolboxState::new(BackendClient::mock());
    assert!(!state.is_running(crate::model::ToolId::InfiniteBattle));
    assert!(state.toggle(crate::model::ToolId::InfiniteBattle).unwrap());
    assert_eq!(state.feedback.as_deref(), Some("工具已启动"));
    assert!(!state.toggle(crate::model::ToolId::InfiniteBattle).unwrap());
    assert_eq!(state.feedback.as_deref(), Some("工具已停止"));

    assert!(state.screenshot().unwrap().contains("mock.png"));
    assert_eq!(
        state.apply_resolution().unwrap(),
        "已修改分辨率为 1080P (240 DPI)"
    );
    assert_eq!(state.reset_resolution().unwrap(), "已还原设备分辨率与 DPI");
}
