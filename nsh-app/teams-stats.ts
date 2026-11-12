// A team's accumulated mirror-run statistics.
//
// The shape is the contract's `TeamStats` (`TEAM_STATS_GET` /
// `TEAM_STATS_CLEAR`). P1 replaces the fixture below with the RPC's answer; the
// editor keeps reading the same functions.
import { signal } from "nsh";
import { setTeamsFeedback } from "./teams";

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

/** The stats the mock backend has for a team. */
const MOCK: TeamStats = {
  totalCount: 6,
  hard: {
    count: 3,
    averageSeconds: 120.5,
    last5AverageSeconds: 118.2,
    last10AverageSeconds: 121.4,
  },
  normal: {
    count: 3,
    averageSeconds: 95,
    last5AverageSeconds: 94.8,
    last10AverageSeconds: 96.1,
  },
};

const EMPTY: TeamStats = {
  totalCount: 0,
  hard: EMPTY_BUCKET,
  normal: EMPTY_BUCKET,
};

const stats = signal<Record<string, TeamStats>>({});
const clearRequested = signal(false);

/** A team's stats (tracked); an unloaded team has zeroes, like the client
 *  before its first `TEAM_STATS_GET` answer. */
export function teamStats(id: string | null): TeamStats {
  return (id === null ? undefined : stats()[id]) ?? EMPTY;
}

/** Whether the clear-history confirmation is open (tracked). */
export function statsClearRequested(): boolean {
  return clearRequested();
}

/** Loads the team's stats. In the mock this is the only load there is. */
export function refreshTeamStats(id: string | null): void {
  if (id === null) return;
  stats.update((all) => ({ ...all, [id]: MOCK }));
  setTeamsFeedback(null);
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
  stats.update((all) => ({ ...all, [id]: EMPTY }));
  setTeamsFeedback("队伍统计已清空");
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
