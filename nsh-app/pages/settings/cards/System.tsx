// System & Protection, and Experimental Features: two cards of switches.
//
// A switch takes effect at once, so it writes the store and nothing else; the
// backend that acts on the value arrives with P1.
import { SettingRow, SettingList } from "../../../components/Field";
import { Switch } from "../../../components/Switch";
import { systemFlag, toggleSystemFlag } from "../../../settings";
import type { SystemFlag } from "../../../settings";
import { SettingsCard } from "../SettingsCard";

export function SystemCard() {
  return (
    <SettingsCard title="系统与防护">
      <SettingList>
        <FlagRow
          flag="memoryProtection"
          label="内存占用保护"
          detail="电脑总内存占用超过 90% 时自动清理内存防崩溃"
        />
        <FlagRow
          flag="minimizeToTray"
          label="最小化到托盘"
          detail="窗口最小化时隐藏到系统托盘区"
        />
        <FlagRow flag="autostart" label="开机自动启动" detail="跟随系统开机自启 AALC.nsh" />
      </SettingList>
    </SettingsCard>
  );
}

export function ExperimentalCard() {
  return (
    <SettingsCard title="实验性功能">
      <SettingList>
        <FlagRow
          flag="keepScreenAwake"
          label="运行时阻止休眠"
          detail="任务执行期间阻止系统与显示器进入休眠，任务结束后自动恢复"
        />
        <FlagRow
          flag="hdrWarning"
          label="显示器 HDR 检测提示"
          detail="检测到游戏处于 HDR 显示器时提示潜在图像识别问题"
        />
        <FlagRow
          flag="templateBlur"
          label="模板高斯模糊"
          detail="抗 Scrcpy H.264 压缩噪点：Scrcpy 建议开启，MuMu 可关闭省 CPU"
        />
      </SettingList>
    </SettingsCard>
  );
}

/** One setting line with its switch. */
function FlagRow(props: { flag: SystemFlag; label: string; detail: string }) {
  return (
    <SettingRow label={props.label} detail={props.detail}>
      <Switch
        checked={systemFlag(props.flag)}
        accessibilityLabel={props.label}
        onChange={() => toggleSystemFlag(props.flag)}
      />
    </SettingRow>
  );
}
