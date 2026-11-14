// The saved teams: Rust owns the list, the formation slots and every
// `team.*` mutation. This module mirrors the returned arrays into signals and
// keeps the labels the list shows. The editor's draft stays in
// `teams-editor.ts`; both are re-exported by `teams.ts`.
import { signal } from "nsh";
import { Backend, TeamsStore } from "aalc/native";
import type { LocalizedTextView, TeamPresetView, TeamSlotView, TeamView } from "aalc/native";
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
  /** 1-based mirror rotation position, or 0 when the team is not queued. */
  queuePosition: number;
  /** The mirror-run configuration; `null` for a resource team. */
  mirrorConfig: TeamMirrorConfig | null;
};

/** One localized preset string. */
export type LocalizedText = LocalizedTextView;

/** One built-in team template. */
export type TeamPreset = {
  presetId: string;
  routeId: string;
  name: LocalizedText;
  description: LocalizedText;
  floorHint: LocalizedText;
  routeName: LocalizedText;
  team: Team;
};

/** The filter above the saved-team list: all teams or one purpose. */
export type TeamFilter = "all" | TeamPurpose;

/** The saved teams, mirrored from the Rust store. */
export const teams = signal<Team[]>([]);

/** The built-in templates, mirrored from `team.preset.list`. */
export const presets = signal<TeamPreset[]>([]);

/** `true` selects the physical 1–40 row instead of OCRing the in-game name. */
export const selectionMode = signal(false);

/** Whether the first sidecar load is still in flight. */
export const teamsLoading = signal(false);

/** One delete confirmation target, or `null` when the dialog is closed. */
export const deleteTarget = signal<Team | null>(null);

const filter = signal<TeamFilter>("all");
const feedback = signal<string | null>(null);
const feedbackTone = signal<"info" | "success" | "warning" | "danger">("success");

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
    queuePosition: view.queuePosition,
    mirrorConfig: view.mirrorConfig as TeamMirrorConfig | null,
  };
}

function toPreset(view: TeamPresetView): TeamPreset {
  return {
    presetId: view.presetId,
    routeId: view.routeId,
    name: view.name,
    description: view.description,
    floorHint: view.floorHint,
    routeName: view.routeName,
    team: toTeam(view.team),
  };
}

function hydrate(list: TeamView[]): void {
  teams.set(list.map(toTeam));
}

function hydratePresets(list: TeamPresetView[]): void {
  presets.set(list.map(toPreset));
}

function failure(error: unknown): void {
  feedbackTone.set("danger");
  feedback.set(error instanceof Error ? error.message : String(error));
}

/** The Rust store once it is loaded, or `null` while a sidecar connects. */
function ensureStore(): TeamsStore | null {
  if (store) return store;
  if (Backend.mode() === "mock") {
    try {
      store = new TeamsStore();
      const error = store.error();
      if (error) {
        failure(error);
      } else {
        hydrate(store.teams());
        hydratePresets(store.presets());
        selectionMode.set(store.selection());
      }
    } catch (error) {
      failure(error);
    }
  } else if (!connecting) {
    connecting = true;
    teamsLoading.set(true);
    TeamsStore.connect()
      .then((created) => {
        store = created;
        hydrate(created.teams());
        hydratePresets(created.presets());
        selectionMode.set(created.selection());
      })
      .catch(failure)
      .finally(() => {
        connecting = false;
        teamsLoading.set(false);
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

/** Every game loadout number (1–40), with the team saved there, if any. */
export function allSlots(): { number: number; team: Team | null }[] {
  // Reading the mirrored list keeps this binding reactive after every save,
  // delete or enabled toggle; the slot rules themselves stay in Rust.
  teams();
  const backend = ensureStore();
  return backend
    ? backend.slots().map((slot: TeamSlotView) => ({
        number: slot.number,
        team: slot.team ? toTeam(slot.team) : null,
      }))
    : [];
}

/** Legacy teams whose number is outside the forty game loadouts. */
export function extraTeams(): Team[] {
  teams();
  const backend = ensureStore();
  return backend ? backend.extras().map(toTeam) : [];
}

/** The numbers with no team configured in AALC.nsh. */
export function freeSlots(): { value: string; label: string }[] {
  return allSlots()
    .filter((slot) => slot.team === null)
    .map((slot) => ({
      value: String(slot.number),
      label: `#${String(slot.number).padStart(2, "0")} 空位`,
    }));
}

/** The mirror queue, in execution order. */
export function queueTeams(): Team[] {
  return teams()
    .filter((team) => team.queuePosition > 0)
    .sort((left, right) => left.queuePosition - right.queuePosition);
}

/** The formation number of `id` (`team-4` → 4), or `null`. */
export function teamNumber(id: string): number | null {
  if (!id.startsWith("team-")) return null;
  const number = Number(id.slice("team-".length));
  return Number.isInteger(number) && number > 0 ? number : null;
}

/** Where the team's formation comes from: an imported code, the click order,
 *  or neither (a newly created team that has not been filled yet). */
export function teamSource(team: Team): "code" | "order" | "empty" {
  const code =
    team.purpose === "luxcavation" ? team.teamCode : (team.mirrorConfig?.team_code ?? "");
  const codeEnabled =
    team.purpose === "luxcavation" ? code.trim() !== "" : team.mirrorConfig?.use_team_code === true;
  if (codeEnabled) return "code";
  return team.sinners.length > 0 ? "order" : "empty";
}

/** The code the team's source uses, without deciding which field owns it. */
export function teamCodeValue(team: Team): string {
  return team.purpose === "luxcavation"
    ? team.teamCode
    : (team.mirrorConfig?.team_code ?? "");
}

/** Turns a mirror team on or off through the partial `team.save` update. */
export function setTeamEnabled(id: string, enabled: boolean): void {
  const backend = ensureStore();
  if (!backend) return;
  try {
    hydrate(backend.setEnabled(id, enabled));
    feedbackTone.set("success");
    feedback.set(enabled ? "队伍已加入镜牢轮换" : "队伍已移出镜牢轮换");
  } catch (error) {
    failure(error);
  }
}

/** Switches between OCR-by-name and physical-row selection. */
export function setSelectionMode(enabled: boolean): void {
  const backend = ensureStore();
  if (!backend) return;
  try {
    selectionMode.set(backend.setSelection(enabled));
    feedbackTone.set("success");
    feedback.set(
      enabled
        ? "已改为按序号选队：游戏内编队顺序需要保持 1–40"
        : "已改为按队名选队：游戏内队名需要保持 编队#N",
    );
  } catch (error) {
    failure(error);
  }
}

/** Asks for confirmation before removing a team and its stored config. */
export function requestDeleteTeam(id: string): void {
  deleteTarget.set(allTeams().find((team) => team.id === id) ?? null);
}

/** Closes the delete confirmation without changing anything. */
export function cancelDeleteTeam(): void {
  deleteTarget.set(null);
}

/** Confirms the pending delete. */
export function confirmDeleteTeam(): void {
  const target = deleteTarget();
  deleteTarget.set(null);
  if (target === null) return;
  const backend = ensureStore();
  if (!backend) return;
  try {
    hydrate(backend.delete(target.id));
    feedbackTone.set("success");
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
    feedbackTone.set("success");
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

/** The tone of the last feedback line. */
export function teamsFeedbackTone(): "info" | "success" | "warning" | "danger" {
  return feedbackTone();
}

/** Writes the editor's feedback line. */
export function setTeamsFeedback(
  message: string | null,
  tone: "info" | "success" | "warning" | "danger" = "success",
): void {
  feedbackTone.set(tone);
  feedback.set(message);
}

/** A blank team in `number`, or in the first formation slot that is free. */
export function blankTeam(number?: number): Team {
  const used = new Set(allTeams().map((team) => teamNumber(team.id)));
  let slot = number ?? 1;
  while (used.has(slot)) slot += 1;
  return {
    id: `team-${slot}`,
    name: `编队 ${slot}`,
    purpose: "mirror",
    accessoryScheme: "tremor",
    sinners: [],
    enabled: true,
    teamCode: "",
    queuePosition: 0,
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
  { value: "luxcavation", label: "资源本" },
  { value: "general", label: "通用（手动）" },
];

/** A team's purpose, as the page labels it. */
export const PURPOSE_LABELS: Record<TeamPurpose, string> = {
  mirror: "镜牢",
  luxcavation: "资源本",
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
