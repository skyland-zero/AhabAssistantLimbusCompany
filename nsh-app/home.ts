// The console page's state: the backend link, the device, the five tasks and
// the run log.
//
// Every value here is what the sidecar reports (`Device.*`, `Execution.*`,
// `Stats.*`, `Settings.*`) or what the user set for the next run. P1 replaces
// the fixtures and the writes with RPC calls and the execution events; the page
// keeps reading the same functions.
import { signal } from "nsh";

/** What the Python sidecar link is doing. */
export type BackendState = "mock" | "starting" | "ready" | "failed";

/** The device the console drives. */
export type DeviceState = "disconnected" | "connecting" | "connected";

/** One console task card: the queue's unit of work. */
export type TaskId =
  | "daily"
  | "rewards"
  | "enkephalin"
  | "mirror"
  | "resonance";

/** One parameter shown as a small select on a task card. */
export type TaskParam = { id: string; label: string; value: string; options: { value: string; label: string }[] };

/** One task card. */
export type Task = {
  id: TaskId;
  icon: "listChecks" | "package" | "sparkles" | "target" | "audioLines";
  name: string;
  enabled: boolean;
  params: TaskParam[];
};

const backend = signal<BackendState>("mock");
const devices = signal<{ id: string; name: string }[]>([]);
const device = signal<DeviceState>("disconnected");
const resolution = signal("1080P");
const asyncInput = signal(false);
const logCount = signal(0);
const feedback = signal<string | null>(null);

const tasks = signal<Task[]>([
  {
    id: "daily",
    icon: "listChecks",
    name: "日常任务",
    enabled: true,
    params: [
      {
        id: "exp",
        label: "经验本",
        value: "1",
        options: ["0", "1", "2", "3", "4"].map((value) => ({ value, label: `×${value}` })),
      },
      {
        id: "thread",
        label: "纽本",
        value: "3",
        options: ["0", "1", "2", "3", "4"].map((value) => ({ value, label: `×${value}` })),
      },
      {
        id: "chain",
        label: "连战",
        value: "off",
        options: [
          { value: "off", label: "关" },
          { value: "on", label: "开" },
        ],
      },
    ],
  },
  {
    id: "rewards",
    icon: "package",
    name: "领取奖励",
    enabled: true,
    params: [
      {
        id: "mode",
        label: "模式",
        value: "all",
        options: [
          { value: "all", label: "全部" },
          { value: "daily", label: "日常" },
          { value: "weekly", label: "周常" },
        ],
      },
    ],
  },
  {
    id: "enkephalin",
    icon: "sparkles",
    name: "狂气换体",
    enabled: true,
    params: [
      {
        id: "times",
        label: "换体",
        value: "2",
        options: ["1", "2", "3", "4", "5"].map((value) => ({ value, label: value })),
      },
      {
        id: "grandet",
        label: "葛朗台",
        value: "off",
        options: [
          { value: "off", label: "关" },
          { value: "on", label: "开" },
        ],
      },
    ],
  },
  {
    id: "mirror",
    icon: "target",
    name: "自动镜牢",
    enabled: true,
    params: [
      {
        id: "runs",
        label: "轮次",
        value: "1",
        options: ["1", "2", "3", "4", "5"].map((value) => ({ value, label: value })),
      },
      {
        id: "difficulty",
        label: "难度",
        value: "normal",
        options: [
          { value: "normal", label: "普通" },
          { value: "hard", label: "困难" },
        ],
      },
    ],
  },
  {
    id: "resonance",
    icon: "audioLines",
    name: "亚哈共鸣",
    enabled: true,
    params: [
      {
        id: "voice",
        label: "语录",
        value: "on",
        options: [
          { value: "on", label: "开启" },
          { value: "off", label: "关闭" },
        ],
      },
    ],
  },
]);

/** The sidecar link's state (tracked). */
export function backendState(): BackendState {
  return backend();
}

/** The device list the selector offers (tracked). */
export function deviceList(): { id: string; name: string }[] {
  return devices();
}

/** The chosen device's state (tracked). */
export function deviceState(): DeviceState {
  return device();
}

/** The window resolution shown in the window card (tracked). */
export function currentResolution(): string {
  return resolution();
}

/** Whether asynchronous input is on (tracked). */
export function isAsyncInput(): boolean {
  return asyncInput();
}

/** How many log lines the panel holds (tracked). */
export function logLines(): number {
  return logCount();
}

/** The console's tasks (tracked). */
export function consoleTasks(): Task[] {
  return tasks();
}

/** The line under the queue, when the last action left something to say. */
export function consoleFeedback(): string | null {
  return feedback();
}

/** Turns a task on or off. */
export function toggleTask(id: TaskId): void {
  tasks.update((list) =>
    list.map((task) => (task.id === id ? { ...task, enabled: !task.enabled } : task)),
  );
}

/** Sets one of a task's parameters. */
export function setTaskParam(id: TaskId, paramId: string, value: string): void {
  tasks.update((list) =>
    list.map((task) =>
      task.id === id
        ? {
            ...task,
            params: task.params.map((param) =>
              param.id === paramId ? { ...param, value } : param,
            ),
          }
        : task,
    ),
  );
}

/** Enables every task. */
export function selectAllTasks(): void {
  tasks.update((list) => list.map((task) => ({ ...task, enabled: true })));
  feedback.set("已全选任务");
}

/** Empties the queue's selection without deleting the tasks. */
export function clearTasks(): void {
  tasks.update((list) => list.map((task) => ({ ...task, enabled: false })));
  feedback.set("已清空任务");
}

/** Starts the queue. */
export function startQueue(): void {
  feedback.set("已提交启动请求（执行随 P1 的 sidecar 接入）");
}

/** Empties the run log. */
export function clearLog(): void {
  logCount.set(0);
  feedback.set("日志已清空");
}

/** Re-reads the device list. */
export function refreshDevices(): void {
  devices.set([
    { id: "mumu", name: "MuMu 模拟器" },
    { id: "scrcpy", name: "Scrcpy 窗口" },
  ]);
  feedback.set("已刷新设备列表");
}

/** Picks a device to drive. */
export function selectDevice(id: string): void {
  device.set(id === "" ? "disconnected" : "connecting");
  feedback.set("正在连接设备…");
}
