import type { IpcEventName } from "./types";

/**
 * UI 层唯一允许依赖的 IPC 接口。
 * M0-M5: getIpc() 返回 MockRpcClient（内存实现）
 * M6+:   切换为 WebSocket 实现，UI 零改动
 */
export interface RpcClient {
  /** 建立/恢复连接 */
  connect(): Promise<void>;
  close(): void;
  connectionState(): "disconnected" | "connecting" | "connected";

  /** JSON-RPC 请求 */
  request<T = unknown>(method: string, params?: unknown): Promise<T>;

  /** 订阅后端事件推送，返回取消订阅函数 */
  on(event: IpcEventName, handler: (payload: unknown) => void): () => void;
}

let instance: RpcClient | null = null;
let instancePromise: Promise<RpcClient> | null = null;

/** 获取全局 IPC 客户端单例 */
export function getIpc(): Promise<RpcClient> {
  if (instance) return Promise.resolve(instance);
  if (instancePromise) return instancePromise;

  // 多个页面/组件首次同时请求时共用同一个初始化 Promise，避免创建多个
  // mock/WebSocket 客户端及其心跳定时器，最后只保留其中一个实例。
  instancePromise = (async () => {
    const { createMockClient } = await import("./mock/server");
    const client = createMockClient();
    await client.connect();
    instance = client;
    return client;
  })().catch((error) => {
    instancePromise = null;
    throw error;
  });

  return instancePromise;
}
