// The theme-pack page's state: each pack's switch and weight.
//
// Theme packs are the Mirror Dungeon pack filter: only enabled packs are picked,
// each with a weight 0–10. The payload is the sidecar's (`ThemePacks.*`); P1
// replaces the fixtures and writes with RPC calls.
import { signal } from "nsh";

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

const packs = signal<ThemePack[]>([
  { id: "black-cloud", name: "黑云会", tier: "T1", enabled: true, weight: 5 },
  { id: "thumb", name: "拇指", tier: "T2", enabled: true, weight: 3 },
  { id: "blade-brotherhood", name: "利刃兄弟会", tier: "T2", enabled: true, weight: 4 },
  { id: "wu-merchant", name: "厄伍商会", tier: "T3", enabled: false, weight: 2 },
  { id: "district-20", name: "二十区福利机构", tier: "T1", enabled: true, weight: 6 },
  { id: "tech-liberator", name: "技术科学解放者联盟", tier: "T4", enabled: false, weight: 1 },
  { id: "company-hq", name: "公司总部", tier: "T3", enabled: true, weight: 3 },
  { id: "resonance", name: "残响乐团", tier: "T1", enabled: true, weight: 7 },
]);

const sortByWeight = signal(false);
const feedback = signal<string | null>(null);

/** Whether the Mirror Dungeon is in its hard cycle, which the page warns about. */
const hardMirrorActive = signal(true);

/** The packs, in the order the page draws them (by weight when sorted). */
export function themePacks(): ThemePack[] {
  const list = packs();
  return sortByWeight()
    ? [...list].sort((left, right) => right.weight - left.weight)
    : list;
}

/** The weight the enabled packs add up to, which is what the run rolls against. */
export function totalWeight(): number {
  return packs()
    .filter((pack) => pack.enabled)
    .reduce((sum, pack) => sum + pack.weight, 0);
}

/** Whether the list is sorted by weight (tracked). */
export function isSortedByWeight(): boolean {
  return sortByWeight();
}

/** Turns the weight sort on or off. */
export function toggleSortByWeight(): void {
  sortByWeight.update((current) => !current);
  feedback.set("主题包排序已更新");
}

/** Whether the hard-cycle warning shows (tracked). */
export function hardMirror(): boolean {
  return hardMirrorActive();
}

/** The line under the toolbar, when the last action left something to say. */
export function themePacksFeedback(): string | null {
  return feedback();
}

/** Turns one pack on or off. */
export function togglePack(id: string): void {
  packs.update((list) =>
    list.map((pack) => (pack.id === id ? { ...pack, enabled: !pack.enabled } : pack)),
  );
}

/** Sets one pack's weight, held inside the range. */
export function setPackWeight(id: string, weight: number): void {
  const held = Math.max(0, Math.min(MAX_WEIGHT, Math.round(weight)));
  packs.update((list) =>
    list.map((pack) => (pack.id === id ? { ...pack, weight: held } : pack)),
  );
}

/** Turns every pack on or off. */
export function setAllEnabled(enabled: boolean): void {
  packs.update((list) => list.map((pack) => ({ ...pack, enabled })));
  feedback.set(enabled ? "主题包已全部启用" : "主题包已全部停用");
}

/** Puts the shipped weights back. */
export function resetWeights(): void {
  packs.update((list) => list.map((pack) => ({ ...pack, weight: defaults[pack.id] ?? pack.weight })));
  feedback.set("主题包权重已恢复");
}

/** The weights the packs ship with (`ThemePacksState::default`). */
const defaults: Record<string, number> = {
  "black-cloud": 5,
  thumb: 3,
  "blade-brotherhood": 4,
  "wu-merchant": 2,
  "district-20": 6,
  "tech-liberator": 1,
  "company-hq": 3,
  resonance: 7,
};
