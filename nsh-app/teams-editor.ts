// The team editor's draft: the section being edited, every change the mirror
// tabs make, and the JSON/clipboard round-trip. The list state is in
// `teams-store.ts`; `teams.ts` re-exports both.
import { signal } from "nsh";
import {
  SYSTEM_NAMES,
  clampMirrorNumber,
  defaultMirrorConfig,
  starlightCost,
  systemIndex,
} from "./teams-mirror";
import type { TeamMirrorConfig } from "./teams-mirror";
import { blankTeam, saveTeam, setTeamsFeedback } from "./teams-store";
import type { Team, TeamPreset, TeamPurpose } from "./teams-store";
import { currentLanguage } from "./settings";
import type { LocalizedText } from "./teams-store";

/** The editor's five sections. */
export type TeamEditorTab = "basic" | "shop" | "combat" | "starlight" | "advanced";

/** The sections a purpose offers: only mirror teams have a mirror run. */
export function editorTabs(purpose: TeamPurpose): TeamEditorTab[] {
  return purpose === "mirror"
    ? ["basic", "shop", "combat", "starlight", "advanced"]
    : ["basic"];
}

/** Which string of a localized preset label the current language uses. */
export function localizedText(text: LocalizedText): string {
  return currentLanguage() === "enUs" ? text.enUs : text.zhCn;
}

/** The editor section's label. */
export const EDITOR_TAB_LABELS: Record<TeamEditorTab, string> = {
  basic: "基础编成",
  shop: "商店与合成",
  combat: "二体系与战斗",
  starlight: "开局星光",
  advanced: "观测与高级",
};

/** The draft the editor is editing, or `null` when it is closed. */
const draft = signal<Team | null>(null);

/** Whether the draft is a new formation rather than a saved one. */
const draftIsNew = signal(false);

/** The editor's active section, reset whenever the editor opens. */
const editorTab = signal<TeamEditorTab>("basic");

/** Whether the editor is creating a team (tracked). */
export function editorIsNew(): boolean {
  return draftIsNew();
}

/** The draft in the editor (tracked). */
export function editingTeam(): Team | null {
  return draft();
}

/** The editor's current section (tracked). */
export function activeEditorTab(): TeamEditorTab {
  return editorTab();
}

/** Switches the editor's section. */
export function setEditorTab(next: TeamEditorTab): void {
  editorTab.set(next);
}

/** Opens the editor on `team`, or on a new team in `number` / the first free slot. */
export function openEditor(team: Team | null, number?: number): void {
  draft.set(team ?? blankTeam(number));
  draftIsNew.set(team === null);
  editorTab.set("basic");
  setTeamsFeedback(null);
}

/** Moves a new draft to another free formation number. The default
 *  `编队 N` name follows the slot; a custom name is kept. */
export function setDraftSlot(number: number): void {
  if (!draftIsNew()) return;
  draft.update((current) => {
    if (current === null) return null;
    const previous = current.name;
    const wasDefault = previous === `编队 ${current.id.replace("team-", "")}`;
    return {
      ...current,
      id: `team-${number}`,
      name: wasDefault ? `编队 ${number}` : previous,
      queuePosition: 0,
    };
  });
}

/** Applies a built-in preset to a new draft or over an existing team. */
export function applyPreset(preset: TeamPreset, number: number, overwrite?: Team): void {
  const isNew = overwrite === undefined;
  const team = preset.team;
  draft.set({
    ...team,
    id: isNew ? `team-${number}` : overwrite.id,
    name: localizedText(preset.name) || team.name || `编队 ${number}`,
    // A preset template has no saved slot; keep the current queue membership
    // when it overwrites a mirror team, otherwise start disabled.
    enabled: isNew ? false : overwrite.enabled,
    queuePosition: isNew ? 0 : overwrite.queuePosition,
  });
  draftIsNew.set(isNew);
  editorTab.set("basic");
  setTeamsFeedback(`已载入预设：${localizedText(preset.name)}`);
}

/** The draft's current formation number, or `null`. */
export function draftTeamNumber(): number | null {
  const id = draft()?.id ?? "";
  if (!id.startsWith("team-")) return null;
  const number = Number(id.slice("team-".length));
  return Number.isInteger(number) && number > 0 ? number : null;
}

/** The draft's formation source: an imported team code or the sinner order. */
export function draftSource(): "code" | "order" {
  const team = draft();
  if (team === null) return "order";
  if (team.purpose === "luxcavation") return team.teamCode.trim() === "" ? "order" : "code";
  return team.mirrorConfig?.use_team_code === true ? "code" : "order";
}

/** Switches the draft between its team code and its click order. */
export function setDraftSource(source: "code" | "order"): void {
  if (draft()?.purpose === "luxcavation") return;
  editMirror((config) => ({ ...config, use_team_code: source === "code" }));
}

/** A light shape check for a pasted Limbus team code. */
export function looksLikeTeamCode(value: string): boolean {
  const code = value.trim();
  return code.length > 32 && /^H4sI[A-Za-z0-9+/]+={0,2}$/.test(code);
}

/** Sets the draft's mirror route profile. */
export function setDraftRoute(route: string): void {
  editMirror((config) => ({ ...config, mirror_route_profile: route }));
}

/** Closes the editor without saving. */
export function closeEditor(): void {
  draft.set(null);
}

/** Rewrites part of the draft. */
export function editTeam(change: Partial<Team>): void {
  draft.update((current) => {
    if (current === null) return null;
    const next = { ...current, ...change };
    if (next.purpose !== current.purpose) {
      // A resource team keeps its code at the top level; the other purposes
      // read it from the mirror config. Copy it across so changing purpose
      // cannot silently drop the imported formation.
      if (current.purpose === "luxcavation" && next.purpose !== "luxcavation") {
        const code = current.teamCode.trim();
        if (code !== "" && (next.mirrorConfig?.team_code ?? "").trim() === "") {
          next.mirrorConfig = {
            ...(next.mirrorConfig ?? defaultMirrorConfig()),
            team_code: code,
            use_team_code: true,
          };
        }
      }
      if (next.purpose === "luxcavation") {
        const code = next.mirrorConfig?.team_code?.trim() ?? "";
        if (code !== "") next.teamCode = code;
      }
    }
    if (next.purpose !== "mirror" && editorTab() !== "basic") editorTab.set("basic");
    return next;
  });
}

/** Rewrites part of the draft's mirror configuration, creating it if needed. */
export function editMirror(
  change: (config: TeamMirrorConfig) => TeamMirrorConfig,
): void {
  draft.update((current) =>
    current === null
      ? null
      : { ...current, mirrorConfig: change(current.mirrorConfig ?? defaultMirrorConfig()) },
  );
}

/** Sets one boolean of the mirror configuration. */
export function setMirrorBool(key: keyof TeamMirrorConfig, value: boolean): void {
  editMirror((config) => ({ ...config, [key]: value }));
}

/** Sets one numeric choice of the mirror configuration, clamped to its range. */
export function setMirrorNumber(key: keyof TeamMirrorConfig, value: number): void {
  editMirror((config) => ({ ...config, [key]: clampMirrorNumber(key, value) }));
}

/** Sets the accessory scheme, and the system index the backend reads with it. */
export function setAccessoryScheme(scheme: string): void {
  editTeam({ accessoryScheme: scheme });
  editMirror((config) => ({ ...config, team_system: systemIndex(scheme) }));
}

/** Turns a system on or off in the shop's discard set. */
export function toggleDiscardSystem(index: number): void {
  const name = SYSTEM_NAMES[index];
  if (name === undefined) return;
  editMirror((config) => ({
    ...config,
    discard_systems: {
      ...config.discard_systems,
      [name]: !config.discard_systems[name],
    },
  }));
}

/** Ignores or restores one shop floor (1–5). */
export function toggleIgnoreShop(index: number): void {
  editMirror((config) => ({
    ...config,
    ignore_shop: config.ignore_shop.map((ignored, at) =>
      at === index ? !ignored : ignored,
    ),
  }));
}

/** Sets one opening bonus to a level (0–3). */
export function setStarlightLevel(index: number, level: number): void {
  editMirror((config) => ({
    ...config,
    opening_bonus: Array.from({ length: 10 }, (_, at) =>
      at === index ? Math.min(3, Math.max(0, level)) : (config.opening_bonus[at] ?? 0),
    ),
  }));
}

/** Sets every opening bonus to the same level (0–3). */
export function setAllStarlight(level: number): void {
  const clamped = Math.min(3, Math.max(0, level));
  editMirror((config) => ({
    ...config,
    opening_bonus: Array.from({ length: 10 }, () => clamped),
  }));
}

/** The draft's opening-starlight cost (tracked). */
export function draftStarlightCost(): number {
  return starlightCost(draft()?.mirrorConfig ?? null);
}

/** Adds a gift to observe, keeping the client's three-gift limit and order. */
export function addObserveGift(gift: string): void {
  const value = gift.trim();
  if (value === "") return;
  editMirror((config) => {
    const canonical = value === "赤红纠缠的蜘蛛巢" ? "spiderweb_entangled_in_red" : value;
    if (config.observe_ego_gift_selected.includes(canonical)) return config;
    if (config.observe_ego_gift_selected.length >= 3) {
      setTeamsFeedback("最多只能观测 3 个 E.G.O 饰品");
      return config;
    }
    return {
      ...config,
      observe_ego_gift_selected: [...config.observe_ego_gift_selected, canonical],
    };
  });
}

/** Removes one gift from the observe list. */
export function removeObserveGift(gift: string): void {
  editMirror((config) => ({
    ...config,
    observe_ego_gift_selected: config.observe_ego_gift_selected.filter(
      (item) => item !== gift,
    ),
  }));
}

/** Rewrites the draft's team code, on the team and (when it has one) on the
 *  mirror config the backend reads. */
export function setDraftTeamCode(value: string): void {
  editTeam({ teamCode: value });
  if (draft()?.purpose !== "luxcavation") {
    editMirror((config) => ({ ...config, team_code: value }));
  }
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

/** Clears every sinner from the draft. */
export function clearDraftSinners(): void {
  draft.update((current) => (current === null ? null : { ...current, sinners: [] }));
}

/** Saves the draft: a new formation takes the next free number. */
export function saveDraft(): void {
  const current = draft();
  if (current === null) return;
  const name = current.name.trim();
  if (name === "") return;
  const saved: Team = {
    ...current,
    name,
    mirrorConfig: current.purpose === "luxcavation" ? null : current.mirrorConfig,
  };
  if (!saveTeam(saved, draftIsNew())) return;
  draft.set(null);
  setTeamsFeedback("队伍已保存");
}

/** The draft as the `Teams.*` payload (schema 3), pretty-printed for copy. */
export function draftJson(): string {
  const current = draft();
  if (current === null) return "";
  return JSON.stringify(
    {
      schemaVersion: 3,
      id: current.id,
      name: current.name,
      sinners: current.sinners,
      purpose: current.purpose,
      accessoryScheme: current.accessoryScheme,
      enabled: current.enabled,
      teamCode: current.teamCode,
      mirrorConfig: current.mirrorConfig,
    },
    null,
    2,
  );
}

/** Applies a pasted `TeamDetail` JSON to the draft. Returns false and writes a
 *  feedback line when the text is not a valid team. */
export function applyDraftJson(text: string): boolean {
  let value: unknown;
  try {
    value = JSON.parse(text);
  } catch {
    setTeamsFeedback("JSON 解析失败：请检查格式");
    return false;
  }
  if (value === null || typeof value !== "object") {
    setTeamsFeedback("JSON 必须是队伍对象");
    return false;
  }
  const record = value as Record<string, unknown>;
  if (typeof record.name !== "string" || record.name.trim() === "") {
    setTeamsFeedback("JSON 缺少队伍名称");
    return false;
  }
  const purpose: TeamPurpose =
    record.purpose === "luxcavation" || record.purpose === "general"
      ? record.purpose
      : "mirror";
  const sinners = Array.isArray(record.sinners)
    ? record.sinners.filter((item): item is string => typeof item === "string")
    : [];
  const mirror = record.mirrorConfig;
  const defaults = defaultMirrorConfig();
  let mirrorConfig: TeamMirrorConfig | null = null;
  if (purpose !== "luxcavation" && mirror !== null && typeof mirror === "object") {
    const candidate = mirror as Partial<TeamMirrorConfig>;
    mirrorConfig = {
      ...defaults,
      ...candidate,
      discard_systems: {
        ...defaults.discard_systems,
        ...(candidate.discard_systems ?? {}),
      },
      ignore_shop: Array.isArray(candidate.ignore_shop)
        ? defaults.ignore_shop.map((_, index) => candidate.ignore_shop?.[index] === true)
        : defaults.ignore_shop,
      opening_bonus: Array.isArray(candidate.opening_bonus)
        ? defaults.opening_bonus.map((fallback, index) =>
            Math.min(
              3,
              Math.max(0, Number(candidate.opening_bonus?.[index] ?? fallback)),
            ),
          )
        : defaults.opening_bonus,
      observe_ego_gift_selected: Array.isArray(candidate.observe_ego_gift_selected)
        ? candidate.observe_ego_gift_selected.filter(
            (item): item is string => typeof item === "string",
          )
        : [],
    };
  } else if (purpose !== "luxcavation") {
    mirrorConfig = defaults;
  }
  draft.set({
    ...(draft() ?? blankTeam()),
    name: record.name,
    purpose,
    sinners,
    accessoryScheme:
      typeof record.accessoryScheme === "string" ? record.accessoryScheme : "burn",
    enabled: record.enabled !== false,
    teamCode: typeof record.teamCode === "string" ? record.teamCode : "",
    mirrorConfig,
  });
  setTeamsFeedback("队伍 JSON 已导入");
  return true;
}

/** The id a team's stats are stored under. */
export function editorTeamId(): string | null {
  return draft()?.id ?? null;
}

/** The draft's opening starlight, for the cards the editor shows. */
export function draftOpeningBonus(): number[] {
  return draft()?.mirrorConfig?.opening_bonus ?? [];
}

/** The draft's mirror configuration, with the contract defaults when it has
 *  none yet (a new team, or one opened before its config was created). */
export function draftMirror(): TeamMirrorConfig {
  return draft()?.mirrorConfig ?? defaultMirrorConfig();
}
