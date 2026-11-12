// The settings page's state: the values the page edits.
//
// The same split as `store.ts`: signals stay private and the page calls the
// functions. This is a mirror of the sidecar's settings payload (defaults from
// the original client's model), P1 replaces the writes with RPC calls and the
// initial values with what the backend reports; the page does not change.
import { signal } from "nsh";
import type { UpdateSource } from "./model/types";

/** A system flag, as the sidecar names it. */
export type SystemFlag =
  | "memoryProtection"
  | "minimizeToTray"
  | "autostart"
  | "keepScreenAwake"
  | "hdrWarning"
  | "templateBlur"
  | "prerelease";

/** The interface language; copy lands with the i18n module in P1. */
export type Language = "zhCn" | "enUs";

const flags = signal<Record<SystemFlag, boolean>>({
  memoryProtection: true,
  minimizeToTray: true,
  autostart: false,
  keepScreenAwake: true,
  hdrWarning: true,
  templateBlur: false,
  prerelease: false,
});

const updateSource = signal<UpdateSource>("github");
const cdk = signal("");
const wxpusherSpt = signal("");
const language = signal<Language>("zhCn");

const hotkeys = signal({
  enabled: true,
  startStop: "F10",
  pauseResume: "F11",
});

/** The value of `flag` (tracked where it is read). */
export function systemFlag(flag: SystemFlag): boolean {
  return flags()[flag];
}

/** Flips `flag`. */
export function toggleSystemFlag(flag: SystemFlag): void {
  flags.update((current) => ({ ...current, [flag]: !current[flag] }));
}

/** The update mirror in use (tracked). */
export function currentUpdateSource(): UpdateSource {
  return updateSource();
}

/** Chooses the update mirror. */
export function setUpdateSource(next: UpdateSource): void {
  updateSource.set(next);
}

/** The Mirror-Chyan CDK as typed (tracked). */
export function currentCdk(): string {
  return cdk();
}

/** Stores the CDK on screen; saving is P1's `Settings.setMirrorChyanCdk`. */
export function setCdk(next: string): void {
  cdk.set(next);
}

/** The WxPusher SPT as typed (tracked). */
export function currentWxpusherSpt(): string {
  return wxpusherSpt();
}

/** Stores the SPT on screen; saving is P1's `Settings.setWxpusherSpt`. */
export function setWxpusherSpt(next: string): void {
  wxpusherSpt.set(next);
}

/** The interface language (tracked). */
export function currentLanguage(): Language {
  return language();
}

/** Chooses the interface language. */
export function setLanguage(next: Language): void {
  language.set(next);
}

/** The global hotkey configuration (tracked). */
export function currentHotkeys(): { enabled: boolean; startStop: string; pauseResume: string } {
  return hotkeys();
}

/** Turns global hotkeys on or off. */
export function setHotkeysEnabled(enabled: boolean): void {
  hotkeys.update((current) => ({ ...current, enabled }));
}

/** Clears one of the two hotkeys. */
export function clearHotkey(target: "startStop" | "pauseResume"): void {
  hotkeys.update((current) => ({ ...current, [target]: "" }));
}

const feedback = signal<string | null>(null);

/** The line under the card stack, when the last action left something to say. */
export function currentFeedback(): string | null {
  return feedback();
}

/** Says what an action did; `null` clears it. */
export function setFeedback(next: string | null): void {
  feedback.set(next);
}
