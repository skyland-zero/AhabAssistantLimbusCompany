// The resource sync's state (Settings → 资源同步): the resource groups, their
// versions and the sync in flight.
//
// Rust owns the groups and the sync (`resource.status` / `resource.checkUpdate`
// / `resource.syncStart`); this module mirrors the snapshots and gives the
// terminal 100% a short moment on screen.
import { signal } from "nsh";
import { Backend, ResourcesStore } from "aalc/native";
import { translatedFeedback } from "./settings";

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

const groups = signal<ResourceGroup[]>([]);
const syncProgress = signal<number | null>(null);
const feedback = signal<string | null>(null);

let store: ResourcesStore | null = null;
let connecting = false;

function toGroup(view: {
  id: string;
  name: string;
  localVersion: string;
  remoteVersion?: string | null;
  lastSyncAt?: number | null;
}): ResourceGroup {
  return {
    id: view.id,
    name: view.name,
    localVersion: view.localVersion,
    remoteVersion: view.remoteVersion ?? null,
    lastSyncAt: view.lastSyncAt ?? null,
  };
}

function hydrate(view: ReturnType<ResourcesStore["groups"]>): void {
  groups.set(view.map(toGroup));
}

function failure(error: unknown): void {
  feedback.set(error instanceof Error ? error.message : String(error));
}

function ensureStore(): ResourcesStore | null {
  if (store) return store;
  if (Backend.mode() === "mock") {
    try {
      store = new ResourcesStore();
      hydrate(store.groups());
    } catch (error) {
      failure(error);
    }
  } else if (!connecting) {
    connecting = true;
    ResourcesStore.connect()
      .then((created) => {
        store = created;
        hydrate(created.groups());
      })
      .catch(failure)
      .finally(() => {
        connecting = false;
      });
  }
  return store;
}

if (Backend.mode() === "mock") ensureStore();

/** The resource groups (tracked where they are read). */
export function resourceGroups(): ResourceGroup[] {
  ensureStore();
  return groups();
}

/** The sync's progress in percent, or `null` when nothing is syncing. */
export function currentSyncProgress(): number | null {
  return syncProgress();
}

/** The line under the grid, when the last action left something to say. */
export function resourcesFeedback(): string | null {
  return translatedFeedback(feedback());
}

/** Whether `group` has a newer version than the one on disk. */
export function hasUpdate(group: ResourceGroup): boolean {
  return group.remoteVersion !== null && group.remoteVersion !== group.localVersion;
}

/** Re-reads the remote versions. */
export function checkResources(): void {
  const backend = ensureStore();
  if (!backend) return;
  try {
    hydrate(backend.check());
    feedback.set(backend.feedback());
  } catch (error) {
    failure(error);
  }
}

/** Starts a sync; the terminal progress settles after a short beat. */
export function syncResources(): void {
  if (syncProgress() !== null) return;
  const backend = ensureStore();
  if (!backend) return;
  try {
    const progress = backend.syncNow();
    feedback.set(backend.feedback());
    syncProgress.set(progress);
    setTimeout(() => {
      hydrate(backend.finishSync());
      syncProgress.set(null);
      feedback.set(backend.feedback());
    }, 450);
  } catch (error) {
    syncProgress.set(null);
    failure(error);
  }
}
