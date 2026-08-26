import type { RpcClient } from "../client";
import type { IpcEventName, QueueTask, RpcError, RpcRequest, RpcResponse } from "../types";

/**
 * 内存版 mock 后端。
 * 提供与真实 WebSocket 后端一致的请求/事件语义，
 * 让 UI 在 M0-M5 阶段完全独立开发。
 */

type Handler = (params: unknown) => unknown;

/** 任务队列假数据 */
const fixtureTasks: QueueTask[] = [
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

const handlers: Record<string, Handler> = {
  "app.ping": () => "pong",
  "app.version": () => ({ ui: "0.1.0", backend: null }), // backend 未接入
  "task.list": () => structuredClone(fixtureTasks),
};

export function createMockClient(): RpcClient {
  let state: "disconnected" | "connecting" | "connected" = "disconnected";
  let nextId = 1;
  const listeners = new Map<IpcEventName, Set<(payload: unknown) => void>>();
  const timers = new Set<ReturnType<typeof setTimeout>>();

  const emit = (event: IpcEventName, payload: unknown) => {
    const set = listeners.get(event);
    if (!set) return;
    for (const handler of set) {
      handler(payload);
    }
  };

  return {
    async connect() {
      state = "connected"; // mock 无需真实握手
      // 演示用：周期性推送一条日志事件，验证事件管线
      let n = 0;
      const t = setInterval(() => {
        emit("log.entry", {
          ts: Date.now(),
          level: "info",
          message: `[mock] 心跳 #${++n}（真实后端未接入）`,
        });
      }, 5000);
      timers.add(t);
    },
    close() {
      for (const timer of timers) {
        clearInterval(timer);
      }
      timers.clear();
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
