// The global-hotkey card: one switch, two key combinations.
//
// "捕获" listens to the window's keystroke stream through `nsh`'s keymap while
// it is active, then stores the combination through the Rust settings store.
// Escape cancels without changing the binding.
import { Text, View, effect, keymap } from "nsh";
import { Button } from "../../../components/Button";
import { SettingLine, SettingList } from "../../../components/Field";
import { Switch } from "../../../components/Switch";
import {
  beginHotkeyCapture,
  cancelHotkeyCapture,
  clearHotkey,
  currentHotkeyCapture,
  currentHotkeys,
  finishHotkeyCapture,
  setHotkeysEnabled,
} from "../../../settings";
import { SettingsCard } from "../SettingsCard";

/** Keys the capture listens for, before modifiers are added. */
const KEYS = [
  ...Array.from({ length: 24 }, (_, index) => `f${index + 1}`),
  ..."abcdefghijklmnopqrstuvwxyz".split(""),
  ..."0123456789".split(""),
  "up",
  "down",
  "left",
  "right",
  "home",
  "end",
  "pageup",
  "pagedown",
  "insert",
  "delete",
  "minus",
  "plus",
  "/",
  ",",
  ".",
  ";",
  "'",
  "[",
  "]",
  "\\",
  "`",
  "space",
  "tab",
  "enter",
];

const MODIFIERS = [
  "",
  "ctrl+",
  "alt+",
  "shift+",
  "ctrl+alt+",
  "ctrl+shift+",
  "alt+shift+",
  "ctrl+alt+shift+",
];

/** Every combination the capture mode can hear. */
const CANDIDATES = MODIFIERS.flatMap((modifier) => KEYS.map((key) => `${modifier}${key}`));

/** `ctrl+f10` → `Ctrl+F10`, the spelling the settings store keeps. */
function formatShortcut(keystroke: string): string {
  return keystroke
    .split("+")
    .map((part) => {
      if (part === "ctrl" || part === "control" || part === "mod") return "Ctrl";
      if (part === "alt" || part === "option") return "Alt";
      if (part === "shift") return "Shift";
      if (part === "cmd" || part === "command" || part === "super" || part === "meta") {
        return "Super";
      }
      return part.length === 1 ? part.toUpperCase() : part[0].toUpperCase() + part.slice(1);
    })
    .join("+");
}

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

/** One combination: the current keys, capture, and clear. */
function Combo(props: {
  target: "startStop" | "pauseResume";
  value: string;
  disabled: boolean;
}) {
  const capturing = () => currentHotkeyCapture() === props.target;

  // While this row captures, bind the candidate keystrokes for this component's
  // lifetime. The handlers are removed when capture ends.
  effect(() => {
    const target = currentHotkeyCapture();
    if (target === null) return;
    const stops: (() => void)[] = [];
    try {
      stops.push(keymap.bind("escape", () => cancelHotkeyCapture()));
    } catch {
      // Escape may be reserved on some platforms; capture still works.
    }
    for (const candidate of CANDIDATES) {
      try {
        stops.push(
          keymap.bind(candidate, (event) => {
            finishHotkeyCapture(target, formatShortcut(event.keystroke));
          }),
        );
      } catch {
        // A platform-reserved combination is skipped.
      }
    }
    return () => {
      for (const stop of stops) stop();
    };
  });

  return (
    <View class="flex-row items-center gap-2">
      <View
        class={`min-w-32 flex-row items-center rounded-(--radius) border px-3 py-1 ${
          capturing()
            ? "border-brand bg-brand/10"
            : "border-input bg-card"
        } ${props.disabled ? "opacity-50" : ""}`}
      >
        <Text class={`text-xs ${capturing() || props.value ? "text-foreground" : "text-muted-foreground"}`}>
          {capturing() ? "按下组合键…" : props.value || "未设置"}
        </Text>
      </View>
      <Button
        label={capturing() ? "取消" : "捕获"}
        variant="outline"
        size="xs"
        testID={`hotkey-capture-${props.target}`}
        disabled={props.disabled}
        onPress={() =>
          capturing() ? cancelHotkeyCapture() : beginHotkeyCapture(props.target)
        }
      />
      <Button
        label="清除"
        variant="ghost"
        size="xs"
        testID={`hotkey-clear-${props.target}`}
        disabled={props.disabled || !props.value}
        onPress={() => clearHotkey(props.target)}
      />
    </View>
  );
}
