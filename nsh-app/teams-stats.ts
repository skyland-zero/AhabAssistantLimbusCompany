// A team's accumulated mirror-run statistics.
//
// Rust owns the `TEAM_STATS_GET` / `TEAM_STATS_CLEAR` calls; this module mirrors
// the answer into signals and formats durations for the advanced tab.
import { signal } from "nsh";
import { setTeamsFeedback, teamsBackend } from "./teams";

export type TeamStatsBucket = {
  count: number;
  averageSeconds: number;
  last5AverageSeconds: number;
  last10AverageSeconds: number;
};

export type TeamStats = {
  totalCount: number;
  hard: TeamStatsBucket;
  normal: TeamStatsBucket;
};

const EMPTY_BUCKET: TeamStatsBucket = {
  count: 0,
  averageSeconds: 0,
  last5AverageSeconds: 0,
  last10AverageSeconds: 0,
};

const EMPTY: TeamStats = {
  totalCount: 0,
  hard: EMPTY_BUCKET,
  normal: EMPTY_BUCKET,
};

const stats = signal<Record<string, TeamStats>>({});
const clearRequested = signal(false);

function bucket(value: Partial<TeamStatsBucket> | undefined): TeamStatsBucket {
  return {
    count: value?.count ?? 0,
    averageSeconds: value?.averageSeconds ?? 0,
    last5AverageSeconds: value?.last5AverageSeconds ?? 0,
    last10AverageSeconds: value?.last10AverageSeconds ?? 0,
  };
}

function toStats(value: unknown): TeamStats {
  const payload = value as { totalCount?: number; hard?: Partial<TeamStatsBucket>; normal?: Partial<TeamStatsBucket> };
  return {
    totalCount: payload.totalCount ?? 0,
    hard: bucket(payload.hard),
    normal: bucket(payload.normal),
  };
}

function failure(error: unknown): void {
  setTeamsFeedback(error instanceof Error ? error.message : String(error));
}

/** A team's stats (tracked); an unloaded team has zeroes, like the client
 *  before its first `TEAM_STATS_GET` answer. */
export function teamStats(id: string | null): TeamStats {
  return (id === null ? undefined : stats()[id]) ?? EMPTY;
}

/** Whether the clear-history confirmation is open (tracked). */
export function statsClearRequested(): boolean {
  return clearRequested();
}

/** Loads the team's stats from the backend. */
export function refreshTeamStats(id: string | null): void {
  if (id === null) return;
  const backend = teamsBackend();
  if (!backend) return;
  try {
    const value = toStats(backend.stats(id));
    stats.update((all) => ({ ...all, [id]: value }));
    setTeamsFeedback(null);
  } catch (error) {
    failure(error);
  }
}

/** Opens the confirmation for clearing a team's history. */
export function requestClearTeamStats(id: string | null): void {
  if (id === null) return;
  clearRequested.set(true);
}

/** Closes the confirmation without clearing. */
export function cancelClearTeamStats(): void {
  clearRequested.set(false);
}

/** Clears the team's history after the confirmation. */
export function clearTeamStats(id: string | null): void {
  clearRequested.set(false);
  if (id === null) return;
  const backend = teamsBackend();
  if (!backend) return;
  try {
    const value = toStats(backend.clearStats(id));
    stats.update((all) => ({ ...all, [id]: value }));
    setTeamsFeedback("队伍统计已清空");
  } catch (error) {
    failure(error);
  }
}

/** A duration as `m:ss.cc`, or `暂无` when there is none. */
export function durationLabel(seconds: number): string {
  if (seconds <= 0) return "暂无";
  const centiseconds = Math.round(seconds * 100);
  const minutes = Math.floor(centiseconds / 6000);
  const rest = centiseconds % 6000;
  const whole = Math.floor(rest / 100);
  const fraction = rest % 100;
  return `${minutes}:${String(whole).padStart(2, "0")}.${String(fraction).padStart(2, "0")}`;
}
