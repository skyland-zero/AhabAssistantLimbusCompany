// The toolbox page's state: which tools run, and what the last action said.
//
// A tool is a request to the sidecar (`Toolbox.start` / `Toolbox.stop` /
// `Toolbox.screenshot` / device resolution). P1 replaces the mock below with
// those calls and feeds `running` from the `tool.status` events; the page keeps
// reading the same functions.
import { signal } from "nsh";

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
    description: "截取当前游戏窗口画面并保存到 AALC 目录",
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

/** Whether `id` is running (tracked where it is read). */
export function isToolRunning(id: ToolId): boolean {
  return running()[id];
}

/** The line under the tool grid, when the last action left something to say. */
export function toolboxFeedback(): string | null {
  return feedback();
}

/** Starts or stops a tool. */
export function toggleTool(id: ToolId): void {
  const next = !running()[id];
  running.update((current) => ({ ...current, [id]: next }));
  feedback.set(next ? "工具已启动" : "工具已停止");
}

/** Captures the game window. */
export function screenshot(): void {
  feedback.set("截图完成：AALC/screenshot.png");
}

/** Switches the device to 1080P landscape through ADB. */
export function applyResolution(): void {
  feedback.set("已修改分辨率为 1080P (240 DPI)");
}

/** Puts the device's resolution and DPI back. */
export function resetResolution(): void {
  feedback.set("已还原设备分辨率与 DPI");
}
