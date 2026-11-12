// The teams page's state: the saved teams, the filter, and which team is being
// edited.
//
// The data mirrors the sidecar's `Teams.*` payload (`TeamDetail`: name,
// purpose, accessory scheme, sinners, mirror config). P1 replaces the fixtures
// and the writes with RPC calls; the page keeps reading the same functions.
import { signal } from "nsh";

/** What a team is for. */
export type TeamPurpose = "mirror" | "luxcavation" | "general";

/** One saved team. */
export type Team = {
  /** `team-<number>`, the persisted formation id. */
  id: string;
  name: string;
  purpose: TeamPurpose;
  /** The accessory scheme's id (`tremor`, `burn`, ...). */
  accessoryScheme: string;
  /** Sinner ids, in party order. */
  sinners: string[];
  enabled: boolean;
  /** Opening bonuses carried into the mirror run. */
  starlight: number[];
  /** Discarded mirror choices, by count. */
  discarded: number;
};

/** The filter above the grid: one shared slot per formation, or one category. */
export type TeamFilter = "all" | TeamPurpose;

/** How many formation slots the game has. */
export const SLOT_COUNT = 14;

const teams = signal<Team[]>([
  {
    id: "team-1",
    name: "编队 1 (震颤)",
    purpose: "mirror",
    accessoryScheme: "tremor",
    sinners: ["faust", "ishmael", "ryoshu", "hong_lu"],
    enabled: true,
    starlight: [2, 15, 0, 0, 0, 0],
    discarded: 2,
  },
  {
    id: "team-2",
    name: "编队 2 (烧伤)",
    purpose: "luxcavation",
    accessoryScheme: "burn",
    sinners: ["heathcliff", "rodion", "gregor"],
    enabled: false,
    starlight: [],
    discarded: 0,
  },
  {
    id: "team-3",
    name: "编队 3 (呼吸)",
    purpose: "general",
    accessoryScheme: "poise",
    sinners: ["yi_sang", "don_quixote", "meursault", "sinclair", "outis"],
    enabled: false,
    starlight: [0, 12, 0, 0, 0, 0],
    discarded: 0,
  },
]);

const filter = signal<TeamFilter>("all");
const feedback = signal<string | null>(null);

/** Every saved team (tracked where it is read). */
export function allTeams(): Team[] {
  return teams();
}

/** The active filter (tracked). */
export function teamFilter(): TeamFilter {
  return filter();
}

/** Switches the filter. */
export function setTeamFilter(next: TeamFilter): void {
  filter.set(next);
}

/** How many teams a filter holds, for its tab's count. */
export function countFor(next: TeamFilter): number {
  if (next === "all") return teams().length;
  return teams().filter((team) => team.purpose === next).length;
}

/** The teams a filter shows. */
export function teamsFor(next: TeamFilter): Team[] {
  return next === "all" ? teams() : teams().filter((team) => team.purpose === next);
}

/** The formation numbers that filter owns, with the team saved there (if any).
 *
 *  Every formation slot is shown whether or not it holds a team, so an empty
 *  slot is a place to create one rather than a gap. */
export function slotsFor(next: TeamFilter): { number: number; team: Team | null }[] {
  const shown = teamsFor(next);
  return Array.from({ length: SLOT_COUNT }, (_, index) => {
    const number = index + 1;
    return { number, team: shown.find((team) => teamNumber(team.id) === number) ?? null };
  });
}

/** The formation number of `id` (`team-4` → 4), or `null`. */
export function teamNumber(id: string): number | null {
  if (!id.startsWith("team-")) return null;
  const number = Number(id.slice("team-".length));
  return Number.isInteger(number) && number > 0 ? number : null;
}

/** Turns a team on or off. */
export function setTeamEnabled(id: string, enabled: boolean): void {
  teams.update((list) =>
    list.map((team) => (team.id === id ? { ...team, enabled } : team)),
  );
  feedback.set(enabled ? "已启用队伍" : "已停用队伍");
}

/** Removes a team from its formation slot. */
export function deleteTeam(id: string): void {
  teams.update((list) => list.filter((team) => team.id !== id));
  feedback.set("队伍已删除");
}

/** The line under the toolbar, when the last action left something to say. */
export function teamsFeedback(): string | null {
  return feedback();
}

/** The draft the editor is editing, or `null` when it is closed. */
const draft = signal<Team | null>(null);

/** The draft in the editor (tracked). */
export function editingTeam(): Team | null {
  return draft();
}

/** Opens the editor on `team`, or on a new team in the first free slot. */
export function openEditor(team: Team | null): void {
  draft.set(team ?? blankTeam());
}

/** Closes the editor without saving. */
export function closeEditor(): void {
  draft.set(null);
}

/** Rewrites part of the draft. */
export function editTeam(change: Partial<Team>): void {
  draft.update((current) => (current === null ? null : { ...current, ...change }));
}

/** Turns a sinner in the draft on or off (party order follows the roster). */
export function toggleDraftSinner(id: string): void {
  draft.update((current) => {
    if (current === null) return null;
    const picked = current.sinners.includes(id);
    const sinners = picked
      ? current.sinners.filter((sinner) => sinner !== id)
      : [...current.sinners, id];
    return { ...current, sinners };
  });
}

/** Saves the draft: a new formation takes the next free number. */
export function saveDraft(): void {
  const current = draft();
  if (current === null) return;
  const name = current.name.trim();
  if (name === "") return;
  const saved = { ...current, name };
  teams.update((list) => {
    const existing = list.some((team) => team.id === saved.id);
    return existing ? list.map((team) => (team.id === saved.id ? saved : team)) : [...list, saved];
  });
  draft.set(null);
  feedback.set("队伍已保存");
}

/** A blank team for the first formation slot that holds nothing. */
function blankTeam(): Team {
  const used = new Set(teams().map((team) => teamNumber(team.id)));
  let number = 1;
  while (used.has(number)) number += 1;
  return {
    id: `team-${number}`,
    name: `编队 ${number}`,
    purpose: "mirror",
    accessoryScheme: "tremor",
    sinners: [],
    enabled: true,
    starlight: [],
    discarded: 0,
  };
}

/** The sinners a team can hold, in roster order. */
export const ROSTER = [
  "yi_sang",
  "faust",
  "don_quixote",
  "ryoshu",
  "meursault",
  "hong_lu",
  "heathcliff",
  "ishmael",
  "rodion",
  "sinclair",
  "outis",
  "gregor",
] as const;

/** The accessory schemes a team can wear. */
export const SCHEMES = [
  "tremor",
  "burn",
  "poise",
  "bleed",
  "sinking",
  "rupture",
  "charge",
] as const;

/** A team's purpose as a choice, for the editor's segmented control. */
export const PURPOSES: { value: TeamPurpose; label: string }[] = [
  { value: "mirror", label: "镜牢" },
  { value: "luxcavation", label: "经验本" },
  { value: "general", label: "通用" },
];

// ---------------------------------------------------------------------------
// Names the page shows for the ids the payload carries.

/** A team's purpose, as the page labels it. */
export const PURPOSE_LABELS: Record<TeamPurpose, string> = {
  mirror: "镜牢",
  luxcavation: "经验本",
  general: "通用",
};

/** The purpose with the accessory scheme's mark, as a card's meta line shows it. */
export const SCHEME_LABELS: Record<string, string> = {
  tremor: "震颤",
  burn: "烧伤",
  poise: "呼吸",
  bleed: "流血",
  sinking: "沉没",
  rupture: "破裂",
  charge: "充能",
};

/** The sinners, by the id the payload uses. */
export const SINNER_NAMES: Record<string, string> = {
  yi_sang: "李箱",
  faust: "浮士德",
  don_quixote: "堂吉诃德",
  ryoshu: "良秀",
  meursault: "默尔索",
  hong_lu: "鸿璐",
  heathcliff: "希斯克利夫",
  ishmael: "以实玛利",
  rodion: "罗佳",
  sinclair: "辛克莱",
  outis: "奥提斯",
  gregor: "格蕾戈尔",
};

/** How many mirror choices a team gives up (`discarded`). */
export function discardCount(team: Team): number {
  return team.purpose === "mirror" ? team.discarded : 0;
}

/** Whether a team carries opening bonuses into the run. */
export function hasStarlight(team: Team): boolean {
  return team.purpose !== "luxcavation" && team.starlight.some((level) => level > 0);
}
