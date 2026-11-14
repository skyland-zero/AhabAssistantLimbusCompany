// The statistics page: the console's mirror runs, by time.
//
// The sidecar keeps the last 30 records (phase seconds, team, difficulty,
// route) and persists them, so this page is a view over `home.ts`'s payload:
// a summary strip, the recent runs as stacked bars, the phase split, the teams
// and the daily counts.
import { Scroll, Text, View, effect, signal } from "nsh";
import { Bar, Grid, Plot, PlotAxis } from "nsh/base/plot";
import { Badge } from "../../components/Badge";
import { Button } from "../../components/Button";
import { Card } from "../../components/Card";
import { EmptyState } from "../../components/EmptyState";
import { Select } from "../../components/Select";
import { Segmented } from "../../components/Segmented";
import { PageRoot, PageToolbar } from "../../components/Page";
import { ENTER_EASING, MS, MOVE_EASING } from "../../motion";
import {
  currentDailyStats,
  currentStatsDetailsError,
  loadDailyStats,
  pollMirrorHistory,
} from "../../home";
import type { DailyStatEntry, MirrorHistoryEntry } from "../../home";
import {
  PHASE_GROUPS,
  chartRows,
  clockText,
  phaseTotals,
  runLabel,
  selectRuns,
  smallPhaseTotals,
  spanText,
  summarize,
  teamLabel,
  teamRows,
} from "./aggregate";
import type { ChartRow, PhaseSeconds } from "./aggregate";

/** The newest runs the page offers; the sidecar keeps 30 in all. */
const RANGES = [
  { value: "5", label: "5 次" },
  { value: "10", label: "10 次" },
  { value: "all", label: "全部" },
];

const MODES = [
  { value: "all", label: "全部" },
  { value: "hard", label: "困难" },
  { value: "normal", label: "普通" },
];

export function Stats() {
  const history = signal<MirrorHistoryEntry[]>([]);
  const range = signal("10");
  const mode = signal("all");
  const team = signal("all");
  const hovered = signal<number | null>(null);

  const reload = () => {
    history.set(pollMirrorHistory());
    loadDailyStats();
  };

  // Poll while the page is on screen, as the console does: a run that finishes
  // in the background shows up without leaving the page. The poll is also what
  // drains the event queue here.
  effect(() => {
    reload();
    const timer = setInterval(() => history.set(pollMirrorHistory()), 1000);
    return () => clearInterval(timer);
  });

  const runs = () =>
    selectRuns(history(), {
      limit: range() === "all" ? "all" : Number(range()),
      mode: mode() as "all" | "hard" | "normal",
      team: team(),
    });
  const summary = () => summarize(runs());
  const chart = () => chartRows(runs());
  const hoveredRow = () => {
    const index = hovered();
    const rows = chart();
    return index !== null && index >= 0 && index < rows.length ? rows[index] : null;
  };
  const teamOptions = () => {
    const seen = new Map<string, string>();
    for (const run of history()) {
      const key = run.team?.id ?? (run.team?.number ? `team-${run.team.number}` : "unknown");
      if (!seen.has(key)) seen.set(key, teamLabel(run));
    }
    return [{ value: "all", label: "全部队伍" }, ...[...seen].map(([value, label]) => ({ value, label }))];
  };

  return (
    <PageRoot>
      <PageToolbar>
        <View class="w-full flex-row flex-wrap items-center justify-between gap-3">
          <View class="flex-row flex-wrap items-center gap-2">
            <Text class="text-xs text-muted-foreground">最近</Text>
            <Segmented
              options={RANGES}
              value={range}
              onChange={(next) => range.set(next)}
              accessibilityLabel="统计范围"
            />
            <Text class="text-xs text-muted-foreground">难度</Text>
            <Segmented
              options={MODES}
              value={mode}
              onChange={(next) => mode.set(next)}
              accessibilityLabel="难度筛选"
            />
            <Select<string>
              items={teamOptions()}
              value={team()}
              onChange={(next) => team.set(next)}
              accessibilityLabel="队伍筛选"
              class="w-40"
            />
          </View>
          <View class="flex-row items-center gap-2">
            <Text class="text-xs text-muted-foreground">
              {`${summary().count} 场 · 完成 ${summary().completed}`}
            </Text>
            <Button label="刷新" icon="refresh" variant="ghost" onPress={reload} />
          </View>
        </View>
      </PageToolbar>

      <Scroll testID="stats-scroll" class="min-h-0 flex-1">
        <View class="w-full flex-col gap-2 pb-2">
          <If when={history().length === 0}>
            <EmptyState
              title="暂无镜牢记录"
              detail="完成一次镜牢后，这里会显示每次的用时、环节拆分和队伍统计。"
            />
          </If>
          <If when={history().length > 0}>
            <SummaryStrip
              count={summary().count}
              completed={summary().completed}
              failed={summary().failed}
              hard={summary().hard}
              totalSeconds={summary().totalSeconds}
              averageSeconds={summary().averageSeconds}
              fastestSeconds={summary().fastestSeconds}
              slowestSeconds={summary().slowestSeconds}
            />
            <ChartCard
              rows={chart()}
              generation={`${range()}|${mode()}|${team()}`}
              hovered={hoveredRow()}
              onHover={(index) => hovered.set(index)}
            />
            <View class="w-full flex-row items-stretch gap-2">
              <PhaseCard runs={runs()} />
              <TeamCard runs={runs()} />
            </View>
            <RecentCard runs={runs()} />
          </If>
          <DailyCard />
        </View>
      </Scroll>
    </PageRoot>
  );
}

/** The KPI strip: counts, the totals and the averages, on one line each half. */
function SummaryStrip(props: {
  count: number;
  completed: number;
  failed: number;
  hard: number;
  totalSeconds: number;
  averageSeconds: number;
  fastestSeconds: number;
  slowestSeconds: number;
}) {
  return (
    <Card class="w-full p-2.5" arrive={0}>
      <View class="w-full flex-col gap-1">
        <View class="w-full flex-row flex-wrap items-center gap-x-4 gap-y-1">
          <Chip label="场次" value={String(props.count)} />
          <Chip label="完成" value={`${props.completed} / ${props.count}`} />
          <Chip label="困难" value={String(props.hard)} />
          <Chip label="未完成" value={String(props.failed)} />
          <Chip label="完成总用时" value={spanText(props.totalSeconds)} />
        </View>
        <View class="w-full flex-row flex-wrap items-center gap-x-4 gap-y-1">
          <Chip label="平均" value={clockText(props.averageSeconds)} />
          <Chip label="最快" value={clockText(props.fastestSeconds)} />
          <Chip label="最慢" value={clockText(props.slowestSeconds)} />
        </View>
      </View>
    </Card>
  );
}

function Chip(props: { label: string; value: string }) {
  return (
    <View class="flex-none flex-row items-center gap-1">
      <Text class="text-xs text-muted-foreground">{props.label}</Text>
      <Text class="text-xs text-foreground">{props.value}</Text>
    </View>
  );
}

/** The recent runs as one stacked bar each, oldest on the left. */
function ChartCard(props: {
  rows: ChartRow[];
  /** What the bars are: a change replays their draw-in, a data tick does not. */
  generation: string;
  hovered: ChartRow | null;
  onHover: (index: number | null) => void;
}) {
  return (
    <Card class="w-full p-3" arrive={1}>
      <View class="w-full flex-col gap-2">
        <View class="w-full flex-row items-center justify-between gap-2">
          <Text class="text-xs font-medium text-foreground">最近镜牢用时（按环节）</Text>
          <View class="flex-row flex-wrap items-center gap-2">
            <Each of={PHASE_GROUPS}>
              {(phase) => <LegendItem phase={phase()} />}
            </Each>
          </View>
        </View>
        <If
          when={props.rows.length > 0}
          else={<Text class="py-8 text-center text-xs text-muted-foreground">没有符合条件的记录</Text>}
        >
          {/* The y axis writes its labels outside the plot, so the shell leaves it
              that room. */}
          <View class="w-full ps-7">
            <Plot
              data={props.rows}
              xScale={{ type: "band", paddingInner: 0.28, paddingOuter: 0.08 }}
              yScale={{ type: "linear" }}
              class="h-52 w-full"
              motion={{
                appear: { duration: MS.slow, easing: ENTER_EASING },
                update: { duration: MS.base, easing: MOVE_EASING },
              }}
              appearGeneration={props.generation}
              onHoverChange={({ index }) => props.onHover(index)}
              tooltipContent={<ChartTooltip row={props.hovered} />}
              crossLineClass="w-px bg-input"
              accessibilityLabel="最近几次镜牢的用时，按环节堆叠"
            >
              <Grid class="stroke-input" />
              <Bar stack="phases" value="battle" class="fill-chart-1" stagger={0.35} />
              <Bar stack="phases" value="event" class="fill-chart-2" stagger={0.35} />
              <Bar stack="phases" value="shop" class="fill-chart-3" stagger={0.35} />
              <Bar stack="phases" value="findRoad" class="fill-chart-4" stagger={0.35} />
              <Bar
                stack="phases"
                value="other"
                class="fill-chart-5"
                stagger={0.35}
                label={(row: ChartRow) => (row.failed ? "未完成" : "")}
                labelClass="text-[10px] text-danger"
              />
              <PlotAxis
                placement="left"
                ticks={4}
                format={(value) => `${Math.round(value / 60)}m`}
                labelClass="text-[10px] text-muted-foreground"
                tickClass="h-px w-1.5 bg-input"
                lineClass="w-px bg-input"
              />
              <PlotAxis
                placement="bottom"
                ticks={Math.min(props.rows.length, 5)}
                format={(value) => props.rows[Math.round(value)]?.label ?? ""}
                labelClass="text-[10px] text-muted-foreground"
                tickClass="h-1.5 w-px bg-input"
                lineClass="h-px bg-input"
              />
            </Plot>
          </View>
        </If>
        <Text class="pt-2 text-[10px] text-muted-foreground">
          平均与占比只算完成场次；图表含未完成场次，其柱上标「未完成」。
        </Text>
      </View>
    </Card>
  );
}

function LegendItem(props: { phase: { label: string; bar: string } }) {
  return (
    <View class="flex-row items-center gap-1">
      <View class={`size-2.5 flex-none rounded-sm ${props.phase.bar}`} />
      <Text class="text-[10px] text-muted-foreground">{props.phase.label}</Text>
    </View>
  );
}

function ChartTooltip(props: { row: ChartRow | null }) {
  return (
    <If when={props.row}>
      {(row) => (
        <View class="flex-col gap-1 rounded-(--radius) border border-border bg-popover px-2 py-1.5 shadow-lg">
          <View class="flex-row items-center gap-2">
            <Text class="text-xs font-medium text-popover-foreground">{row().label}</Text>
            <If when={row().failed}>
              <Badge label="未完成" tone="danger" />
            </If>
            <Text class="font-mono text-xs text-brand">{clockText(row().totalSeconds)}</Text>
          </View>
          <Each of={PHASE_GROUPS}>
            {(phase) => (
              <View class="flex-row items-center gap-1.5">
                <View class={`size-2 flex-none rounded-sm ${phase().bar}`} />
                <Text class="w-10 flex-none text-[10px] text-muted-foreground">{phase().label}</Text>
                <Text class="font-mono text-[10px] text-foreground">
                  {clockText(row()[phase().key])}
                </Text>
              </View>
            )}
          </Each>
        </View>
      )}
    </If>
  );
}

/** The phase split over the completed runs. */
function PhaseCard(props: { runs: MirrorHistoryEntry[] }) {
  return (
    <Card class="min-w-0 flex-1 p-3" arrive={2}>
      <View class="w-full flex-col gap-2">
        <Text class="text-xs font-medium text-foreground">环节用时</Text>
        <Each of={phaseTotals(props.runs)}>
          {(phase) => (
            <View class="w-full flex-col gap-1">
              <View class="w-full flex-row items-center gap-2">
                <View class={`size-2.5 flex-none rounded-sm ${phase().bar}`} />
                <Text class="w-12 flex-none text-xs text-muted-foreground">{phase().label}</Text>
                <Text class="min-w-0 flex-1 text-right font-mono text-xs text-foreground">
                  {spanText(phase().seconds)}
                </Text>
                <Text class="w-10 flex-none text-right text-xs text-muted-foreground">
                  {`${Math.round(phase().share * 100)}%`}
                </Text>
              </View>
              <View class="h-1.5 w-full overflow-hidden rounded-full bg-muted">
                <View
                  class={`h-full rounded-full ${phase().bar}`}
                  style={{ width: `${Math.round(phase().share * 100)}%` }}
                />
              </View>
            </View>
          )}
        </Each>
        <View class="w-full flex-row flex-wrap items-center gap-x-3 gap-y-1 pt-1">
          <Each of={smallPhaseTotals(props.runs)}>
            {(phase) => (
              <Text class="text-[10px] text-muted-foreground">
                {`${phase().label} ${spanText(phase().seconds)}`}
              </Text>
            )}
          </Each>
        </View>
      </View>
    </Card>
  );
}

/** One row per team: runs, average and its own phase split. */
function TeamCard(props: { runs: MirrorHistoryEntry[] }) {
  return (
    <Card class="min-w-0 flex-1 p-3" arrive={3}>
      <View class="w-full flex-col gap-2">
        <Text class="text-xs font-medium text-foreground">队伍</Text>
        <If
          when={teamRows(props.runs).length > 0}
          else={<Text class="py-4 text-center text-xs text-muted-foreground">没有符合条件的队伍</Text>}
        >
          <Each of={teamRows(props.runs)}>
            {(team) => (
              <View class="w-full flex-col gap-1">
                <View class="w-full flex-row items-center gap-2">
                  <Text class="min-w-0 flex-1 truncate text-xs text-foreground">{team().label}</Text>
                  <Text class="flex-none text-xs text-muted-foreground">{`${team().count} 场`}</Text>
                  <Text class="flex-none font-mono text-xs text-foreground">
                    {clockText(team().averageSeconds)}
                  </Text>
                </View>
                <View class="h-1.5 w-full flex-row overflow-hidden rounded-full bg-muted">
                  <Each of={PHASE_GROUPS}>
                    {(phase) => <PhaseSegment phase={phase()} seconds={team().phases} />}
                  </Each>
                </View>
              </View>
            )}
          </Each>
        </If>
      </View>
    </Card>
  );
}

/** One team bar's segment: its width is the group's share of the team's time. */
function PhaseSegment(props: { phase: { key: keyof PhaseSeconds; bar: string }; seconds: PhaseSeconds }) {
  const total = () =>
    PHASE_GROUPS.reduce((sum, phase) => sum + props.seconds[phase.key], 0);
  const share = () => {
    const sum = total();
    return sum > 0 ? Math.round((props.seconds[props.phase.key] / sum) * 100) : 0;
  };
  return <View class={`h-full ${props.phase.bar}`} style={{ width: `${share()}%` }} />;
}

/** The newest runs, newest first, with their team and result. */
function RecentCard(props: { runs: MirrorHistoryEntry[] }) {
  return (
    <Card class="w-full p-3" arrive={4}>
      <View class="w-full flex-col gap-2">
        <Text class="text-xs font-medium text-foreground">最近记录</Text>
        <Each of={props.runs}>
          {(run) => (
            <View class="w-full flex-row flex-wrap items-center gap-2 rounded-(--radius-sm) border border-border px-2 py-1.5">
              <Text class="w-24 flex-none font-mono text-xs text-muted-foreground">
                {runLabel(run().completedAt)}
              </Text>
              <Text class="min-w-0 flex-1 truncate text-xs text-foreground">{teamLabel(run())}</Text>
              <Badge
                label={run().hardMode ? "困难" : "普通"}
                tone={run().hardMode ? "warning" : "neutral"}
              />
              <If when={run().floorCount}>
                {(floors) => (
                  <Text class="flex-none text-xs text-muted-foreground">{`${floors()} 层`}</Text>
                )}
              </If>
              <Text class="flex-none font-mono text-xs text-brand">{clockText(run().totalSeconds)}</Text>
              <Badge
                label={run().failed ? "未完成" : "已完成"}
                tone={run().failed ? "danger" : "success"}
              />
            </View>
          )}
        </Each>
      </View>
    </Card>
  );
}

/** The daily counts the console's summary no longer opens a dialog for. */
function DailyCard() {
  const data = () => currentDailyStats();
  const error = () => currentStatsDetailsError();
  return (
    <Card class="w-full p-3" arrive={5}>
      <View class="w-full flex-col gap-2">
        <View class="w-full flex-row items-center justify-between gap-2">
          <Text class="text-xs font-medium text-foreground">每日次数</Text>
          <If when={data()}>
            {(stats) => (
              <Text class="text-[10px] text-muted-foreground">
                {stats().dateFrom && stats().dateTo
                  ? `${stats().dateFrom} – ${stats().dateTo}`
                  : "每日完成"}
              </Text>
            )}
          </If>
        </View>
        <If when={data()} else={<Text class="py-4 text-center text-xs text-muted-foreground">{error() ?? "正在加载每日统计…"}</Text>}>
          {(stats) => (
            <If
              when={stats().days.length > 0}
              else={<Text class="py-4 text-center text-xs text-muted-foreground">暂无每日数据</Text>}
            >
              <View class="w-full flex-col gap-1">
                <View class="w-full flex-row items-center rounded-(--radius-sm) bg-muted px-2 py-1.5">
                  <HeaderCell label="日期" />
                  <HeaderCell label="经验本" />
                  <HeaderCell label="纽本" />
                  <HeaderCell label="镜牢" />
                  <HeaderCell label="合计" />
                </View>
                <Each of={stats().days} key={(day) => day.date}>
                  {(day) => <DailyRow day={day()} />}
                </Each>
              </View>
            </If>
          )}
        </If>
      </View>
    </Card>
  );
}

function DailyRow(props: { day: DailyStatEntry }) {
  return (
    <View class="w-full flex-row items-center rounded-(--radius-sm) px-2 py-1.5">
      <ValueCell value={props.day.date} align="left" />
      <ValueCell value={String(props.day.exp)} />
      <ValueCell value={String(props.day.thread)} />
      <ValueCell value={String(props.day.mirror)} />
      <ValueCell value={String(props.day.total)} />
    </View>
  );
}

function HeaderCell(props: { label: string }) {
  return <Text class="min-w-0 flex-1 text-[10px] text-muted-foreground">{props.label}</Text>;
}

function ValueCell(props: { value: string; align?: "left" | "center" }) {
  return (
    <Text
      class={`min-w-0 flex-1 text-xs text-foreground ${
        props.align === "left" ? "" : "text-center"
      }`}
    >
      {props.value}
    </Text>
  );
}
