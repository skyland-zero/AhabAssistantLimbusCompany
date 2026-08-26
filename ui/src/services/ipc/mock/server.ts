import type { RpcClient } from "../client";
import type {
  ConnectionStatus,
  DeviceInfo,
  DeviceStatusPayload,
  ExecutionState,
  ExecutionStatusPayload,
  FixedTaskId,
  HotkeyConfig,
  IpcEventName,
  MirrorProgressPayload,
  ResourceGroup,
  RpcError,
  RpcResponse,
  SinnerInfo,
  SyncProgressPayload,
  SystemSettingsConfig,
  TasksConfig,
  TeamDetail,
  ThemePack,
  ThemePackState,
  ToolId,
  ToolStatusPayload,
  UpdateInfo,
} from "../types";
import { createDefaultMirrorConfig } from "../types";

type Handler = (params: unknown) => unknown;

/**
 * 内存版 mock 后端。
 * 提供与真实 Python sidecar 一致的请求/事件语义。
 */
export function createMockClient(): RpcClient {
  let state: "disconnected" | "connecting" | "connected" = "disconnected";
  let nextId = 1;
  let heartbeat: ReturnType<typeof setInterval> | null = null;
  let syncTimer: ReturnType<typeof setInterval> | null = null;

  /* ------------------------------ 假数据 ------------------------------ */

  const SINNERS: SinnerInfo[] = [
    { id: "yi_sang", name: "李箱" },
    { id: "faust", name: "浮士德" },
    { id: "don_quixote", name: "堂吉诃德" },
    { id: "ryoshu", name: "良秀" },
    { id: "meursault", name: "默尔索" },
    { id: "hong_lu", name: "鸿璐" },
    { id: "heathcliff", name: "希斯克利夫" },
    { id: "ishmael", name: "以实玛利" },
    { id: "rodion", name: "罗佳" },
    { id: "sinclair", name: "辛克莱" },
    { id: "outis", name: "奥提斯" },
    { id: "gregor", name: "格雷戈尔" },
  ];

  let teams: TeamDetail[] = [
    {
      id: "team-1",
      name: "编队 1 (震颤)",
      purpose: "mirror",
      sinners: ["faust", "ishmael", "ryoshu", "hong_lu"],
      accessoryScheme: "tremor",
      enabled: true,
      mirrorConfig: {
        ...createDefaultMirrorConfig(),
        team_system: 2,
        discard_systems: {
          ...createDefaultMirrorConfig().discard_systems,
          sinking: true,
          poise: true,
        },
        opening_bonus: [2, 2, 1, 1, 0, 0, 0, 0, 0, 0],
      },
    },
    {
      id: "team-2",
      name: "编队 2 (烧伤)",
      purpose: "luxcavation",
      sinners: ["heathcliff", "rodion", "gregor"],
      accessoryScheme: "burn",
      enabled: true,
      mirrorConfig: {
        ...createDefaultMirrorConfig(),
        team_system: 0,
      },
    },
    {
      id: "team-3",
      name: "编队 3 (呼吸)",
      purpose: "general",
      sinners: ["yi_sang", "don_quixote", "meursault", "sinclair", "outis"],
      accessoryScheme: "poise",
      enabled: false,
      mirrorConfig: {
        ...createDefaultMirrorConfig(),
        team_system: 5,
      },
    },
  ];

  let tasksConfig: TasksConfig = {
    enabledTasks: {
      daily_task: true,
      get_reward: true,
      buy_enkephalin: false,
      mirror: true,
      resonate_with_Ahab: true,
    },
    set_windows: {
      set_win_size: 1080,
      set_win_position: "0",
      set_reduce_miscontact: true,
      screenshot_interval: 0.5,
      mouse_action_interval: 0.3,
      mouse_down_duration: 0.1,
      use_post_message: false,
    },
    daily_task: {
      set_EXP_count: 3,
      set_thread_count: 3,
      daily_teams: 1,
      use_continuous_combat: true,
      use_continuous_combat_select: 3,
      targeted_teaming_EXP: false,
      EXP_day_1_2: 1,
      EXP_day_3_4: 1,
      EXP_day_5_6: 1,
      EXP_day_7: 1,
      targeted_teaming_thread: false,
      thread_day_1: 1,
      thread_day_2: 1,
      thread_day_3: 1,
      thread_day_4: 1,
      thread_day_5: 1,
      thread_day_6: 1,
      thread_day_7: 1,
    },
    get_reward: {
      set_get_prize: 0,
    },
    buy_enkephalin: {
      set_lunacy_to_enkephalin: 2,
      Dr_Grandet_mode: true,
      skip_enkephalin: false,
    },
    mirror: {
      set_mirror_count: 3,
      infinite_dungeons: false,
      hard_mirror: false,
      no_weekly_bonuses: false,
      floor_3_exit: false,
      save_rewards: false,
      hard_mirror_single_bonuses: false,
      select_event_pack: false,
      skip_event_pack: false,
      re_claim_rewards: false,
      not_skip_whitegossypium: false,
      fight_to_last_man: false,
      mirror_keyboard_navigation: false,
      mirror_keyboard_simple_pathfinding: false,
    },
    resonate_with_Ahab: {
      enabled: true,
    },
    afterCompletion: {
      actions: ["exit_game"],
      powerAction: "none",
      keepAfterCompletion: true,
    },
  };

  let executionState: ExecutionState = "idle";
  let currentRunningTask: FixedTaskId | null = null;
  let executionTimer: ReturnType<typeof setTimeout> | null = null;

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

  const hotkey: HotkeyConfig = { startStop: "F10", pauseResume: "F11", enabled: true };
  const systemSettings: SystemSettingsConfig = {
    simulator: true,
    simulator_type: 0,
    simulator_port: 16384,
    start_emulator_timeout: 60,
    memory_protection: true,
    minimize_to_tray: true,
    autostart: false,
    experimental_keep_screen_awake: true,
    experimental_hdr_warning: true,
    update_prerelease_enable: false,
    update_source: "GitHub",
    mirrorchyan_cdk: "",
  };

  const devices: DeviceInfo[] = [
    { id: "win-limbus", name: "Limbus Company", detail: "1920×1080 · 窗口化" },
    { id: "mumu-instance", name: "MuMu 模拟器", detail: "1280×720 · 端口 16384" },
  ];
  let deviceStatus: ConnectionStatus = "connected";

  /* ------------------------------ 内部事件发射 ------------------------------ */

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

  const setExecutionState = (st: ExecutionState, taskId: FixedTaskId | null) => {
    executionState = st;
    currentRunningTask = taskId;
    emit("execution.status", { state: st, currentTaskId: taskId } satisfies ExecutionStatusPayload);
  };

  const stopExecutionSim = () => {
    if (executionTimer) {
      clearTimeout(executionTimer);
      executionTimer = null;
    }
    setExecutionState("idle", null);
  };

  const startExecutionSim = () => {
    stopExecutionSim();
    setExecutionState("running", null);
    log("info", "Link Start! 开始执行所有已勾选任务");

    const steps: { name: string; id: FixedTaskId; duration: number }[] = [];
    if (tasksConfig.enabledTasks.daily_task) {
      steps.push({ name: "日常任务 (经验本 & 纽本)", id: "daily_task", duration: 2500 });
    }
    if (tasksConfig.enabledTasks.get_reward) {
      steps.push({ name: "领取奖励", id: "get_reward", duration: 1500 });
    }
    if (tasksConfig.enabledTasks.buy_enkephalin) {
      steps.push({ name: "狂气换体", id: "buy_enkephalin", duration: 1500 });
    }
    if (tasksConfig.enabledTasks.mirror) {
      steps.push({ name: "坐牢任务 (镜牢探索)", id: "mirror", duration: 4000 });
    }

    if (steps.length === 0) {
      log("warn", "未勾选任何执行任务，流程结束");
      stopExecutionSim();
      return;
    }

    let stepIdx = 0;
    const runNextStep = () => {
      if (executionState !== "running") return;
      if (stepIdx >= steps.length) {
        log("info", "所有任务已完成！");
        if (
          tasksConfig.afterCompletion.actions.length > 0 ||
          tasksConfig.afterCompletion.powerAction !== "none"
        ) {
          log(
            "info",
            `执行收尾动作：${tasksConfig.afterCompletion.actions.join(", ") || "无"} / ${tasksConfig.afterCompletion.powerAction}`,
          );
        }
        stopExecutionSim();
        return;
      }
      const cur = steps[stepIdx];
      setExecutionState("running", cur.id);
      log("info", `>> 开始执行：${cur.name}`);

      if (cur.id === "mirror") {
        const total = tasksConfig.mirror.infinite_dungeons
          ? 9999
          : tasksConfig.mirror.set_mirror_count;
        emit("execution.mirrorProgress", {
          current: 1,
          total,
          isHard: tasksConfig.mirror.hard_mirror,
          isInfinite: tasksConfig.mirror.infinite_dungeons,
        } satisfies MirrorProgressPayload);
      }

      executionTimer = setTimeout(() => {
        log("info", `✓ 完成：${cur.name}`);
        stepIdx++;
        runNextStep();
      }, cur.duration);
    };

    runNextStep();
  };

  /* ------------------------------ Handlers ------------------------------ */

  const handlers: Record<string, Handler> = {
    "app.ping": () => "pong",
    "app.version": () => ({ ui: __APP_VERSION__, backend: "mock-1.0.0" }),
    "app.checkUpdate": (): UpdateInfo => ({
      updateAvailable: false,
      latest: __APP_VERSION__,
    }),

    /* 任务配置与调度 */
    "tasks.getConfig": () => structuredClone(tasksConfig),
    "tasks.setConfig": (params) => {
      tasksConfig = structuredClone(params as TasksConfig);
      return true;
    },
    "execution.getState": (): ExecutionStatusPayload => ({
      state: executionState,
      currentTaskId: currentRunningTask,
    }),
    "execution.start": () => {
      startExecutionSim();
      return true;
    },
    "execution.stop": () => {
      stopExecutionSim();
      log("warn", "用户手动停止了任务执行");
      return true;
    },
    "execution.pause": () => {
      if (executionState === "running") {
        setExecutionState("paused", currentRunningTask);
        log("info", "任务执行已暂停");
      }
      return true;
    },
    "execution.resume": () => {
      if (executionState === "paused") {
        setExecutionState("running", currentRunningTask);
        log("info", "任务执行已恢复");
      }
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
      toolRunning.set(toolId, true);
      emit("tool.status", { toolId, running: true } satisfies ToolStatusPayload);
      log("info", `[mock] 工具启动：${toolId}`);
      return true;
    },
    "tool.stop": (params) => {
      const toolId = (params as { id?: ToolId }).id;
      if (!toolId) throw new Error("tool id required");
      toolRunning.set(toolId, false);
      emit("tool.status", { toolId, running: false } satisfies ToolStatusPayload);
      log("info", `[mock] 工具停止：${toolId}`);
      return true;
    },
    "tool.screenshot": () => {
      log("info", "[mock] 截图完成，已保存至 AALC/screenshots 目录");
      return { path: "AALC/screenshots/2025-06-01_12-00-00.png" };
    },

    /* 主题包 */
    "themePack.list": () => structuredClone(themePackState),
    "themePack.save": (params) => {
      const pack = params as ThemePack;
      const idx = themePackState.packs.findIndex((p) => p.id === pack.id);
      if (idx >= 0) themePackState.packs[idx] = structuredClone(pack);
      return true;
    },
    "themePack.setHardMirrorActive": (params) => {
      const active = (params as { active: boolean }).active;
      themePackState.hardMirrorActive = active;
      return true;
    },

    /* 资源中心 */
    "resource.list": () => structuredClone(resources),
    "resource.sync": (params) => {
      const scope = ((params as { scope?: string })?.scope ?? "all") as string;
      log("info", `[mock] 开始同步资源：${scope}`);
      let progress = 0;
      if (syncTimer) clearInterval(syncTimer);
      syncTimer = setInterval(() => {
        progress += 25;
        emit("resource.sync.progress", { scope, progress } satisfies SyncProgressPayload);
        if (progress >= 100) {
          if (syncTimer) clearInterval(syncTimer);
          syncTimer = null;
          resources = resources.map((r) =>
            scope === "all" || r.id === scope
              ? { ...r, localVersion: "v2025.06.2", lastSyncAt: Date.now() }
              : r,
          );
          log("info", `[mock] 资源同步完成：${scope}`);
        }
      }, 300);
      return true;
    },

    /* 全局热键 */
    "hotkey.get": () => structuredClone(hotkey),
    "hotkey.set": (params) => {
      Object.assign(hotkey, params as HotkeyConfig);
      return true;
    },

    /* 系统设置 */
    "systemSettings.get": () => structuredClone(systemSettings),
    "systemSettings.set": (params) => {
      Object.assign(systemSettings, params as Partial<SystemSettingsConfig>);
      return true;
    },

    /* 设备连接 */
    "device.list": () => structuredClone(devices),
    "device.connect": (params) => {
      const id = (params as { id: string }).id;
      deviceStatus = "connected";
      emit("device.status", { deviceId: id, status: deviceStatus } satisfies DeviceStatusPayload);
      log("info", `已连接设备：${id}`);
      return true;
    },
    "device.disconnect": () => {
      deviceStatus = "disconnected";
      emit("device.status", { deviceId: null, status: deviceStatus } satisfies DeviceStatusPayload);
      log("info", "设备已断开连接");
      return true;
    },
  };

  /* ------------------------------ Client 实现 ------------------------------ */

  const client: RpcClient = {
    connect: async () => {
      if (state === "connected") return;
      state = "connecting";
      await new Promise((r) => setTimeout(r, 50));
      state = "connected";

      heartbeat = setInterval(() => {
        if (Math.random() < 0.05) {
          emit("log.entry", {
            ts: Date.now(),
            level: "debug",
            message: "自动化引擎心跳探测正常",
          });
        }
      }, 15000);
    },

    close: () => {
      state = "disconnected";
      if (heartbeat) clearInterval(heartbeat);
      if (syncTimer) clearInterval(syncTimer);
      listeners.clear();
    },

    connectionState: () => state,

    request: async <T>(method: string, params?: unknown): Promise<T> => {
      const reqId = nextId++;
      const handler = handlers[method];
      if (!handler) {
        const error: RpcError = { code: -32601, message: `Method not found: ${method}` };
        const res: RpcResponse = { jsonrpc: "2.0", id: reqId, error };
        throw new Error(res.error?.message);
      }
      try {
        const result = (await handler(params)) as T;
        return result;
      } catch (err) {
        throw err instanceof Error ? err : new Error(String(err));
      }
    },

    on: (event: IpcEventName, handler: (payload: unknown) => void) => {
      let set = listeners.get(event);
      if (!set) {
        set = new Set();
        listeners.set(event, set);
      }
      set.add(handler);
      return () => {
        set?.delete(handler);
      };
    },
  };

  return client;
}
