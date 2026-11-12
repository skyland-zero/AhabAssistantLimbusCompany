// The saved teams: Rust owns the list, the formation slots and every
// `team.*` mutation. This module mirrors the returned arrays into signals and
// keeps the labels the list shows. The editor's draft stays in
// `teams-editor.ts`; both are re-exported by `teams.ts`.
import { signal } from "nsh";
import { Backend, TeamsStore } from "aalc/native";
import type { TeamSlotView, TeamView } from "aalc/native";
import { SYSTEM_LABELS, defaultMirrorConfig } from "./teams-mirror";
import type { TeamMirrorConfig } from "./teams-mirror";

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
  /** The code the game imports to rebuild the formation. */
  teamCode: string;
  /** The mirror-run configuration; `null` for an experience-dungeon team. */
  mirrorConfig: TeamMirrorConfig | null;
};

/** The filter above the grid: one shared slot per formation, or one category. */
export type TeamFilter = "all" | TeamPurpose;

/** The saved teams, mirrored from the Rust store. */
export const teams = signal<Team[]>([]);

const filter = signal<TeamFilter>("all");
const feedback = signal<string | null>(null);

let store: TeamsStore | null = null;
let connecting = false;

function toTeam(view: TeamView): Team {
  return {
    id: view.id,
    name: view.name,
    purpose: view.purpose as TeamPurpose,
    accessoryScheme: view.accessoryScheme,
    sinners: view.sinners,
    enabled: view.enabled,
    teamCode: view.teamCode,
    mirrorConfig: view.mirrorConfig as TeamMirrorConfig | null,
  };
}

function hydrate(list: TeamView[]): void {
  teams.set(list.map(toTeam));
}

function failure(error: unknown): void {
  feedback.set(error instanceof Error ? error.message : String(error));
}

/** The Rust store once it is loaded, or `null` while a sidecar connects. */
function ensureStore(): TeamsStore | null {
  if (store) return store;
  if (Backend.mode() === "mock") {
    try {
      store = new TeamsStore();
      const error = store.error();
      if (error) feedback.set(error);
      hydrate(store.teams());
    } catch (error) {
      failure(error);
    }
  } else if (!connecting) {
    connecting = true;
    TeamsStore.connect()
      .then((created) => {
        store = created;
        hydrate(created.teams());
      })
      .catch(failure)
      .finally(() => {
        connecting = false;
      });
  }
  return store;
}

/** The Rust store for the stats module; `null` while a sidecar connects. */
export function teamsBackend(): TeamsStore | null {
  return ensureStore();
}

// Mock mode loads synchronously so the first render, tests and offline
// development see the saved teams. A real sidecar loads through `connect()`.
if (Backend.mode() === "mock") ensureStore();

/** Every saved team (tracked where it is read). */
export function allTeams(): Team[] {
  ensureStore();
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
  const list = allTeams();
  if (next === "all") return list.length;
  return list.filter((team) => team.purpose === next).length;
}

/** The teams a filter shows. */
export function teamsFor(next: TeamFilter): Team[] {
  const list = allTeams();
  return next === "all" ? list : list.filter((team) => team.purpose === next);
}

/** The fixed formation numbers a filter owns, with the team saved there. */
export function slotsFor(next: TeamFilter): { number: number; team: Team | null }[] {
  // Reading the mirrored list keeps this binding reactive after every save,
  // delete or enabled toggle; the slot rules themselves stay in Rust.
  teams();
  const backend = ensureStore();
  if (backend) {
    return backend.slots(next).map((slot: TeamSlotView) => ({
      number: slot.number,
      team: slot.team ? toTeam(slot.team) : null,
    }));
  }
  return [];
}

/** The teams outside the fixed formation numbers a filter shows. */
export function extrasFor(next: TeamFilter): Team[] {
  teams();
  const backend = ensureStore();
  return backend ? backend.extras(next).map(toTeam) : [];
}

/** The formation number of `id` (`team-4` → 4), or `null`. */
export function teamNumber(id: string): number | null {
  if (!id.startsWith("team-")) return null;
  const number = Number(id.slice("team-".length));
  return Number.isInteger(number) && number > 0 ? number : null;
}

/** Turns a team on or off through the partial `team.save` update. */
export function setTeamEnabled(id: string, enabled: boolean): void {
  const backend = ensureStore();
  if (!backend) return;
  try {
    hydrate(backend.setEnabled(id, enabled));
    feedback.set(enabled ? "队伍已启用" : "队伍已停用");
  } catch (error) {
    failure(error);
  }
}

/** Removes a team from its formation slot. */
export function deleteTeam(id: string): void {
  const backend = ensureStore();
  if (!backend) return;
  try {
    hydrate(backend.delete(id));
    feedback.set("队伍已删除");
  } catch (error) {
    failure(error);
  }
}

/** Saves a draft team; `isNew` clears the provisional slot id so the backend
 *  assigns the requested formation number. */
export function saveTeam(team: Team, isNew: boolean): boolean {
  const backend = ensureStore();
  if (!backend) {
    feedback.set("队伍数据尚未加载");
    return false;
  }
  const number = teamNumber(team.id);
  const payload = {
    ...team,
    id: isNew ? "" : team.id,
    ...(isNew && number !== null ? { teamNumber: number } : {}),
  };
  try {
    hydrate(backend.save(payload));
    feedback.set("队伍已保存");
    return true;
  } catch (error) {
    failure(error);
    return false;
  }
}

/** The line under the toolbar, when the last action left something to say. */
export function teamsFeedback(): string | null {
  return feedback();
}

/** Writes the editor's feedback line. */
export function setTeamsFeedback(message: string | null): void {
  feedback.set(message);
}

/** A blank team for the first formation slot that holds nothing. */
export function blankTeam(): Team {
  const used = new Set(allTeams().map((team) => teamNumber(team.id)));
  let number = 1;
  while (used.has(number)) number += 1;
  return {
    id: `team-${number}`,
    name: `编队 ${number}`,
    purpose: "mirror",
    accessoryScheme: "tremor",
    sinners: [],
    enabled: true,
    teamCode: "",
    mirrorConfig: defaultMirrorConfig({ team_system: 2 }),
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
  "burn",
  "bleed",
  "tremor",
  "rupture",
  "sinking",
  "poise",
  "charge",
  "slash",
  "pierce",
  "blunt",
] as const;

/** A team's purpose as a choice, for the editor's select. */
export const PURPOSES: { value: TeamPurpose; label: string }[] = [
  { value: "mirror", label: "镜牢" },
  { value: "luxcavation", label: "经验本" },
  { value: "general", label: "通用" },
];

/** A team's purpose, as the page labels it. */
export const PURPOSE_LABELS: Record<TeamPurpose, string> = {
  mirror: "镜牢",
  luxcavation: "经验本",
  general: "通用",
};

/** The accessory scheme's label, as a card's meta line shows it. */
export const SCHEME_LABELS: Record<string, string> = SYSTEM_LABELS;

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
