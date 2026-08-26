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

/** 获取全局 IPC 客户端单例 */
export async function getIpc(): Promise<RpcClient> {
  if (!instance) {
    const { createMockClient } = await import("./mock/server");
    instance = createMockClient();
    await instance.connect();
  }
  return instance;
}
