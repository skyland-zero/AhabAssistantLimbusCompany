/**
 * IPC 契约层类型定义
 *
 * 前端与后端(Python sidecar)之间使用 WebSocket + JSON-RPC 风格消息。
 * 第一阶段由 mock/server.ts 提供内存实现；M6 阶段由 ws.ts 提供真实实现。
 * UI 层只允许依赖 client.ts 的 RpcClient 接口，禁止直接 import mock/ws 实现。
 */

/** JSON-RPC 2.0 请求 */
export interface RpcRequest {
  jsonrpc: "2.0";
  id: number;
  method: string;
  params?: unknown;
}

/** JSON-RPC 2.0 响应 */
export interface RpcResponse<T = unknown> {
  jsonrpc: "2.0";
  id: number;
  result?: T;
  error?: RpcError;
}

/** JSON-RPC 错误对象 */
export interface RpcError {
  code: number;
  message: string;
  data?: unknown;
}

/**
 * 后端主动推送的事件：
 * { event: string, payload: unknown }
 */
export type IpcEventName =
  | "screenshot.frame" // payload: { instanceId: string; jpeg: ArrayBuffer; width; height }
  | "execution.status" // payload: ExecutionStatusPayload
  | "execution.mirrorProgress" // payload: MirrorProgressPayload
  | "tool.status" // payload: ToolStatusPayload
  | "device.status" // payload: DeviceStatusPayload
  | "log.entry" // payload: LogEntryPayload
  | "resource.sync.progress" // payload: SyncProgressPayload
  | "app.notice"; // payload: { level: 'info' | 'warn' | 'error'; message: string }

/* ------------------------------ 任务与执行数据模型 ------------------------------ */

export type FixedTaskId =
  | "set_windows"
  | "daily_task"
  | "get_reward"
  | "buy_enkephalin"
  | "mirror"
  | "resonate_with_Ahab";

export type ExecutionState = "idle" | "running" | "paused";

export type AfterExitAction = "exit_game" | "exit_emulator" | "exit_aalc";
export type AfterPowerAction = "none" | "sleep" | "hibernate" | "lock" | "shutdown";

export interface AfterCompletionConfig {
  actions: AfterExitAction[];
  powerAction: AfterPowerAction;
  keepAfterCompletion: boolean;
}

/** 窗口设置 */
export interface SetWindowsConfig {
  set_win_size: number;
  set_win_position: string;
  set_reduce_miscontact: boolean;
  screenshot_interval: number;
  mouse_action_interval: number;
  mouse_down_duration: number;
  use_post_message: boolean;
}

/** 日常任务设置 */
export interface DailyTaskConfig {
  set_EXP_count: number;
  set_thread_count: number;
  daily_teams: number;
  use_continuous_combat: boolean;
  use_continuous_combat_select: number;
  targeted_teaming_EXP: boolean;
  EXP_day_1_2: number;
  EXP_day_3_4: number;
  EXP_day_5_6: number;
  EXP_day_7: number;
  targeted_teaming_thread: boolean;
  thread_day_1: number;
  thread_day_2: number;
  thread_day_3: number;
  thread_day_4: number;
  thread_day_5: number;
  thread_day_6: number;
  thread_day_7: number;
}

/** 领取奖励设置 */
export interface GetRewardConfig {
  set_get_prize: number; // 0: 全部, 1: 狂气与通行证, 2: 仅邮件
}

/** 狂气换体设置 */
export interface BuyEnkephalinConfig {
  set_lunacy_to_enkephalin: number; // 0-10次
  Dr_Grandet_mode: boolean;
  skip_enkephalin: boolean;
}

/** 镜牢设置 */
export interface MirrorConfig {
  set_mirror_count: number;
  infinite_dungeons: boolean;
  hard_mirror: boolean;
  no_weekly_bonuses: boolean;
  floor_3_exit: boolean;
  save_rewards: boolean;
  hard_mirror_single_bonuses: boolean;
  select_event_pack: boolean;
  skip_event_pack: boolean;
  re_claim_rewards: boolean;
  not_skip_whitegossypium: boolean;
  fight_to_last_man: boolean;
  mirror_keyboard_navigation: boolean;
  mirror_keyboard_simple_pathfinding: boolean;
}

/** 所有固定任务的完整配置汇总 */
export interface TasksConfig {
  enabledTasks: {
    daily_task: boolean;
    get_reward: boolean;
    buy_enkephalin: boolean;
    mirror: boolean;
    resonate_with_Ahab: boolean;
  };
  set_windows: SetWindowsConfig;
  daily_task: DailyTaskConfig;
  get_reward: GetRewardConfig;
  buy_enkephalin: BuyEnkephalinConfig;
  mirror: MirrorConfig;
  resonate_with_Ahab: {
    enabled: boolean;
  };
  afterCompletion: AfterCompletionConfig;
}

export interface ExecutionStatusPayload {
  state: ExecutionState;
  currentTaskId: FixedTaskId | null;
}

export interface MirrorProgressPayload {
  current: number;
  total: number;
  isHard: boolean;
  isInfinite: boolean;
}

/* ------------------------------ 队伍与其他数据模型 ------------------------------ */

export interface TeamSummary {
  id: string;
  name: string;
  sinners: string[];
}

export type TeamPurpose = "mirror" | "luxcavation" | "general";

export interface TeamDetail extends TeamSummary {
  purpose: TeamPurpose;
  accessoryScheme: string;
  enabled: boolean;
}

export interface SinnerInfo {
  id: string;
  name: string;
}

export type ToolId = "infinite_battle" | "enkephalin" | "screenshot";

export interface ToolStatusPayload {
  toolId: ToolId;
  running: boolean;
}

export interface ThemePack {
  id: string;
  name: string;
  weight: number;
  enabled: boolean;
  tier: string;
}

export interface ThemePackState {
  hardMirrorActive: boolean;
  packs: ThemePack[];
}

export interface ResourceGroup {
  id: string;
  name: string;
  localVersion: string;
  remoteVersion: string | null;
  lastSyncAt: number | null;
}

export interface SyncProgressPayload {
  scope: string;
  progress: number;
}

export interface LogEntryPayload {
  ts: number;
  level: "debug" | "info" | "warn" | "error";
  message: string;
}

export interface HotkeyConfig {
  startStop: string;
  pauseResume?: string;
  enabled: boolean;
}

export interface SystemSettingsConfig {
  // 模拟器设置
  simulator: boolean;
  simulator_type: number; // 0: MuMu, 10: Other
  simulator_port: number;
  start_emulator_timeout: number;

  // 系统与内存保护
  memory_protection: boolean;
  minimize_to_tray: boolean;
  autostart: boolean;

  // 实验性功能
  experimental_keep_screen_awake: boolean;
  experimental_hdr_warning: boolean;

  // 更新设置
  update_prerelease_enable: boolean;
  update_source: "GitHub" | "MirrorChyan";
  mirrorchyan_cdk: string;
}
export interface UpdateInfo {
  updateAvailable: boolean;
  latest: string;
}

/* ------------------------------ 设备连接 ------------------------------ */

export interface DeviceInfo {
  id: string;
  name: string;
  detail?: string;
}

export type ConnectionStatus = "disconnected" | "connecting" | "connected";

export interface DeviceStatusPayload {
  deviceId: string | null;
  status: ConnectionStatus;
}
