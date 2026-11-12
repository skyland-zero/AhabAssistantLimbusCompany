// The console page's state: the backend link, the device, the five tasks, the
// run log and the statistics summary.
//
// Rust owns the task config, execution, devices, log events and stats; this
// module mirrors the snapshots into signals and writes through the store.
import { signal } from "nsh";
import { Backend, HomeStore } from "aalc/native";
import { mirrorHomeStatus } from "./store";

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

/** One log line. */
export type LogEntry = { level: string; message: string; ts: number };

const DEFAULT_TASKS: Task[] = [
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
];

const backend = signal<BackendState>(Backend.mode() === "mock" ? "mock" : "starting");
const devices = signal<{ id: string; name: string }[]>([]);
const device = signal<DeviceState>("disconnected");
const resolution = signal("1080P");
const asyncInput = signal(false);
const logs = signal<LogEntry[]>([]);
const feedback = signal<string | null>(null);
const tasks = signal<Task[]>(DEFAULT_TASKS.map((task) => ({ ...task })));
const previewFrame = signal<string | null>(null);
const previewStatus = signal("stopped");

let store: HomeStore | null = null;
let connecting = false;

function selectionFrom(list: Task[]): Record<string, Record<string, unknown>> {
  const byId = Object.fromEntries(list.map((task) => [task.id, task]));
  const value = (id: TaskId, param: string) =>
    byId[id]?.params.find((entry) => entry.id === param)?.value ?? "";
  return {
    daily: {
      enabled: byId.daily?.enabled ?? false,
      exp: value("daily", "exp"),
      thread: value("daily", "thread"),
      chain: value("daily", "chain"),
    },
    rewards: {
      enabled: byId.rewards?.enabled ?? false,
      mode: value("rewards", "mode"),
    },
    enkephalin: {
      enabled: byId.enkephalin?.enabled ?? false,
      times: value("enkephalin", "times"),
      grandet: value("enkephalin", "grandet"),
    },
    mirror: {
      enabled: byId.mirror?.enabled ?? false,
      runs: value("mirror", "runs"),
      difficulty: value("mirror", "difficulty"),
    },
    resonance: {
      enabled: byId.resonance?.enabled ?? false,
      voice: value("resonance", "voice"),
    },
  };
}

function hydrateSelection(selection: Record<string, Record<string, unknown>>): void {
  tasks.update((list) =>
    list.map((task) => {
      const node = selection[task.id];
      if (!node) return task;
      return {
        ...task,
        enabled: Boolean(node.enabled),
        params: task.params.map((param) => ({
          ...param,
          value: String(node[param.id] ?? param.value),
        })),
      };
    }),
  );
}

function syncTasks(): void {
  if (!store) return;
  try {
    store.configure(selectionFrom(tasks()));
  } catch (error) {
    feedback.set(error instanceof Error ? error.message : String(error));
  }
}

function hydrateFrom(created: HomeStore): void {
  store = created;
  hydrateSelection(created.selection() as Record<string, Record<string, unknown>>);
  devices.set(created.devices().map((entry) => ({ id: entry.id, name: entry.name })));
  device.set(created.deviceState() as DeviceState);
  const window = created.window();
  resolution.set(window.resolution);
  asyncInput.set(window.asyncInput);
  logs.set(created.logs());
  mirrorHomeStatus(created.executionState());
  backend.set(Backend.mode() === "mock" ? "mock" : "ready");
}

function ensureStore(): HomeStore | null {
  if (store) return store;
  if (Backend.mode() === "mock") {
    try {
      hydrateFrom(new HomeStore());
    } catch (error) {
      backend.set("failed");
      feedback.set(error instanceof Error ? error.message : String(error));
    }
  } else if (!connecting) {
    connecting = true;
    HomeStore.connect()
      .then(hydrateFrom)
      .catch((error) => {
        backend.set("failed");
        feedback.set(error instanceof Error ? error.message : String(error));
      })
      .finally(() => {
        connecting = false;
      });
  }
  return store;
}

if (Backend.mode() === "mock") ensureStore();

/** The sidecar link's state (tracked). */
export function backendState(): BackendState {
  return backend();
}

/** The device list the selector offers (tracked). */
export function deviceList(): { id: string; name: string }[] {
  ensureStore();
  return devices();
}

/** The chosen device's state (tracked). */
export function deviceState(): DeviceState {
  return device();
}

/** The window resolution shown in the window card (tracked). */
export function currentResolution(): string {
  ensureStore();
  return resolution();
}

/** Whether asynchronous input is on (tracked). */
export function isAsyncInput(): boolean {
  ensureStore();
  return asyncInput();
}

/** Cycles 1080P ↔ 1440P through the task config. */
export function cycleResolution(): void {
  const current = ensureStore();
  if (!current) return;
  try {
    const next = resolution() === "1080P" ? 1440 : 1080;
    const window = current.setWindowSize(next);
    resolution.set(window.resolution);
    asyncInput.set(window.asyncInput);
    feedback.set("窗口设置已保存");
  } catch (error) {
    feedback.set(error instanceof Error ? error.message : String(error));
  }
}

/** Turns asynchronous input on or off through the task config. */
export function toggleAsyncInput(): void {
  const current = ensureStore();
  if (!current) return;
  try {
    const window = current.toggleAsyncInput();
    resolution.set(window.resolution);
    asyncInput.set(window.asyncInput);
    feedback.set("窗口设置已保存");
  } catch (error) {
    feedback.set(error instanceof Error ? error.message : String(error));
  }
}

/** How many log lines the panel holds (tracked). */
export function logLines(): number {
  ensureStore();
  return logs().length;
}

/** The log lines, oldest first (tracked). */
export function logEntries(): LogEntry[] {
  ensureStore();
  return logs();
}

/** The console's tasks (tracked). */
export function consoleTasks(): Task[] {
  ensureStore();
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
  syncTasks();
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
  syncTasks();
}

/** Enables every task. */
export function selectAllTasks(): void {
  tasks.update((list) => list.map((task) => ({ ...task, enabled: true })));
  syncTasks();
  feedback.set("已全选任务");
}

/** Empties the queue's selection without deleting the tasks. */
export function clearTasks(): void {
  tasks.update((list) => list.map((task) => ({ ...task, enabled: false })));
  syncTasks();
  feedback.set("已清空任务");
}

/** Starts the queue. */
export function startQueue(): void {
  const current = ensureStore();
  if (!current) return;
  syncTasks();
  try {
    current.start();
    mirrorHomeStatus(current.executionState());
    logs.set(current.logs());
    feedback.set(current.feedback() ?? "已提交启动请求");
  } catch (error) {
    feedback.set(error instanceof Error ? error.message : String(error));
  }
}

/** Pauses a running queue or resumes a paused one. */
export function pauseOrResumeQueue(): void {
  const current = ensureStore();
  if (!current) return;
  try {
    current.pauseOrResume();
    mirrorHomeStatus(current.executionState());
    logs.set(current.logs());
    feedback.set(current.feedback());
  } catch (error) {
    feedback.set(error instanceof Error ? error.message : String(error));
  }
}

/** Stops the running queue. */
export function stopQueue(): void {
  const current = ensureStore();
  if (!current) return;
  try {
    current.stop();
    mirrorHomeStatus(current.executionState());
    logs.set(current.logs());
    feedback.set(current.feedback());
  } catch (error) {
    feedback.set(error instanceof Error ? error.message : String(error));
  }
}

/** Empties the run log. */
export function clearLog(): void {
  const current = ensureStore();
  if (!current) return;
  current.clearLogs();
  logs.set(current.logs());
  feedback.set(current.feedback());
}

/** Re-reads the device list. */
export function refreshDevices(): void {
  const current = ensureStore();
  if (!current) return;
  try {
    devices.set(current.refreshDevices().map((entry) => ({ id: entry.id, name: entry.name })));
    feedback.set(current.feedback());
  } catch (error) {
    feedback.set(error instanceof Error ? error.message : String(error));
  }
}

/** Picks a device to drive. */
export function selectDevice(id: string): void {
  const current = ensureStore();
  if (!current) return;
  try {
    device.set(current.connectDevice(id) as DeviceState);
    feedback.set(current.feedback());
    if (device() === "connected") current.setPreviewEnabled(true);
  } catch (error) {
    feedback.set(error instanceof Error ? error.message : String(error));
  }
}

/** The latest preview frame as a `data:` URL, or `null` before the first. */
export function currentPreviewFrame(): string | null {
  ensureStore();
  return previewFrame();
}

/** The preview stream's status (`stopped`, `starting`, `running`, `error`). */
export function currentPreviewStatus(): string {
  ensureStore();
  return previewStatus();
}

/** Drains queued events and updates the live preview and log mirror. */
export function pollPreview(): void {
  const current = ensureStore();
  if (!current) return;
  const view = current.pollPreview();
  previewStatus.set(view.status);
  if (view.frame) previewFrame.set(view.frame);
  logs.set(current.logs());
}

/** The statistics summary shown on the console (tracked). */
export function consoleStats(): {
  today: { exp: number; thread: number; mirror: number };
  week: { exp: number; thread: number; mirror: number };
  lastMirror: string | null;
} {
  const current = ensureStore();
  if (!current) {
    return {
      today: { exp: 0, thread: 0, mirror: 0 },
      week: { exp: 0, thread: 0, mirror: 0 },
      lastMirror: null,
    };
  }
  const stats = current.stats();
  return {
    today: { exp: stats.todayExp, thread: stats.todayThread, mirror: stats.todayMirror },
    week: { exp: stats.weekExp, thread: stats.weekThread, mirror: stats.weekMirror },
    lastMirror: stats.lastMirror ?? null,
  };
}
