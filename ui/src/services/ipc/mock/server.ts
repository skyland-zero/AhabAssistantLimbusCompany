import type { RpcClient } from "../client";
import type {
  HotkeyConfig,
  IpcEventName,
  NoticeItem,
  QueueOptions,
  QueueTask,
  ResourceGroup,
  RpcError,
  RpcRequest,
  RpcResponse,
  SinnerInfo,
  TaskProgressPayload,
  TaskStatusPayload,
  TeamDetail,
  ThemePack,
  ThemePackState,
  ToolId,
  ToolStatusPayload,
} from "../types";

type Handler = (params: unknown) => unknown;

/**
 * 内存版 mock 后端。
 * 提供与真实 WebSocket 后端一致的请求/事件语义，
 * 让 UI 在 M0-M5 阶段完全独立开发（不依赖任何后端）。
 */
export function createMockClient(): RpcClient {
  let state: "disconnected" | "connecting" | "connected" = "disconnected";
  let nextId = 1;
  let heartbeat: ReturnType<typeof setInterval> | null = null;
  let syncTimer: ReturnType<typeof setInterval> | null = null;

  /* ------------------------------ 假数据 ------------------------------ */

  const SINNERS: SinnerInfo[] = [
    { id: "yi_sang", name: "以升" },
    { id: "faust", name: "浮士德" },
    { id: "don_quixote", name: "堂吉诃德" },
    { id: "ryoshu", name: "良秀" },
    { id: "meursault", name: "默尔索" },
    { id: "hong_lu", name: "鸿路" },
    { id: "heathcliff", name: "希斯克利夫" },
    { id: "ishmael", name: "伊什梅尔" },
    { id: "rodion", name: "罗佳" },
    { id: "sinclair", name: "辛克莱" },
    { id: "outis", name: "奥提斯" },
    { id: "gregor", name: "格雷戈尔" },
  ];

  let teams: TeamDetail[] = [
    {
      id: "team-1",
      name: "一队",
      purpose: "mirror",
      sinners: ["faust", "ishmael", "ryoshu", "hong_lu"],
      accessoryScheme: "tremor",
      enabled: true,
    },
    {
      id: "team-2",
      name: "二队",
      purpose: "luxcavation",
      sinners: ["heathcliff", "rodion", "gregor"],
      accessoryScheme: "burn",
      enabled: true,
    },
    {
      id: "team-3",
      name: "三队",
      purpose: "general",
      sinners: ["yi_sang", "don_quixote", "meursault", "sinclair", "outis"],
      accessoryScheme: "poise",
      enabled: false,
    },
  ];

  const tasks: QueueTask[] = [
    {
      id: "t-1",
      kind: "mirror",
      name: "镜牢·自动挂机",
      teamId: "team-1",
      repeat: null,
      status: "idle",
      progress: 0,
      detail: "使用队伍：一队",
    },
    {
      id: "t-2",
      kind: "luxcavation",
      name: "经验本·Luxcavation",
      teamId: "team-2",
      repeat: 5,
      status: "idle",
      progress: 0,
      detail: "次数 ×5",
    },
    {
      id: "t-3",
      kind: "prizes",
      name: "每日领奖",
      status: "idle",
      repeat: 1,
      progress: 0,
    },
  ];

  /** 工具运行状态（内存即可） */
  const toolRunning = new Map<ToolId, boolean>();

  const DEFAULT_PACKS: ThemePack[] = [
    { id: "pk-1", name: "黑云会", weight: 5, enabled: true, tier: "T1" },
    { id: "pk-2", name: "拇指", weight: 3, enabled: true, tier: "T2" },
    { id: "pk-3", name: "利刃兄弟会", weight: 4, enabled: true, tier: "T2" },
    { id: "pk-4", name: "厄伍商会", weight: 2, enabled: false, tier: "T3" },
    { id: "pk-5", name: "二十区福利机构", weight: 6, enabled: true, tier: "T1" },
    { id: "pk-6", name: "技术科学解放者联盟", weight: 1, enabled: false, tier: "T4" },
    { id: "pk-7", name: "公司总部", weight: 3, enabled: true, tier: "T3" },
    { id: "pk-8", name: "残响乐团", weight: 7, enabled: true, tier: "T1" },
  ];
  const themePackState: ThemePackState = {
    hardMirrorActive: true,
    packs: structuredClone(DEFAULT_PACKS),
  };

  let resources: ResourceGroup[] = [
    {
      id: "templates",
      name: "模板资源",
      localVersion: "v2025.06.1",
      remoteVersion: null,
      lastSyncAt: Date.now() - 86_400_000,
    },
    {
      id: "models",
      name: "ONNX 模型",
      localVersion: "v1.2.0",
      remoteVersion: null,
      lastSyncAt: null,
    },
  ];

  const notices: NoticeItem[] = [
    {
      id: "n-1",
      title: "Mock 阶段公告：界面先行，后端未接入",
      date: "2025-06-01",
      level: "info",
      content:
        "**当前处于 M0-M5 界面阶段**\n\n所有数据均来自内存 mock 后端，与真实 Python sidecar 的对接将在 M6 完成。\n\n- 任务队列 / 队伍 / 工具箱均可交互\n- 配置不会写入 config.yaml\n- 遇到渲染问题请提交 issue 并附上日志页截图",
    },
    {
      id: "n-2",
      title: "困难镜牢周期进行中",
      date: "2025-05-28",
      level: "warn",
      content:
        "本周期为**困难镜牢**，建议：\n\n1. 在「主题包」页面提高高星主题包权重\n2. 将困牢次数设置为 3\n3. 优先使用带饰品体系的队伍",
    },
  ];

  const queueOptions: QueueOptions = { afterCompletion: "none", customCommand: "" };
  const hotkey: HotkeyConfig = { startStop: "", enabled: false };

  /* ------------------------------ 内部工具 ------------------------------ */

  const runTimers = new Map<string, ReturnType<typeof setInterval>>();
  const listeners = new Map<IpcEventName, Set<(payload: unknown) => void>>();

  const emit = (event: IpcEventName, payload: unknown) => {
    const set = listeners.get(event);
    if (!set) return;
    for (const handler of set) {
      handler(payload);
    }
  };

  const log = (level: "info" | "warn" | "error", message: string) => {
    emit("log.entry", { ts: Date.now(), level, message });
  };

  const findTask = (params: unknown): QueueTask => {
    const id = (params as { id?: string } | undefined)?.id;
    const task = tasks.find((t) => t.id === id);
    if (!task) throw new Error(`task not found: ${String(id)}`);
    return task;
  };

  const setStatus = (task: QueueTask, status: QueueTask["status"]) => {
    task.status = status;
    emit("task.status", { taskId: task.id, status } satisfies TaskStatusPayload);
  };

  const stopSimulation = (task: QueueTask) => {
    const timer = runTimers.get(task.id);
    if (!timer) return;
    clearInterval(timer);
    runTimers.delete(task.id);
  };

  const setToolRunning = (toolId: ToolId, running: boolean) => {
    toolRunning.set(toolId, running);
    emit("tool.status", { toolId, running } satisfies ToolStatusPayload);
  };

  /* ------------------------------ Handlers ------------------------------ */

  const handlers: Record<string, Handler> = {
    "app.ping": () => "pong",
    "app.version": () => ({ ui: __APP_VERSION__, backend: null }), // backend 未接入
    "app.checkUpdate": (): { updateAvailable: boolean; latest: string } => ({
      updateAvailable: false,
      latest: __APP_VERSION__,
    }),

    /* 任务队列 */
    "task.list": () => structuredClone(tasks),
    "task.start": (params) => {
      const task = findTask(params);
      stopSimulation(task);
      task.progress = 0;
      setStatus(task, "running");
      log("info", `[mock] 启动任务：${task.name}`);
      const timer = setInterval(() => {
        task.progress = Math.min(100, task.progress + 4 + Math.round(Math.random() * 8));
        emit("task.progress", {
          taskId: task.id,
          progress: task.progress,
        } satisfies TaskProgressPayload);
        if (task.progress >= 100) {
          stopSimulation(task);
          setStatus(task, "done");
          log("info", `[mock] 任务完成：${task.name}`);
        }
      }, 600);
      runTimers.set(task.id, timer);
      return true;
    },
    "task.stop": (params) => {
      const task = findTask(params);
      stopSimulation(task);
      task.progress = 0;
      setStatus(task, "idle");
      log("info", `[mock] 停止任务：${task.name}`);
      return true;
    },

    /* 队伍管理 */
    "team.list": () => structuredClone(teams),
    "team.save": (params) => {
      const team = params as TeamDetail;
      if (!team.name?.trim()) throw new Error("team name required");
      const idx = teams.findIndex((t) => t.id === team.id);
      if (idx >= 0) {
        teams[idx] = structuredClone(team);
      } else {
        teams.push(structuredClone({ ...team, id: `team-${nextId++}` }));
      }
      return true;
    },
    "team.delete": (params) => {
      const id = (params as { id?: string })?.id;
      teams = teams.filter((t) => t.id !== id);
      return true;
    },
    "sinner.list": () => structuredClone(SINNERS),

    /* 工具箱 */
    "tool.start": (params) => {
      const toolId = (params as { id?: ToolId }).id;
      if (!toolId) throw new Error("tool id required");
      setToolRunning(toolId, true);
      log("info", `[mock] 启动工具：${toolId}`);
      return true;
    },
    "tool.stop": (params) => {
      const toolId = (params as { id?: ToolId }).id;
      if (!toolId) throw new Error("tool id required");
      setToolRunning(toolId, false);
      log("info", `[mock] 停止工具：${toolId}`);
      return true;
    },
    "tool.screenshot": () => {
      log("info", "[mock] 截图完成：AALC/screenshot_mock.png");
      return true;
    },

    /* 镜牢主题包 */
    "themePack.list": (): ThemePackState => structuredClone(themePackState),
    "themePack.updateAll": (params) => {
      const packs = (params as { packs?: ThemePack[] })?.packs;
      if (!Array.isArray(packs)) throw new Error("packs required");
      themePackState.packs = structuredClone(packs);
      return true;
    },
    "themePack.resetWeights": () => {
      themePackState.packs = structuredClone(DEFAULT_PACKS);
      return structuredClone(themePackState);
    },

    /* 资源同步 */
    "resource.status": () => structuredClone(resources),
    "resource.checkUpdate": () => {
      // mock：远端版本号 +1 位小版本
      resources = resources.map((g) => ({
        ...g,
        remoteVersion: g.localVersion.replace(/(\d+)$/, (m) => String(Number(m) + 1)),
      }));
      return structuredClone(resources);
    },
    "resource.sync.start": () => {
      if (syncTimer) return true; // 已在同步中
      let progress = 0;
      syncTimer = setInterval(() => {
        progress = Math.min(100, progress + 6 + Math.round(Math.random() * 6));
        emit("resource.sync.progress", { scope: "all", progress });
        if (progress >= 100) {
          if (syncTimer) {
            clearInterval(syncTimer);
            syncTimer = null;
          }
          resources = resources.map((g) =>
            g.remoteVersion
              ? { ...g, localVersion: g.remoteVersion, remoteVersion: null, lastSyncAt: Date.now() }
              : g,
          );
          log("info", "[mock] 资源同步完成");
        }
      }, 400);
      return true;
    },

    /* 公告板 */
    "notice.list": () => structuredClone(notices),

    /* 队列全局选项（完成后动作） */
    "queue.getOptions": (): QueueOptions => structuredClone(queueOptions),
    "queue.setOption": (params) => {
      Object.assign(queueOptions, params as Partial<QueueOptions>);
      return true;
    },

    /* 全局热键 */
    "hotkey.get": (): HotkeyConfig => ({ ...hotkey }),
    "hotkey.set": (params) => {
      Object.assign(hotkey, params as Partial<HotkeyConfig>);
      return true;
    },
  };

  /* ------------------------------ Client ------------------------------ */

  return {
    async connect() {
      if (state === "connected") return; // 防止重复 connect 叠加心跳定时器
      state = "connected"; // mock 无需真实握手
      // 演示用：周期性推送一条日志事件，验证事件管线
      let n = 0;
      heartbeat = setInterval(() => {
        log("info", `[mock] 心跳 #${++n}（真实后端未接入）`);
      }, 5000);
    },
    close() {
      if (heartbeat) {
        clearInterval(heartbeat);
        heartbeat = null;
      }
      if (syncTimer) {
        clearInterval(syncTimer);
        syncTimer = null;
      }
      for (const timer of runTimers.values()) {
        clearInterval(timer);
      }
      runTimers.clear();
      listeners.clear();
      state = "disconnected";
    },
    connectionState: () => state,

    request<T>(method: string, params?: unknown): Promise<T> {
      const respond = (): RpcResponse => {
        const req: RpcRequest = {
          jsonrpc: "2.0",
          id: nextId++,
          method,
          params,
        };
        const handler = handlers[req.method];
        if (!handler) {
          return {
            jsonrpc: "2.0",
            id: req.id,
            error: {
              code: -32601,
              message: `Method not found: ${req.method}`,
            } as RpcError,
          };
        }
        try {
          return { jsonrpc: "2.0", id: req.id, result: handler(req.params) };
        } catch (e) {
          return {
            jsonrpc: "2.0",
            id: req.id,
            error: { code: -32000, message: String(e) },
          };
        }
      };

      // 模拟网络延迟
      return new Promise<RpcResponse>((resolve) => setTimeout(() => resolve(respond()), 80)).then(
        (res) => {
          if (res.error) throw new Error(res.error.message);
          return res.result as T;
        },
      );
    },

    on(event: IpcEventName, handler: (payload: unknown) => void) {
      let set = listeners.get(event);
      if (!set) {
        set = new Set();
        listeners.set(event, set);
      }
      set.add(handler);
      return () => {
        listeners.get(event)?.delete(handler);
      };
    },
  };
}
