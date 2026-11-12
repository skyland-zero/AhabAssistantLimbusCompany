// The global-hotkey card: one switch, two key combinations.
//
// Capturing a combination needs the window's own keystroke stream and the
// system's global-shortcut registration, which are P1's `nsh/desktop`
// `GlobalShortcut` work. Until then the rows show what is bound and can clear
// it; the capture button says so instead of pretending to listen.
import { Text, View } from "nsh";
import { Button } from "../../../components/Button";
import { SettingLine, SettingList } from "../../../components/Field";
import { Switch } from "../../../components/Switch";
import {
  clearHotkey,
  currentHotkeys,
  setFeedback,
  setHotkeysEnabled,
} from "../../../settings";
import { SettingsCard } from "../SettingsCard";

export function HotkeysCard() {
  const hotkeys = () => currentHotkeys();
  return (
    <SettingsCard title="全局热键">
      <SettingList>
        <SettingLine label="启用全局热键">
          <Switch
            checked={hotkeys().enabled}
            accessibilityLabel="启用全局热键"
            onChange={({ checked }) => setHotkeysEnabled(checked)}
          />
        </SettingLine>
        <SettingLine label="启动 / 停止热键">
          <Combo
            target="startStop"
            value={hotkeys().startStop}
            disabled={!hotkeys().enabled}
          />
        </SettingLine>
        <SettingLine label="暂停 / 继续热键">
          <Combo
            target="pauseResume"
            value={hotkeys().pauseResume}
            disabled={!hotkeys().enabled}
          />
        </SettingLine>
      </SettingList>
    </SettingsCard>
  );
}

/** One combination: the current keys, and the command that clears them. */
function Combo(props: {
  target: "startStop" | "pauseResume";
  value: string;
  disabled: boolean;
}) {
  return (
    <View class="flex-row items-center gap-2">
      <View
        class={`min-w-32 flex-row items-center rounded-(--radius) border border-input bg-card px-3 py-1 ${
          props.disabled ? "opacity-50" : ""
        }`}
      >
        <Text class={`text-xs ${props.value ? "text-foreground" : "text-muted-foreground"}`}>
          {props.value || "未设置"}
        </Text>
      </View>
      <Button
        label="清除"
        variant="ghost"
        disabled={props.disabled || !props.value}
        onPress={() => {
          clearHotkey(props.target);
          setFeedback("已清除热键（保存与全局注册随 P1 接入）");
        }}
      />
    </View>
  );
}
