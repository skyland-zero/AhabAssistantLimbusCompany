// The statistics page's view math: the phase groups the chart stacks, the
// filters, the summary and the per-team rows.
//
// All of it is pure functions over the sidecar's mirror history (the payload
// `home.ts` mirrors), so the page only renders and the numbers stay
// unit-testable. "Completed" means the sidecar did not record the run as
// failed: the chart still shows failed runs, the averages and shares do not.
import type { MirrorHistoryEntry } from "../../home";
import { mirrorTimeLabel } from "../../home";

export const runLabel = mirrorTimeLabel;

/** The phase groups the chart stacks and the detail lists. */
export type PhaseKey = "battle" | "event" | "shop" | "findRoad" | "other";

/** One phase group: what it is called, and the chart series it paints. */
export type PhaseGroup = {
  key: PhaseKey;
  label: string;
  /** The mark's `fill-*` token, spelled out because classes are literal. */
  fill: string;
  /** The swatch and bar class (`bg-*`), spelled out for the same reason. */
  bar: string;
};

export const PHASE_GROUPS: PhaseGroup[] = [
  { key: "battle", label: "战斗", fill: "fill-chart-1", bar: "bg-chart-1" },
  { key: "event", label: "事件", fill: "fill-chart-2", bar: "bg-chart-2" },
  { key: "shop", label: "商店", fill: "fill-chart-3", bar: "bg-chart-3" },
  { key: "findRoad", label: "找路", fill: "fill-chart-4", bar: "bg-chart-4" },
  { key: "other", label: "其他", fill: "fill-chart-5", bar: "bg-chart-5" },
];

export type PhaseSeconds = Record<PhaseKey, number>;

/** A run's seconds per group. The small phases the record breaks out (theme
 *  packs, reward cards, E.G.O gifts, settlement) and any time it does not
 *  account for are one group, so the groups always add up to the total. */
export function phaseSeconds(run: MirrorHistoryEntry): PhaseSeconds {
  const battle = run.battleSeconds ?? 0;
  const event = run.eventSeconds ?? 0;
  const shop = run.shopSeconds ?? 0;
  const findRoad = run.findRoadSeconds ?? 0;
  const other = Math.max(0, run.totalSeconds - battle - event - shop - findRoad);
  return { battle, event, shop, findRoad, other };
}

/** The id a run's team is grouped by. */
export function teamKey(run: MirrorHistoryEntry): string {
  if (run.team?.id) return run.team.id;
  if (run.team?.number) return `team-${run.team.number}`;
  return "unknown";
}

/** The team as the page names it. */
export function teamLabel(run: MirrorHistoryEntry): string {
  const team = run.team;
  if (!team) return "未知队伍";
  const name = team.name ?? "未知队伍";
  return team.number ? `${name} #${team.number}` : name;
}

export type StatsFilter = {
  /** The newest N of the runs that pass the other filters, or "all" for the
   *  whole history (the sidecar keeps 30). */
  limit: number | "all";
  mode: "all" | "hard" | "normal";
  /** A team key, or "all". */
  team: string;
};

/** The runs the page shows, newest first (the sidecar's order): the filters
 *  pick among the whole history, then the newest `limit` of them. */
export function selectRuns(
  history: MirrorHistoryEntry[],
  filter: StatsFilter,
): MirrorHistoryEntry[] {
  const filtered = history.filter((run) => {
    if (filter.mode === "hard" && run.hardMode !== true) return false;
    if (filter.mode === "normal" && run.hardMode === true) return false;
    if (filter.team !== "all" && teamKey(run) !== filter.team) return false;
    return true;
  });
  return filter.limit === "all" ? filtered : filtered.slice(0, filter.limit);
}

export type MirrorSummary = {
  count: number;
  completed: number;
  failed: number;
  hard: number;
  /** Completed runs only, as every average and share below. */
  totalSeconds: number;
  averageSeconds: number;
  fastestSeconds: number;
  slowestSeconds: number;
};

export function summarize(runs: MirrorHistoryEntry[]): MirrorSummary {
  const completed = runs.filter((run) => run.failed !== true);
  const durations = completed.map((run) => run.totalSeconds);
  const totalSeconds = durations.reduce((sum, seconds) => sum + seconds, 0);
  return {
    count: runs.length,
    completed: completed.length,
    failed: runs.length - completed.length,
    hard: runs.filter((run) => run.hardMode === true).length,
    totalSeconds,
    averageSeconds: completed.length > 0 ? totalSeconds / completed.length : 0,
    fastestSeconds: durations.length > 0 ? Math.min(...durations) : 0,
    slowestSeconds: durations.length > 0 ? Math.max(...durations) : 0,
  };
}

export type PhaseTotal = PhaseGroup & {
  seconds: number;
  /** The group's share of the completed runs' total time, 0–1. */
  share: number;
  averageSeconds: number;
};

/** The five groups' totals over the completed runs. */
export function phaseTotals(runs: MirrorHistoryEntry[]): PhaseTotal[] {
  const completed = runs.filter((run) => run.failed !== true);
  const totals: PhaseSeconds = { battle: 0, event: 0, shop: 0, findRoad: 0, other: 0 };
  for (const run of completed) {
    const seconds = phaseSeconds(run);
    for (const phase of PHASE_GROUPS) totals[phase.key] += seconds[phase.key];
  }
  const sum = PHASE_GROUPS.reduce((total, phase) => total + totals[phase.key], 0);
  return PHASE_GROUPS.map((phase) => ({
    ...phase,
    seconds: totals[phase.key],
    share: sum > 0 ? totals[phase.key] / sum : 0,
    averageSeconds: completed.length > 0 ? totals[phase.key] / completed.length : 0,
  }));
}

export type SmallPhaseRow = { key: string; label: string; seconds: number };

/** The phases inside "其他": the sidecar names them, the chart folds them into
 *  one series, and the detail card lists them so no phase goes unmeasured. */
const SMALL_PHASES: { key: keyof MirrorHistoryEntry; label: string }[] = [
  { key: "themePackSeconds", label: "主题包" },
  { key: "rewardCardSeconds", label: "奖励卡" },
  { key: "egoGiftSeconds", label: "E.G.O 饰品" },
  { key: "settlementSeconds", label: "结算" },
  { key: "otherSeconds", label: "未分类" },
];

/** The small phases' totals over the completed runs, zero rows left out. */
export function smallPhaseTotals(runs: MirrorHistoryEntry[]): SmallPhaseRow[] {
  const completed = runs.filter((run) => run.failed !== true);
  return SMALL_PHASES.map((phase) => {
    const seconds = completed.reduce((sum, run) => {
      const value = run[phase.key];
      return sum + (typeof value === "number" ? value : 0);
    }, 0);
    return { key: String(phase.key), label: phase.label, seconds };
  }).filter((phase) => phase.seconds > 0);
}

export type TeamRow = {
  key: string;
  label: string;
  count: number;
  completed: number;
  failed: number;
  totalSeconds: number;
  averageSeconds: number;
  phases: PhaseSeconds;
};

/** One row per team, the slowest first. */
export function teamRows(runs: MirrorHistoryEntry[]): TeamRow[] {
  const rows = new Map<string, TeamRow>();
  for (const run of runs) {
    const key = teamKey(run);
    let row = rows.get(key);
    if (!row) {
      row = {
        key,
        label: teamLabel(run),
        count: 0,
        completed: 0,
        failed: 0,
        totalSeconds: 0,
        averageSeconds: 0,
        phases: { battle: 0, event: 0, shop: 0, findRoad: 0, other: 0 },
      };
      rows.set(key, row);
    }
    row.count += 1;
    if (run.failed === true) {
      row.failed += 1;
      continue;
    }
    row.completed += 1;
    row.totalSeconds += run.totalSeconds;
    const seconds = phaseSeconds(run);
    for (const phase of PHASE_GROUPS) row.phases[phase.key] += seconds[phase.key];
  }
  const list = [...rows.values()];
  for (const row of list) {
    row.averageSeconds = row.completed > 0 ? row.totalSeconds / row.completed : 0;
  }
  return list.sort((a, b) => b.totalSeconds - a.totalSeconds);
}

/** One bar of the chart: one run, oldest first so time reads left to right. */
export type ChartRow = PhaseSeconds & {
  label: string;
  failed: boolean;
  totalSeconds: number;
};

export function chartRows(runs: MirrorHistoryEntry[]): ChartRow[] {
  return [...runs].reverse().map((run) => ({
    label: runLabel(run.completedAt),
    failed: run.failed === true,
    totalSeconds: run.totalSeconds,
    ...phaseSeconds(run),
  }));
}

/** A run's length as `m:ss.cc`, the client's format; `暂无` for none. */
export function clockText(seconds: number): string {
  if (seconds <= 0) return "暂无";
  const centiseconds = Math.round(seconds * 100);
  const minutes = Math.floor(centiseconds / 6000);
  const rest = centiseconds % 6000;
  const whole = Math.floor(rest / 100);
  const fraction = rest % 100;
  return `${minutes}:${String(whole).padStart(2, "0")}.${String(fraction).padStart(2, "0")}`;
}

/** A total as `6h 42m` / `42m 31s` / `31s`: the unit that matters, then one. */
export function spanText(seconds: number): string {
  if (seconds <= 0) return "暂无";
  const whole = Math.round(seconds);
  const hours = Math.floor(whole / 3600);
  const minutes = Math.floor((whole % 3600) / 60);
  const rest = whole % 60;
  if (hours > 0) return `${hours}h ${minutes}m`;
  if (minutes > 0) return `${minutes}m ${rest}s`;
  return `${rest}s`;
}
