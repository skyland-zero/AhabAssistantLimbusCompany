// The console page's state: the backend link, the device, the five tasks, the
// run log and the statistics summary.
//
// Rust owns the task config, execution, devices, log events and stats; this
// module mirrors the snapshots into signals and writes through the store.
import { signal } from "nsh";
import { Backend, HomeStore } from "aalc/native";
import { allTeams, teamNumber } from "./teams-store";
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

/** One parameter on a task card: a select, or a switch for an on/off setting. */
export type TaskParam = {
  id: string;
  label: string;
  /** `switch` draws a switch and keeps `"on"` / `"off"`; `select` one of `options`. */
  kind: "select" | "switch";
  value: string;
  /** The choices; a function for one that follows the saved teams. */
  options?: { value: string; label: string }[] | (() => { value: string; label: string }[]);
  /** The section it belongs to, when a card's settings come in two (常规 / 高级). */
  group?: string;
  /** Shown in the collapsed row, the way the client's preview chips were. */
  summary?: boolean;
};

/** A select on a task card. */
function choice(
  id: string,
  label: string,
  value: string,
  options: [string, string][],
  group?: string,
  summary = true,
): TaskParam {
  return {
    id,
    label,
    kind: "select",
    value,
    options: options.map(([value, label]) => ({ value, label })),
    group,
    summary,
  };
}

/** A switch on a task card: the task config stores it as `on` / `off`. */
function toggle(
  id: string,
  label: string,
  on = false,
  group?: string,
  summary = true,
): TaskParam {
  return { id, label, kind: "switch", value: on ? "on" : "off", group, summary };
}

/** The saved teams as choices; one formation's name when the store has none. */
function dailyTeams(): { value: string; label: string }[] {
  const saved = allTeams()
    .map((team) => ({ number: teamNumber(team.id), label: team.name }))
    .filter((entry) => entry.number !== null)
    .map((entry) => ({ value: String(entry.number), label: entry.label }));
  return saved.length > 0 ? saved : [{ value: "1", label: "编队 1" }];
}

/** A select whose choices are the saved teams (their numbers, their names). */
function teamChoice(id: string, label: string, value: string, group?: string): TaskParam {
  return {
    id,
    label,
    kind: "select",
    value,
    options: () => dailyTeams(),
    group,
    summary: false,
  };
}

/** `×0` … `×4`, the run counts a card offers. */
const COUNTS: [string, string][] = ["0", "1", "2", "3", "4"].map((value) => [value, `×${value}`]);

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

/** One of the window card's settings, as the store names it. */
export type WindowField =
  | "size"
  | "position"
  | "reduceMiscontact"
  | "screenshotInterval"
  | "mouseInterval"
  | "asyncInput";

/** The window card's settings (`SetWindowsConfig`), as the card shows them. */
export type WindowSettings = {
  /** `"720"`, `"1080"` or `"1440"`: the size select's value. */
  size: string;
  /** `"1080P"`: the card's summary line. */
  resolution: string;
  /** `center`, `left_top`, `right_top` or `free`. */
  position: string;
  reduceMiscontact: boolean;
  /** Seconds, one decimal, so a select's option matches it. */
  screenshotInterval: string;
  mouseInterval: string;
  asyncInput: boolean;
};

/** The console's "when the queue finishes" actions (`AfterCompletionConfig`). */
export type AfterCompletion = {
  /** `exit_game`, `exit_emulator`, `exit_aalc`, in the order they run. */
  actions: string[];
  /** `none`, `sleep`, `hibernate`, `lock` or `shutdown`. */
  power: string;
  /** Saved as the default, rather than chosen for this run only. */
  keep: boolean;
};

/** The exit actions the queue can run through, in the order they show. */
export const AFTER_EXIT_ACTIONS: { value: string; label: string }[] = [
  { value: "exit_game", label: "退出游戏" },
  { value: "exit_emulator", label: "退出模拟器" },
  { value: "exit_aalc", label: "退出 AALC.nsh" },
];

/** The power action that may follow them. */
export const AFTER_POWER_ACTIONS: { value: string; label: string }[] = [
  { value: "none", label: "无动作" },
  { value: "sleep", label: "睡眠" },
  { value: "hibernate", label: "休眠" },
  { value: "lock", label: "锁屏" },
  { value: "shutdown", label: "关机" },
];

/** One day of the daily statistics. */
export type DailyStatEntry = {
  date: string;
  exp: number;
  thread: number;
  mirror: number;
  total: number;
};

/** The daily statistics dialog payload. */
export type DailyStats = {
  dateFrom: string;
  dateTo: string;
  days: DailyStatEntry[];
  updatedAt: number;
};

/** The team a mirror run was fought with, as the sidecar snapshots it. */
export type MirrorTeamEntry = {
  id?: string;
  name?: string;
  number?: number;
  sinners?: string[];
  sinnerNames?: string[];
  sinnerNamesEn?: string[];
  system?: string;
  accessoryScheme?: string;
};

/** One completed (or failed) mirror run, as the sidecar records and persists
 *  it (the statistics page's only source). */
export type MirrorHistoryEntry = {
  completedAt: string;
  runId?: string | null;
  totalSeconds: number;
  battleSeconds?: number;
  eventSeconds?: number;
  shopSeconds?: number;
  findRoadSeconds?: number;
  themePackSeconds?: number;
  rewardCardSeconds?: number;
  egoGiftSeconds?: number;
  settlementSeconds?: number;
  otherSeconds?: number;
  eventCount?: number;
  hardMode?: boolean;
  mode?: string;
  failed?: boolean;
  failureReason?: string | null;
  team?: MirrorTeamEntry | null;
  floorCount?: number;
  routeId?: string;
  routeName?: string;
  routeNameEn?: string;
};

const DEFAULT_TASKS: Task[] = [
  {
    id: "daily",
    icon: "listChecks",
    name: "日常任务",
    enabled: true,
    params: [
      choice("exp", "经验本", "1", COUNTS, "常规"),
      choice("thread", "纽本", "3", COUNTS, "常规"),
      teamChoice("team", "默认日常编队", "1", "常规"),
      toggle("chain", "连战", false, "常规"),
      choice(
        "chainCount",
        "连战次数",
        "1",
        ["1", "2", "3", "4", "5", "6", "7", "8", "9", "10"].map((value) => [value, value]),
        "常规",
        false,
      ),
      toggle("targetedExp", "经验本按属性配队", false, "高级", false),
      toggle("targetedThread", "纽本按星期配队", false, "高级", false),
      teamChoice("expSlash", "经验斩击（周一/二）", "1", "高级"),
      teamChoice("expPierce", "经验突刺（周三/四）", "1", "高级"),
      teamChoice("expBlunt", "经验打击（周五/六）", "1", "高级"),
      teamChoice("expAll", "经验全属性（周日）", "1", "高级"),
      teamChoice("threadMon", "纽本周一", "1", "高级"),
      teamChoice("threadTue", "纽本周二", "1", "高级"),
      teamChoice("threadWed", "纽本周三", "1", "高级"),
      teamChoice("threadThu", "纽本周四", "1", "高级"),
      teamChoice("threadFri", "纽本周五", "1", "高级"),
      teamChoice("threadSat", "纽本周六", "1", "高级"),
      teamChoice("threadSun", "纽本周日", "1", "高级"),
    ],
  },
  {
    id: "rewards",
    icon: "package",
    name: "领取奖励",
    enabled: true,
    params: [
      choice("mode", "领取模式", "all", [
        ["all", "全部"],
        ["lunacy", "狂气与通行证奖励"],
        ["mail", "仅领取邮件奖励"],
      ]),
    ],
  },
  {
    id: "enkephalin",
    icon: "sparkles",
    name: "狂气换体",
    enabled: true,
    params: [
      choice(
        "times",
        "换体次数",
        "2",
        ["0", "1", "2", "3", "4", "5", "6", "7", "8", "9", "10"].map((value) => [value, value]),
      ),
      toggle("grandet", "葛朗台模式"),
      toggle("skip", "跳过模块合成", false, undefined, false),
    ],
  },
  {
    id: "mirror",
    icon: "target",
    name: "自动镜牢",
    enabled: true,
    params: [
      choice("runs", "轮次", "1", ["1", "2", "3", "4", "5"].map((value) => [value, value]), "常规"),
      choice(
        "difficulty",
        "难度",
        "normal",
        [
          ["normal", "普通"],
          ["hard", "困难"],
        ],
        "常规",
      ),
      choice(
        "hardTarget",
        "困牢目标",
        "5",
        [
          ["5", "5 层速刷"],
          ["15", "15 层叠加"],
        ],
        "常规",
      ),
      toggle("infinite", "无限模式", false, "常规"),
      toggle("noWeekly", "不使用每周加成", false, "高级", false),
      toggle("floorThree", "只打三层", false, "高级", false),
      toggle("saveRewards", "保存困牢奖励", false, "高级", false),
      toggle("singleBonus", "困牢单次加成", false, "高级", false),
      toggle("selectEvent", "第 5 层选活动包", false, "高级", false),
      toggle("skipEvent", "第 5 层跳过活动包", false, "高级", false),
      toggle("reclaim", "再次领取奖励", false, "高级", false),
      toggle("cotton", "不跳过白棉花", false, "高级", false),
      toggle("lastMan", "战斗直到全灭", false, "高级", false),
      toggle("keyboard", "键盘寻路", false, "高级", false),
      toggle("simpleKeys", "简易键盘寻路", false, "高级", false),
      toggle("hatred", "F3/F4 优先 Hatred and Despair", false, "高级", false),
      toggle("minimize", "寻路优先最少非 Boss 战斗", false, "高级", false),
    ],
  },
  {
    id: "resonance",
    icon: "audioLines",
    name: "亚哈共鸣",
    enabled: true,
    params: [],
  },
];

const backend = signal<BackendState>(Backend.mode() === "mock" ? "mock" : "starting");
const devices = signal<{ id: string; name: string }[]>([]);
const device = signal<DeviceState>("disconnected");
const selectedDevice = signal<string | null>(null);
const window_ = signal<WindowSettings>({
  size: "1080",
  resolution: "1080P",
  position: "free",
  reduceMiscontact: true,
  screenshotInterval: "0.5",
  mouseInterval: "0.3",
  asyncInput: false,
});
const afterCompletion = signal<AfterCompletion>({ actions: [], power: "none", keep: false });
const afterCompletionOpen = signal(false);
const logs = signal<LogEntry[]>([]);
const feedback = signal<string | null>(null);
const tasks = signal<Task[]>(DEFAULT_TASKS.map((task) => ({ ...task })));
const previewFrame = signal<string | null>(null);
// The last frame stays underneath the next one while the new image decodes,
// so a frame swap does not blank the card.
const previewPreviousFrame = signal<string | null>(null);
const previewStatus = signal("stopped");
const dailyStats = signal<DailyStats | null>(null);
const statsDetailsError = signal<string | null>(null);
const logDetailsOpen = signal(false);

let store: HomeStore | null = null;
let connecting = false;

function selectionFrom(list: Task[]): Record<string, Record<string, unknown>> {
  return Object.fromEntries(
    list.map((task) => [
      task.id,
      {
        enabled: task.enabled,
        ...Object.fromEntries(task.params.map((param) => [param.id, param.value])),
      },
    ]),
  );
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
  syncDevice(created);
  const window = created.window();
  applyWindow(window);
  applyAfterCompletion(created.afterCompletion());
  logs.set(created.logs());
  mirrorHomeStatus(created.executionState());
  backend.set(Backend.mode() === "mock" ? "mock" : "ready");
}

/** Mirrors the device selection the state layer owns (connect events included).
 *  While a connect is in flight the picker keeps the choice it was given: Rust
 *  only records the selected id once the RPC lands. */
function syncDevice(current: HomeStore): void {
  const state = current.deviceState() as DeviceState;
  device.set(state);
  if (state !== "connecting") selectedDevice.set(current.selectedDeviceId());
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

/** The chosen device's id, or `null` while none is selected (tracked). */
export function selectedDeviceId(): string | null {
  ensureStore();
  return selectedDevice();
}

/** The window card's settings (tracked). */
export function currentWindow(): WindowSettings {
  ensureStore();
  return window_();
}

/** Applies one window setting through the task config. */
export function setWindowSetting(field: WindowField, value: string | boolean): void {
  const current = ensureStore();
  if (!current) return;
  try {
    applyWindow(current.setWindow(field, value));
    feedback.set("窗口设置已保存");
  } catch (error) {
    feedback.set(error instanceof Error ? error.message : String(error));
  }
}

function applyWindow(view: ReturnType<HomeStore["window"]>): void {
  window_.set({
    size: view.size,
    resolution: view.resolution,
    position: view.position,
    reduceMiscontact: view.reduceMiscontact,
    screenshotInterval: view.screenshotInterval,
    mouseInterval: view.mouseInterval,
    asyncInput: view.asyncInput,
  });
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

/** The console's last-action line, shown in the run log card, not under the
 *  queue. */
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

/** Re-reads the device list without blocking the window. */
export async function refreshDevices(): Promise<void> {
  const current = ensureStore();
  if (!current) return;
  try {
    const list = await current.refreshDevices();
    devices.set(list.map((entry) => ({ id: entry.id, name: entry.name })));
    syncDevice(current);
    feedback.set(current.feedback());
  } catch (error) {
    feedback.set(error instanceof Error ? error.message : String(error));
  }
}

/** Picks a device to drive without blocking the window. */
export async function selectDevice(id: string): Promise<void> {
  const current = ensureStore();
  if (!current) return;
  // The popup closes before the RPC returns: state the choice at once, then
  // reconcile it with the id the state layer reports when the call lands.
  selectedDevice.set(id || null);
  try {
    await current.connectDevice(id);
    syncDevice(current);
    feedback.set(current.feedback());
    if (device() === "connected") current.setPreviewEnabled(true);
  } catch (error) {
    syncDevice(current);
    feedback.set(error instanceof Error ? error.message : String(error));
  }
}

/** The latest preview frame as a `data:` URL, or `null` before the first. */
export function currentPreviewFrame(): string | null {
  ensureStore();
  return previewFrame();
}

/** The frame before the latest one; shown while the latest decodes. */
export function currentPreviewPlaceholder(): string | null {
  ensureStore();
  return previewPreviousFrame();
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
  if (view.frame) {
    const previous = previewFrame();
    if (previous) previewPreviousFrame.set(previous);
    previewFrame.set(view.frame);
  }
  // Device events (a connection made or lost elsewhere) arrive with the same
  // drain, so the picker and the badge follow the state layer as well.
  syncDevice(current);
  logs.set(current.logs());
  // The console card is a plain read of the state layer; this signal is what
  // makes it re-render when the run moves on.
  runProgress.set(current.runProgress() as RunProgress);
}

/** The statistics summary shown on the console (tracked). */
export function consoleStats(): {
  today: { exp: number; thread: number; mirror: number };
  week: { exp: number; thread: number; mirror: number };
  lastMirror: string | null;
  /** The last mirror run's time and floors, for the card's "last time" note. */
  lastMirrorSeconds: number | null;
  lastMirrorFloors: number | null;
} {
  const current = ensureStore();
  if (!current) {
    return {
      today: { exp: 0, thread: 0, mirror: 0 },
      week: { exp: 0, thread: 0, mirror: 0 },
      lastMirror: null,
      lastMirrorSeconds: null,
      lastMirrorFloors: null,
    };
  }
  const stats = current.stats();
  return {
    today: { exp: stats.todayExp, thread: stats.todayThread, mirror: stats.todayMirror },
    week: { exp: stats.weekExp, thread: stats.weekThread, mirror: stats.weekMirror },
    lastMirror: stats.lastMirror ?? null,
    lastMirrorSeconds: stats.lastMirrorSeconds ?? null,
    lastMirrorFloors: stats.lastMirrorFloors ?? null,
  };
}

/** The current run as the console card shows it. */
export type RunProgress = {
  /** `idle`, `starting`, `running`, `paused`, `stopping` or `restoring`. */
  state: string;
  /** `daily_task`, `get_reward`, `buy_enkephalin`, `mirror`, ... or null. */
  task: string | null;
  /** A task runs or is paused: the card shows this run, not the totals. */
  live: boolean;
  expDone: number;
  expTarget: number;
  threadDone: number;
  threadTarget: number;
  mirrorDone: number;
  mirrorTarget: number;
  infinite: boolean;
  hard: boolean;
  floor: number | null;
  floorTotal: number | null;
  elapsedMs: number | null;
};

const IDLE_PROGRESS: RunProgress = {
  state: "idle",
  task: null,
  live: false,
  expDone: 0,
  expTarget: 0,
  threadDone: 0,
  threadTarget: 0,
  mirrorDone: 0,
  mirrorTarget: 0,
  infinite: false,
  hard: false,
  floor: null,
  floorTotal: null,
  elapsedMs: null,
};

/** The run the console shows; `pollPreview` refreshes it with every event
 *  drain, so the top card follows the queue instead of freezing at mount. */
const runProgress = signal<RunProgress>(IDLE_PROGRESS);

/** The current run's counts, floor and clock (tracked by the console's poll). */
export function consoleProgress(): RunProgress {
  ensureStore();
  return runProgress();
}

/** The connected device's name, or `null` while nothing is connected. */
export function selectedDeviceName(): string | null {
  const current = ensureStore();
  return current?.selectedDeviceName() ?? null;
}

/** The daily statistics loaded for the statistics page (tracked). */
export function currentDailyStats(): DailyStats | null {
  return dailyStats();
}

/** The daily statistics load error, when there is one (tracked). */
export function currentStatsDetailsError(): string | null {
  return statsDetailsError();
}

/** Loads the daily counts for the statistics page. */
export function loadDailyStats(): void {
  statsDetailsError.set(null);
  const current = ensureStore();
  if (!current) return;
  try {
    dailyStats.set(current.dailyStats() as DailyStats);
  } catch (error) {
    statsDetailsError.set(error instanceof Error ? error.message : String(error));
  }
}

/** The mirror history after draining the event queue: the statistics page's
 *  poll, so a run that just finished shows without leaving the page. */
export function pollMirrorHistory(): MirrorHistoryEntry[] {
  const current = ensureStore();
  if (!current) return [];
  const payload = current.pollStats() as { mirrorHistory?: MirrorHistoryEntry[] };
  return payload.mirrorHistory ?? [];
}

/** A mirror record's completion time as `MM-DD HH:mm`, or the raw stamp when it
 *  cannot be read: the console's strip and the statistics page agree on it. */
export function mirrorTimeLabel(completedAt: string): string {
  const date = new Date(completedAt);
  if (Number.isNaN(date.getTime())) return completedAt;
  const month = String(date.getMonth() + 1).padStart(2, "0");
  const day = String(date.getDate()).padStart(2, "0");
  const hours = String(date.getHours()).padStart(2, "0");
  const minutes = String(date.getMinutes()).padStart(2, "0");
  return `${month}-${day} ${hours}:${minutes}`;
}

/** Whether the full log dialog is open (tracked). */
export function isLogDetailsOpen(): boolean {
  return logDetailsOpen();
}

/** The after-run actions (tracked). */
export function afterCompletionSettings(): AfterCompletion {
  ensureStore();
  return afterCompletion();
}

/** Whether the after-run actions editor is open (tracked). */
export function isAfterCompletionOpen(): boolean {
  return afterCompletionOpen();
}

/** Opens the after-run actions editor. */
export function openAfterCompletion(): void {
  afterCompletionOpen.set(true);
}

/** Closes the after-run actions editor without saving. */
export function closeAfterCompletion(): void {
  afterCompletionOpen.set(false);
}

/** Saves the after-run actions: as the default (`keep`), else for this run. */
export function saveAfterCompletion(next: AfterCompletion): void {
  const current = ensureStore();
  if (!current) return;
  try {
    applyAfterCompletion(current.setAfterCompletion(next.actions, next.power, next.keep));
    feedback.set("结束后操作已保存");
    afterCompletionOpen.set(false);
  } catch (error) {
    feedback.set(error instanceof Error ? error.message : String(error));
  }
}

/** The run bar's summary: `退出游戏/AALC.nsh · 关机`, or `什么也不干`. It stays
 *  short (the actions' names drop their "退出", which the summary says once),
 *  because the run bar also holds the start button. */
export function afterCompletionLabel(config: AfterCompletion): string {
  const exits = config.actions
    .map((action) => AFTER_EXIT_ACTIONS.find((entry) => entry.value === action)?.label ?? action)
    .map((label) => label.replace(/^退出\s*/, ""))
    .join("/");
  const power =
    config.power === "none"
      ? ""
      : (AFTER_POWER_ACTIONS.find((entry) => entry.value === config.power)?.label ?? "");
  const body = [exits === "" ? "" : `退出${exits}`, power].filter((part) => part !== "").join(" · ");
  return body === "" ? "什么也不干" : body;
}

/** Whether the actions are the saved default (`默认`) or this run's (`本次`). */
export function afterCompletionMode(config: AfterCompletion): string {
  return config.keep ? "默认" : "本次";
}

function applyAfterCompletion(view: ReturnType<HomeStore["afterCompletion"]>): void {
  afterCompletion.set({ actions: view.actions, power: view.power, keep: view.keep });
}

/** Opens the full log dialog. */
export function openLogDetails(): void {
  logDetailsOpen.set(true);
}

/** Closes the full log dialog. */
export function closeLogDetails(): void {
  logDetailsOpen.set(false);
}
