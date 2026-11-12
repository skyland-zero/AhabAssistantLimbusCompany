// The theme-pack page's state: each pack's switch and weight.
//
// Rust owns the pack list and persists every change (`themePack.updateAll`,
// `themePack.resetWeights`); this module mirrors the returned list into signals.
import { signal } from "nsh";
import { Backend, ThemePacksStore } from "aalc/native";

/** One Mirror Dungeon theme pack. */
export type ThemePack = {
  id: string;
  name: string;
  /** Its star tier, shown as `T1`–`T4`. */
  tier: string;
  enabled: boolean;
  /** How likely it is to be picked, 0–10. */
  weight: number;
};

/** The weights a pack can take. */
export const MAX_WEIGHT = 10;

const packs = signal<ThemePack[]>([]);
const sortByWeight = signal(false);
const feedback = signal<string | null>(null);
const hardMirrorActive = signal(true);

let store: ThemePacksStore | null = null;
let connecting = false;

function toPack(view: {
  id: string;
  name: string;
  tier: string;
  enabled: boolean;
  weight: number;
}): ThemePack {
  return {
    id: view.id,
    name: view.name,
    tier: view.tier,
    enabled: view.enabled,
    weight: view.weight,
  };
}

function hydrate(view: ReturnType<ThemePacksStore["packs"]>): void {
  packs.set(view.map(toPack));
}

function failure(error: unknown): void {
  feedback.set(error instanceof Error ? error.message : String(error));
}

function ensureStore(): ThemePacksStore | null {
  if (store) return store;
  if (Backend.mode() === "mock") {
    try {
      store = new ThemePacksStore();
      hydrate(store.packs());
      hardMirrorActive.set(store.hardMirrorActive());
    } catch (error) {
      failure(error);
    }
  } else if (!connecting) {
    connecting = true;
    ThemePacksStore.connect()
      .then((created) => {
        store = created;
        hydrate(created.packs());
        hardMirrorActive.set(created.hardMirrorActive());
      })
      .catch(failure)
      .finally(() => {
        connecting = false;
      });
  }
  return store;
}

if (Backend.mode() === "mock") ensureStore();

/** The packs, in the order the page draws them (by weight when sorted). */
export function themePacks(): ThemePack[] {
  ensureStore();
  return packs();
}

/** The weight the enabled packs add up to, which is what the run rolls against. */
export function totalWeight(): number {
  return themePacks()
    .filter((pack) => pack.enabled)
    .reduce((sum, pack) => sum + pack.weight, 0);
}

/** Whether the list is sorted by weight (tracked). */
export function isSortedByWeight(): boolean {
  return sortByWeight();
}

/** Turns the weight sort on or off. */
export function toggleSortByWeight(): void {
  const next = !sortByWeight();
  sortByWeight.set(next);
  const backend = ensureStore();
  if (backend) {
    backend.setSortByWeight(next);
    hydrate(backend.packs());
  }
  feedback.set("主题包排序已更新");
}

/** Whether the hard-cycle warning shows (tracked). */
export function hardMirror(): boolean {
  ensureStore();
  return hardMirrorActive();
}

/** The line under the toolbar, when the last action left something to say. */
export function themePacksFeedback(): string | null {
  return feedback();
}

/** Turns one pack on or off. */
export function togglePack(id: string): void {
  const backend = ensureStore();
  if (!backend) return;
  try {
    hydrate(backend.toggle(id));
    feedback.set(backend.feedback());
  } catch (error) {
    failure(error);
  }
}

/** Sets one pack's weight, held inside the range. */
export function setPackWeight(id: string, weight: number): void {
  const held = Math.max(0, Math.min(MAX_WEIGHT, Math.round(weight)));
  packs.update((list) =>
    list.map((pack) => (pack.id === id ? { ...pack, weight: held } : pack)),
  );
  const backend = ensureStore();
  if (!backend) return;
  try {
    hydrate(backend.setWeight(id, held));
    feedback.set(backend.feedback());
  } catch (error) {
    failure(error);
  }
}

/** Turns every pack on or off. */
export function setAllEnabled(enabled: boolean): void {
  const backend = ensureStore();
  if (!backend) return;
  try {
    hydrate(backend.setAll(enabled));
    feedback.set(backend.feedback());
  } catch (error) {
    failure(error);
  }
}

/** Puts the backend's default weights back. */
export function resetWeights(): void {
  const backend = ensureStore();
  if (!backend) return;
  try {
    hydrate(backend.resetWeights());
    feedback.set(backend.feedback());
  } catch (error) {
    failure(error);
  }
}
