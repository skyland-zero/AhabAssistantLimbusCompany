// Desktop integration: the tray icon, close-to-tray and the global hotkeys.
//
// These need the platform's desktop module, so everything is gated on
// `isAvailable`; on a target without it the component draws nothing.
import { View, effect } from "nsh";
import { Application, GlobalShortcut, Tray, Window, isAvailable } from "nsh/desktop";
import {
  pauseOrResumeQueue,
  startQueue,
  stopQueue,
} from "../home";
import { currentHomeStatus } from "../store";
import { currentHotkeys, systemFlag, t } from "../settings";

/** `Ctrl+F10` → `ctrl+f10`, the spelling the desktop module takes. */
function normalizeShortcut(value: string): string {
  return value
    .trim()
    .toLowerCase()
    .replace(/\s+/g, "")
    .replace("cmdorctrl", "mod")
    .replace("control", "ctrl")
    .replace("option", "alt");
}

function toggleQueue(): void {
  if (currentHomeStatus() === "idle") startQueue();
  else stopQueue();
}

export function DesktopIntegration() {
  // The tray icon and its menu follow the queue state and the language.
  effect(() => {
    if (!isAvailable) return;
    const state = currentHomeStatus();
    currentLanguageTracked();
    const running = state !== "idle";
    const window = Window.get("main");
    const visible = window?.state()?.visible ?? true;
    Tray.set({
      tooltip: "AALC",
      menu: [
        {
          label: visible ? t("trayHide") : t("trayShow"),
          onSelect: () => {
            if (visible) window?.hide();
            else window?.focus();
          },
        },
        { type: "separator" },
        {
          label: running ? t("trayStop") : t("trayStart"),
          onSelect: () => toggleQueue(),
        },
        {
          label: state === "paused" ? t("trayResume") : t("trayPause"),
          disabled: !running,
          onSelect: () => pauseOrResumeQueue(),
        },
        { type: "separator" },
        { label: t("trayQuit"), onSelect: () => Application.quit() },
      ],
      onClick: () => Window.get("main")?.focus(),
    });
    return () => Tray.set(null);
  });

  // Closing the window hides it while "minimize to tray" is on.
  effect(() => {
    if (!isAvailable) return;
    const window = Window.get("main");
    if (!window) return;
    const minimize = systemFlag("minimizeToTray");
    window.set({ closeOnPress: false });
    return window.on("closePress", () => {
      if (minimize) window.hide();
      else Application.quit();
    });
  });

  // Register both global hotkeys whenever the configuration changes.
  effect(() => {
    if (!isAvailable) return;
    const config = currentHotkeys();
    GlobalShortcut.unregisterAll();
    if (config.enabled) {
      if (config.startStop) {
        void GlobalShortcut.register(normalizeShortcut(config.startStop), toggleQueue).catch(
          () => undefined,
        );
      }
      if (config.pauseResume) {
        void GlobalShortcut
          .register(normalizeShortcut(config.pauseResume), pauseOrResumeQueue)
          .catch(() => undefined);
      }
    }
    return () => GlobalShortcut.unregisterAll();
  });

  return <View class="hidden" />;
}

/** Reads the language signal so the tray effect re-runs on a language switch. */
function currentLanguageTracked(): void {
  // `t()` itself reads the language; this keeps the dependency explicit for a
  // reader and for the compiler's tracking pass.
  t("appName");
}
