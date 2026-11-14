// The toolbox page's state: which tools run, and what the last action said.
//
// A tool request goes through the Rust toolbox store (`tool.start` /
// `tool.stop` / `tool.screenshot` / resolution commands); the page keeps reading
// the same functions.
import { signal } from "nsh";
import { Backend, ToolboxStore } from "aalc/native";
import { translatedFeedback } from "./settings";

/** The four tools the page offers, as the sidecar names them. */
export type ToolId = "infiniteBattle" | "enkephalin" | "screenshot" | "resolution";

/** What the page draws for one tool. */
export const TOOLS: {
  id: ToolId;
  icon: "crosshair" | "pill" | "camera" | "monitor";
  title: string;
  description: string;
}[] = [
  {
    id: "infiniteBattle",
    icon: "crosshair",
    title: "自动战斗",
    description: "循环执行战斗直至手动停止",
  },
  {
    id: "enkephalin",
    icon: "pill",
    title: "体力换饼",
    description: "自动将狂气转换为体力并合成脑啡肽模块，防止体力溢出",
  },
  {
    id: "screenshot",
    icon: "camera",
    title: "辅助截图",
    description: "截取当前游戏窗口画面并保存到 AALC.nsh 目录",
  },
  {
    id: "resolution",
    icon: "monitor",
    title: "分辨率修改",
    description:
      "进入游戏后点击，通过 ADB 将 Android 设备修改为 1080P 横屏 (1920x1080 240DPI)，并支持一键还原",
  },
];

const running = signal<Record<ToolId, boolean>>({
  infiniteBattle: false,
  enkephalin: false,
  screenshot: false,
  resolution: false,
});

const feedback = signal<string | null>(null);

let store: ToolboxStore | null = null;
let connecting = false;

function ensureStore(): ToolboxStore | null {
  if (store) return store;
  if (Backend.mode() === "mock") {
    store = new ToolboxStore();
    syncRunning();
  } else if (!connecting) {
    connecting = true;
    ToolboxStore.connect()
      .then((created) => {
        store = created;
      })
      .catch((error) => {
        feedback.set(error instanceof Error ? error.message : String(error));
      })
      .finally(() => {
        connecting = false;
      });
  }
  return store;
}

function syncRunning(): void {
  if (!store) return;
  running.set({
    infiniteBattle: store.running("infiniteBattle"),
    enkephalin: store.running("enkephalin"),
    screenshot: store.running("screenshot"),
    resolution: store.running("resolution"),
  });
}

if (Backend.mode() === "mock") ensureStore();

/** Whether `id` is running (tracked where it is read). */
export function isToolRunning(id: ToolId): boolean {
  ensureStore();
  return running()[id];
}

/** The line under the tool grid, when the last action left something to say. */
export function toolboxFeedback(): string | null {
  return translatedFeedback(feedback());
}

/** Starts or stops a tool. */
export function toggleTool(id: ToolId): void {
  if (id === "screenshot") return;
  const backend = ensureStore();
  if (!backend) return;
  try {
    backend.toggle(id);
    syncRunning();
    feedback.set(backend.feedback());
  } catch (error) {
    feedback.set(error instanceof Error ? error.message : String(error));
  }
}

/** Captures the game window. */
export function screenshot(): void {
  const backend = ensureStore();
  if (!backend) return;
  try {
    backend.screenshot();
    feedback.set(backend.feedback());
  } catch (error) {
    feedback.set(error instanceof Error ? error.message : String(error));
  }
}

/** Switches the device to 1080P landscape through ADB. */
export function applyResolution(): void {
  const backend = ensureStore();
  if (!backend) return;
  try {
    backend.applyResolution();
    feedback.set(backend.feedback());
  } catch (error) {
    feedback.set(error instanceof Error ? error.message : String(error));
  }
}

/** Puts the device's resolution and DPI back. */
export function resetResolution(): void {
  const backend = ensureStore();
  if (!backend) return;
  try {
    backend.resetResolution();
    feedback.set(backend.feedback());
  } catch (error) {
    feedback.set(error instanceof Error ? error.message : String(error));
  }
}
