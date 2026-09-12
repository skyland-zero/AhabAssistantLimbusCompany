//! Single translation point for state-layer feedback messages.
//!
//! The state layer stores human-readable Chinese feedback strings so the
//! logic stays free of render concerns.  Pages must not keep their own
//! partial lookup tables (they drifted and produced mixed-language UI); this
//! catalog is the only place that maps those messages to English.

use crate::model::Language;

/// Exact-message translations for every state-layer feedback literal.
///
/// Keep this table next to the state catalog: when a new state method emits a
/// user-visible message, add its English form here.  [`feedback`] falls back
/// to the original message so an unmapped string is visible rather than lost.
const EN_FEEDBACK: &[(&str, &str)] = &[
    // Resources
    ("正在检查资源更新", "Checking for resource updates"),
    ("发现资源更新", "Resource updates found"),
    ("资源已是最新版本", "Resources are up to date"),
    ("正在启动资源同步", "Starting resource sync"),
    ("资源同步完成", "Resource sync completed"),
    ("资源同步已启动", "Resource sync started"),
    // Settings / update
    ("正在发送测试通知", "Sending test notification"),
    ("测试通知已发送", "Test notification sent"),
    ("正在检查更新", "Checking for updates"),
    ("设置已保存", "Settings saved"),
    ("已请求打开 GitHub 仓库", "GitHub repository opened"),
    ("更新检查失败", "Update check failed"),
    ("未知", "Unknown"),
    // Theme packs
    ("正在恢复默认权重", "Restoring default weights"),
    ("已恢复默认权重", "Default weights restored"),
    ("主题包设置待保存", "Theme-pack settings pending save"),
    ("主题包设置已保存", "Theme-pack settings saved"),
    ("正在保存主题包设置", "Saving theme-pack settings"),
    // Toolbox
    ("正在提交工具请求", "Submitting tool request"),
    ("正在截图", "Taking a screenshot"),
    ("工具已启动", "Tool started"),
    ("工具已停止", "Tool stopped"),
    ("正在修改设备分辨率", "Changing device resolution"),
    ("正在还原设备分辨率", "Restoring device resolution"),
    (
        "已修改分辨率为 1080P (240 DPI)",
        "Resolution changed to 1080P (240 DPI)",
    ),
    (
        "已修改分辨率为 1080P (240 DPI)，Scrcpy 已重连",
        "Resolution changed to 1080P (240 DPI), Scrcpy reconnected",
    ),
    (
        "已还原设备分辨率与 DPI",
        "Device resolution and DPI restored",
    ),
    (
        "已还原设备分辨率与 DPI，Scrcpy 已重连",
        "Device resolution and DPI restored, Scrcpy reconnected",
    ),
    ("未知路径", "Unknown path"),
    // Teams
    (
        "已导入队伍 JSON（尚未保存）",
        "Team JSON imported (not saved yet)",
    ),
    ("队伍保存中…", "Saving team…"),
    ("队伍删除中…", "Deleting team…"),
    ("队伍已保存", "Team saved"),
    ("队伍已删除", "Team deleted"),
    ("队伍已启用", "Team enabled"),
    ("队伍已停用", "Team disabled"),
    ("正在启用队伍…", "Enabling team…"),
    ("正在停用队伍…", "Disabling team…"),
    ("队伍统计已清空", "Team statistics cleared"),
    ("队伍 JSON 已复制", "Team JSON copied"),
    (
        "队伍正在保存，请等待后端响应",
        "The team is being saved; waiting for the backend",
    ),
    (
        "队伍正在保存，请稍候",
        "The team is being saved; please wait",
    ),
    (
        "队伍正在删除，请稍候",
        "The team is being deleted; please wait",
    ),
    (
        "队伍启用状态正在更新，请稍候",
        "The team enabled state is being updated; please wait",
    ),
    ("队伍名称不能为空", "Team name is required"),
    (
        "队伍最多选择 12 名人格",
        "A team can contain at most 12 sinners",
    ),
    ("队伍 JSON 必须是对象", "Team JSON must be an object"),
    ("队伍 JSON 缺少 name", "Team JSON is missing name"),
    ("purpose 无效", "Invalid team purpose"),
    ("sinners 无效", "Invalid sinner list"),
    ("mirrorConfig 必须是对象", "mirrorConfig must be an object"),
    ("mirrorConfig 默认值无效", "Invalid mirrorConfig defaults"),
    ("当前没有打开队伍编辑器", "No team editor is open"),
    ("当前没有打开预设选择器", "No preset picker is open"),
    ("当前没有待删除的队伍", "No team is pending deletion"),
    (
        "当前没有待确认的统计清空操作",
        "No pending statistics clear operation",
    ),
    (
        "当前没有待确认的预设覆盖操作",
        "No pending preset overwrite operation",
    ),
    (
        "未保存的队伍不能切换启用状态",
        "An unsaved team cannot change its enabled state",
    ),
    (
        "未保存的队伍没有统计数据",
        "An unsaved team has no statistics",
    ),
    (
        "未找到所选内置预设",
        "The selected built-in preset was not found",
    ),
    (
        "经验本队伍不参与镜牢启用队列",
        "Luxcavation teams do not join the mirror enabled queue",
    ),
    (
        "team.save 返回了空结果",
        "team.save returned an empty result",
    ),
    (
        "team.delete 返回了无效结果",
        "team.delete returned an invalid result",
    ),
];

/// Translate a state-layer feedback message for display.
pub fn feedback(message: &str, language: Language) -> String {
    if matches!(language, Language::ZhCn) {
        return message.to_owned();
    }
    if let Some((_, translated)) = EN_FEEDBACK.iter().find(|(zh, _)| *zh == message) {
        return (*translated).to_owned();
    }
    // Dynamic message shapes produced by the state layer.
    if let Some(path) = message.strip_prefix("截图完成：") {
        return format!("Screenshot saved: {path}");
    }
    if let Some(error) = message.strip_prefix("导入失败：") {
        return format!("Import failed: {error}");
    }
    if let Some(version) = message.strip_prefix("发现新版本：") {
        return format!("Update available: {version}");
    }
    if let Some(version) = message.strip_prefix("当前已是最新版本：") {
        return format!("You are on the latest version ({version})");
    }
    if let Some(error) = message.strip_prefix("team.list 返回了无效队伍：") {
        return format!("team.list returned an invalid team: {error}");
    }
    message.to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_catalog_entry_translates_and_keeps_chinese_unchanged() {
        for (zh, en) in EN_FEEDBACK {
            assert_ne!(zh, en, "catalog entry {zh} has no English form");
            assert_eq!(feedback(zh, Language::ZhCn), *zh);
            assert_eq!(feedback(zh, Language::EnUs), *en);
        }
    }

    #[test]
    fn dynamic_messages_are_translated() {
        assert_eq!(
            feedback("截图完成：C:/a.png", Language::EnUs),
            "Screenshot saved: C:/a.png"
        );
        assert_eq!(
            feedback("发现新版本：v1.2.3", Language::EnUs),
            "Update available: v1.2.3"
        );
        assert_eq!(
            feedback("当前已是最新版本：v1.2.3", Language::EnUs),
            "You are on the latest version (v1.2.3)"
        );
        assert_eq!(
            feedback("导入失败：bad json", Language::EnUs),
            "Import failed: bad json"
        );
    }

    #[test]
    fn unknown_messages_pass_through_in_both_languages() {
        assert_eq!(feedback("backend text", Language::EnUs), "backend text");
        assert_eq!(feedback("backend text", Language::ZhCn), "backend text");
    }
}
