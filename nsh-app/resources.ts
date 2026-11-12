// The resources page's state: the resource groups, their versions and the sync
// in flight.
//
// Resource groups and the sync progress come from the sidecar
// (`Resources.status`, `Resources.sync`); P1 replaces the fixtures and the timer
// below with those calls and their `resources.sync` events. The page keeps
// reading the same functions.
import { signal } from "nsh";

/** One synchronised resource set. */
export type ResourceGroup = {
  id: string;
  name: string;
  /** The version on disk. */
  localVersion: string;
  /** The version the remote offers; `null` when the remote is not known. */
  remoteVersion: string | null;
  /** When it last synced, or `null` for never. */
  lastSyncAt: number | null;
};

const groups = signal<ResourceGroup[]>([
  {
    id: "templates",
    name: "模板资源",
    localVersion: "v2025.06.1",
    remoteVersion: null,
    lastSyncAt: null,
  },
  {
    id: "onnx",
    name: "ONNX 模型",
    localVersion: "v1.2.0",
    remoteVersion: null,
    lastSyncAt: null,
  },
]);

const syncProgress = signal<number | null>(null);
const feedback = signal<string | null>(null);

/** The resource groups (tracked where they are read). */
export function resourceGroups(): ResourceGroup[] {
  return groups();
}

/** The sync's progress in percent, or `null` when nothing is syncing. */
export function currentSyncProgress(): number | null {
  return syncProgress();
}

/** The line under the grid, when the last action left something to say. */
export function resourcesFeedback(): string | null {
  return feedback();
}

/** Whether `group` has a newer version than the one on disk. */
export function hasUpdate(group: ResourceGroup): boolean {
  return group.remoteVersion !== null && group.remoteVersion !== group.localVersion;
}

/** Re-reads the remote versions. */
export function checkResources(): void {
  feedback.set("资源更新状态已刷新");
}

/** Starts a sync; the progress runs to 100% and then settles.
 *
 *  A stand-in for the sidecar's `resources.sync` events: it is the one place in
 *  this page that keeps a timer, and P1 deletes it with the rest of the mock. */
export function syncResources(): void {
  if (syncProgress() !== null) return;
  feedback.set("正在同步资源…");
  syncProgress.set(0);
  const step = () => {
    const current = syncProgress();
    if (current === null) return;
    const next = current + 25;
    if (next >= 100) {
      syncProgress.set(null);
      groups.update((list) =>
        list.map((group) => ({ ...group, lastSyncAt: Date.now() })),
      );
      feedback.set("资源同步完成");
      return;
    }
    syncProgress.set(next);
    setTimeout(step, 150);
  };
  setTimeout(step, 150);
}
