// The statistics page's contract: the view math over the mirror history
// (filters, phases, teams, formatting) and the page the console's 明细 buttons
// open.
import { expect, open, test } from "nsh/test";
import {
  chartRows,
  clockText,
  phaseSeconds,
  phaseTotals,
  selectRuns,
  smallPhaseTotals,
  spanText,
  summarize,
  teamLabel,
  teamRows,
} from "./aggregate";
import { mirrorTimeLabel } from "../../home";
import type { MirrorHistoryEntry } from "../../home";

const DESKTOP = { width: 820, height: 680, platform: "windows" as const };

/** One record with the fields a view test needs; the rest stays default. */
function run(partial: Partial<MirrorHistoryEntry> & { completedAt: string }): MirrorHistoryEntry {
  return {
    totalSeconds: 1800,
    battleSeconds: 1200,
    eventSeconds: 200,
    shopSeconds: 100,
    findRoadSeconds: 300,
    hardMode: true,
    team: { id: "team-1", name: "编队 1", number: 1 },
    ...partial,
  };
}

const HISTORY: MirrorHistoryEntry[] = [
  run({
    completedAt: "2026-08-31T21:00:00+08:00",
    totalSeconds: 1500,
    battleSeconds: 1000,
    eventSeconds: 150,
    shopSeconds: 80,
    findRoadSeconds: 270,
    hardMode: false,
    team: { id: "team-2", name: "编队 2", number: 2 },
  }),
  run({ completedAt: "2026-08-31T19:00:00+08:00", totalSeconds: 2000, battleSeconds: 1300, eventSeconds: 200, shopSeconds: 100, findRoadSeconds: 400 }),
  run({ completedAt: "2026-08-30T22:00:00+08:00", totalSeconds: 900, battleSeconds: 700, eventSeconds: 90, shopSeconds: 40, findRoadSeconds: 70, failed: true }),
];

test("the phase groups always add up to the run's total", () => {
  for (const entry of HISTORY) {
    const seconds = phaseSeconds(entry);
    const sum = seconds.battle + seconds.event + seconds.shop + seconds.findRoad + seconds.other;
    expect(Math.round(sum * 100)).toBe(Math.round(entry.totalSeconds * 100));
  }
});

test("the filters keep the newest runs, the difficulty and the team", () => {
  expect(selectRuns(HISTORY, { limit: 2, mode: "all", team: "all" }).length).toBe(2);
  expect(selectRuns(HISTORY, { limit: "all", mode: "normal", team: "all" }).length).toBe(1);
  expect(selectRuns(HISTORY, { limit: "all", mode: "all", team: "team-1" }).length).toBe(2);
  expect(selectRuns(HISTORY, { limit: "all", mode: "hard", team: "team-2" }).length).toBe(0);
});

test("the summary averages the completed runs only", () => {
  const summary = summarize(HISTORY);
  expect(summary.count).toBe(3);
  expect(summary.completed).toBe(2);
  expect(summary.failed).toBe(1);
  expect(summary.totalSeconds).toBe(3500);
  expect(summary.averageSeconds).toBe(1750);
  expect(summary.fastestSeconds).toBe(1500);
  expect(summary.slowestSeconds).toBe(2000);
});

test("the phase shares cover the completed runs' time", () => {
  const phases = phaseTotals(HISTORY);
  expect(phases.length).toBe(5);
  const battle = phases.find((phase) => phase.key === "battle");
  expect(battle?.seconds).toBe(2300);
  expect(Math.round((battle?.share ?? 0) * 100)).toBe(66);
  const share = phases.reduce((sum, phase) => sum + phase.share, 0);
  expect(Math.round(share * 100)).toBe(100);
});

test("teams are grouped and sorted by their completed time", () => {
  const teams = teamRows(HISTORY);
  expect(teams.map((team) => team.label)).toEqual(["编队 1 #1", "编队 2 #2"]);
  expect(teams[0].count).toBe(2);
  expect(teams[0].completed).toBe(1);
  expect(teams[0].averageSeconds).toBe(2000);
  expect(teamLabel(HISTORY[0])).toBe("编队 2 #2");
});

test("the phases inside 其他 are listed too", () => {
  const small = smallPhaseTotals([
    run({
      completedAt: "2026-08-31T21:00:00+08:00",
      themePackSeconds: 30,
      rewardCardSeconds: 15,
      egoGiftSeconds: 10,
      settlementSeconds: 20,
      otherSeconds: 5,
    }),
    run({ completedAt: "2026-08-31T22:00:00+08:00", failed: true, themePackSeconds: 99 }),
  ]);
  expect(small.map((phase) => phase.label)).toEqual([
    "主题包",
    "奖励卡",
    "E.G.O 饰品",
    "结算",
    "未分类",
  ]);
  // A failed run's seconds stay out of the completed totals.
  expect(small[0].seconds).toBe(30);
});

test("the chart lists the runs oldest first", () => {
  const rows = chartRows(HISTORY);
  expect(rows.length).toBe(3);
  expect(rows[0].label).toBe(mirrorTimeLabel(HISTORY[2].completedAt));
  expect(rows[2].label).toBe(mirrorTimeLabel(HISTORY[0].completedAt));
  expect(rows[0].failed).toBe(true);
});

test("durations keep the client's formats", () => {
  expect(clockText(2013.5)).toBe("33:33.50");
  expect(clockText(0)).toBe("暂无");
  expect(spanText(24120)).toBe("6h 42m");
  expect(spanText(2551)).toBe("42m 31s");
  expect(spanText(31)).toBe("31s");
});

test("the title bar's chart button opens the statistics page", async () => {
  const app = await open("../../App.tsx", DESKTOP);
  await app.press("role:button[统计]");
  await app.waitFor("最近镜牢用时");
  await app.waitFor("每日次数");
  await app.waitFor("编队 1 (震颤) #1");
  await app.waitFor("最近 6 场");
});

test("the page's filters narrow the summary", async () => {
  const app = await open("../../App.tsx", DESKTOP);
  await app.press("role:button[统计]");
  await app.waitFor("最近 6 场");
  await app.press("role:button[困难]");
  await app.waitFor("最近 4 场");
  await app.press("role:button[普通]");
  await app.waitFor("最近 2 场");
  await app.press("role:button[全部] >> nth=1");
  await app.waitFor("最近 6 场");
  await app.press("role:button[5 次]");
  await app.waitFor("最近 5 场");
});
