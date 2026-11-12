// The settings page's state.
//
// Rust owns the system flags, the update source, the saved CDK/SPT and the
// global-hotkey config. This module mirrors the returned snapshots into signals
// and keeps the language choice (an app-shell preference) on screen.
import { signal } from "nsh";
import { Backend, I18n, SettingsStore } from "aalc/native";
import type { SystemView } from "aalc/native";
import { currentLanguage, setLanguage } from "./store";
import type { UpdateSource } from "./model/types";

export { currentLanguage, setLanguage };
export type { Language } from "./store";

/** A system flag, as the sidecar names it. */
export type SystemFlag =
  | "memoryProtection"
  | "minimizeToTray"
  | "autostart"
  | "keepScreenAwake"
  | "hdrWarning"
  | "templateBlur"
  | "prerelease";

const DEFAULT_FLAGS: Record<SystemFlag, boolean> = {
  memoryProtection: true,
  minimizeToTray: true,
  autostart: false,
  keepScreenAwake: true,
  hdrWarning: true,
  templateBlur: false,
  prerelease: false,
};

const flags = signal<Record<SystemFlag, boolean>>({ ...DEFAULT_FLAGS });
const updateSource = signal<UpdateSource>("github");
const cdk = signal("");
const wxpusherSpt = signal("");

const hotkeys = signal({
  enabled: true,
  startStop: "F10",
  pauseResume: "F11",
});

const hotkeyCapture = signal<"startStop" | "pauseResume" | null>(null);

const feedback = signal<string | null>(null);

let store: SettingsStore | null = null;
let connecting = false;

function hydrateSystem(view: SystemView): void {
  flags.set({
    memoryProtection: view.memoryProtection,
    minimizeToTray: view.minimizeToTray,
    autostart: view.autostart,
    keepScreenAwake: view.experimentalKeepScreenAwake,
    hdrWarning: view.experimentalHdrWarning,
    templateBlur: view.enableTemplateBlur,
    prerelease: view.updatePrereleaseEnable,
  });
  updateSource.set(view.updateSource === "mirrorChyan" ? "mirrorChyan" : "github");
  cdk.set(view.mirrorchyanCdk);
  wxpusherSpt.set(view.wxpusherSpt);
}

function hydrateHotkeys(view: {
  enabled: boolean;
  startStop: string;
  pauseResume?: string | null;
}): void {
  hotkeys.set({
    enabled: view.enabled,
    startStop: view.startStop,
    pauseResume: view.pauseResume ?? "",
  });
}

function failure(error: unknown): void {
  feedback.set(error instanceof Error ? error.message : String(error));
}

/** The Rust store once it is loaded, or `null` while a sidecar connects. */
function ensureStore(): SettingsStore | null {
  if (store) return store;
  if (Backend.mode() === "mock") {
    try {
      store = new SettingsStore();
      hydrateSystem(store.system());
      hydrateHotkeys(store.hotkeys());
    } catch (error) {
      failure(error);
    }
  } else if (!connecting) {
    connecting = true;
    SettingsStore.connect()
      .then((created) => {
        store = created;
        hydrateSystem(created.system());
        hydrateHotkeys(created.hotkeys());
      })
      .catch(failure)
      .finally(() => {
        connecting = false;
      });
  }
  return store;
}

if (Backend.mode() === "mock") ensureStore();

/** The value of `flag` (tracked where it is read). */
export function systemFlag(flag: SystemFlag): boolean {
  ensureStore();
  return flags()[flag];
}

/** Flips `flag` and persists the visible settings patch. */
export function toggleSystemFlag(flag: SystemFlag): void {
  const backend = ensureStore();
  const next = !flags()[flag];
  if (!backend) {
    flags.update((current) => ({ ...current, [flag]: next }));
    return;
  }
  try {
    hydrateSystem(backend.setSystemFlag(flag, next));
    feedback.set("设置已保存");
  } catch (error) {
    failure(error);
  }
}

/** The update mirror in use (tracked). */
export function currentUpdateSource(): UpdateSource {
  ensureStore();
  return updateSource();
}

/** Chooses the update mirror and persists it. */
export function setUpdateSource(next: UpdateSource): void {
  const backend = ensureStore();
  if (!backend) {
    updateSource.set(next);
    return;
  }
  try {
    hydrateSystem(backend.setUpdateSource(next));
    feedback.set("设置已保存");
  } catch (error) {
    failure(error);
  }
}

/** The Mirror-Chyan CDK as typed (tracked). */
export function currentCdk(): string {
  return cdk();
}

/** Stores the CDK on screen; `saveCdk` persists it. */
export function setCdk(next: string): void {
  cdk.set(next);
  ensureStore()?.setCdk(next);
}

/** Persists the Mirror-Chyan CDK. */
export function saveCdk(): void {
  const backend = ensureStore();
  if (!backend) return;
  try {
    hydrateSystem(backend.saveCdk());
    feedback.set("设置已保存");
  } catch (error) {
    failure(error);
  }
}

/** The WxPusher SPT as typed (tracked). */
export function currentWxpusherSpt(): string {
  return wxpusherSpt();
}

/** Stores the SPT on screen; `saveWxpusherSpt` persists it. */
export function setWxpusherSpt(next: string): void {
  wxpusherSpt.set(next);
  ensureStore()?.setSpt(next);
}

/** Persists the WxPusher SPT. */
export function saveWxpusherSpt(): void {
  const backend = ensureStore();
  if (!backend) return;
  try {
    hydrateSystem(backend.saveSpt());
    feedback.set("设置已保存");
  } catch (error) {
    failure(error);
  }
}

/** Sends a test notification through the sidecar. */
export function testNotification(): void {
  const backend = ensureStore();
  if (!backend) return;
  try {
    backend.testNotification();
    feedback.set(backend.feedback() ?? "测试通知已发送");
  } catch (error) {
    failure(error);
  }
}

/** Checks for an update and reports what the backend found. */
export function checkForUpdates(): void {
  const backend = ensureStore();
  if (!backend) return;
  try {
    feedback.set(backend.checkUpdate());
  } catch (error) {
    failure(error);
  }
}

/** The global hotkey configuration (tracked). */
export function currentHotkeys(): { enabled: boolean; startStop: string; pauseResume: string } {
  ensureStore();
  return hotkeys();
}

/** Turns global hotkeys on or off and saves the config. */
export function setHotkeysEnabled(enabled: boolean): void {
  const backend = ensureStore();
  if (!backend) {
    hotkeys.update((current) => ({ ...current, enabled }));
    return;
  }
  try {
    hydrateHotkeys(backend.setHotkeysEnabled(enabled));
    feedback.set("设置已保存");
  } catch (error) {
    failure(error);
  }
}

/** Clears one of the two hotkeys and saves the config. */
export function clearHotkey(target: "startStop" | "pauseResume"): void {
  const backend = ensureStore();
  if (!backend) {
    hotkeys.update((current) => ({ ...current, [target]: "" }));
    return;
  }
  try {
    hydrateHotkeys(backend.clearHotkey(target));
    feedback.set("设置已保存");
  } catch (error) {
    failure(error);
  }
}

/** Sets one hotkey combination and saves the config. */
export function setHotkey(target: "startStop" | "pauseResume", value: string): void {
  const backend = ensureStore();
  if (!backend) return;
  try {
    hydrateHotkeys(backend.setHotkey(target, value));
    feedback.set("设置已保存");
  } catch (error) {
    failure(error);
  }
}

/** The hotkey row currently listening for a combination (tracked). */
export function currentHotkeyCapture(): "startStop" | "pauseResume" | null {
  return hotkeyCapture();
}

/** Starts listening for a combination for `target`. */
export function beginHotkeyCapture(target: "startStop" | "pauseResume"): void {
  hotkeyCapture.set(target);
}

/** Stops listening without changing the hotkey. */
export function cancelHotkeyCapture(): void {
  hotkeyCapture.set(null);
}

/** Saves the combination captured for `target`. */
export function finishHotkeyCapture(
  target: "startStop" | "pauseResume",
  value: string,
): void {
  hotkeyCapture.set(null);
  setHotkey(target, value);
}

/** The line under the card stack, in the current language. */
export function currentFeedback(): string | null {
  return translatedFeedback(feedback());
}

/** Says what an action did; `null` clears it. */
export function setFeedback(next: string | null): void {
  feedback.set(next);
}

/** The catalog entry for a shell/page key in the current language. */
export function t(key: string): string {
  return I18n.text(currentLanguage(), key);
}

/** A state-layer feedback line in the current language. */
export function translatedFeedback(message: string | null): string | null {
  return message === null ? null : I18n.feedback(message, currentLanguage());
}
