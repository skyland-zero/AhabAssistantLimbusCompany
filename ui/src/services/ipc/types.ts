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
 * 后端主动推送的事件（非 JSON-RPC 信封，独立帧）：
 * { event: string, payload: unknown }
 */
export type IpcEventName =
  | "screenshot.frame" // payload: { instanceId: string; jpeg: ArrayBuffer; width; height }
  | "task.progress" // payload: TaskProgressPayload
  | "task.status" // payload: TaskStatusPayload
  | "log.entry" // payload: LogEntryPayload
  | "resource.sync.progress" // payload: SyncProgressPayload
  | "app.notice"; // payload: { level: 'info' | 'warn' | 'error'; message: string }

/* ------------------------------ 业务数据模型 ------------------------------ */

export type TaskStatus = "idle" | "queued" | "running" | "paused" | "done" | "failed";

/** 任务队列中的一项（对应旧版 farming_interface 的执行计划） */
export interface QueueTask {
  id: string;
  /** 任务模板 key：mirror / luxcavation / prizes / event ... */
  kind: string;
  name: string;
  teamId?: string;
  /** 运行次数；null = 无限 */
  repeat: number | null;
  status: TaskStatus;
  /** 进度 0-100 */
  progress: number;
  detail?: string;
}

export interface TeamSummary {
  id: string;
  name: string;
  /** 人格头像 key 列表 */
  sinners: string[];
}

export interface LogEntryPayload {
  ts: number;
  level: "debug" | "info" | "warn" | "error";
  message: string;
}
