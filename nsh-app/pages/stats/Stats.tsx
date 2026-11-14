// The statistics page: the console's mirror runs, by time.
//
// The sidecar keeps the last 30 records (phase seconds, team, difficulty,
// route) and persists them, so this page is a view over `home.ts`'s payload:
// a summary strip, the recent runs as stacked bars, the phase split, the teams
// and the daily counts.
import { Scroll, Text, View, effect, signal, token } from "nsh";
import { Bar, Grid, Plot, PlotAxis } from "nsh/base/plot";
import { Badge } from "../../components/Badge";
import { Button } from "../../components/Button";
import { Card, CardBody, Rule } from "../../components/Card";
import { EmptyState } from "../../components/EmptyState";
import { Figure } from "../../components/Figure";
import { Icon } from "../../components/Icon";
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

/** The overview card: the same data card the console's top card is — a title
 *  line, four figures, and the derived readings as one footer line. */
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
  const share = () =>
    props.count > 0 ? `${Math.round((props.hard / props.count) * 100)}%` : undefined;
  return (
    <Card class="w-full px-4 py-2.5" arrive={0}>
      <View class="w-full flex-row items-center gap-1.5">
        <Icon name="chart" size={14} color={token("brand")} />
        <Text class="flex-none text-xs font-semibold text-foreground">总览</Text>
        <Text class="flex-none text-[10px] text-muted-foreground">{`最近 ${props.count} 场`}</Text>
      </View>
      <View class="mt-1.5 w-full flex-row gap-3">
        <Figure label="场次" value={String(props.count)} />
        <Figure label="完成" value={String(props.completed)} unit={`/ ${props.count}`} />
        <Figure label="困难" note={share()} value={String(props.hard)} />
        <Figure label="平均用时" value={clockText(props.averageSeconds)} />
      </View>
      <Rule class="my-2" />
      <View class="w-full flex-row flex-wrap items-center gap-x-3 gap-y-1">
        <Text class="text-[10px] text-muted-foreground">
          {`完成总用时 ${spanText(props.totalSeconds)}`}
        </Text>
        <Text class="text-[10px] text-muted-foreground">{`最快 ${clockText(props.fastestSeconds)}`}</Text>
        <Text class="text-[10px] text-muted-foreground">{`最慢 ${clockText(props.slowestSeconds)}`}</Text>
        <If when={props.failed > 0}>
          <Text class="text-[10px] text-danger">{`未完成 ${props.failed}`}</Text>
        </If>
      </View>
    </Card>
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
    <Card
      title="最近镜牢用时"
      note="按环节"
      class="w-full"
      arrive={1}
      actions={
        <View class="flex-row flex-wrap items-center gap-2">
          <Each of={PHASE_GROUPS}>{(phase) => <LegendItem phase={phase()} />}</Each>
        </View>
      }
    >
      <CardBody class="gap-2">
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
      </CardBody>
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
    <Card title="环节用时" class="min-w-0 flex-1" arrive={2}>
      <CardBody class="gap-2">
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
      </CardBody>
    </Card>
  );
}

/** One row per team: runs, average and its own phase split. */
function TeamCard(props: { runs: MirrorHistoryEntry[] }) {
  return (
    <Card title="队伍" class="min-w-0 flex-1" arrive={3}>
      <CardBody class="gap-2">
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
      </CardBody>
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
    <Card title="最近记录" note={`${props.runs.length} 场`} class="w-full" arrive={4}>
      <CardBody class="gap-0.5">
        <Each of={props.runs}>
          {(run) => (
            <View class="w-full flex-row flex-wrap items-center gap-2 rounded-(--radius-sm) px-2 py-1.5 hover:bg-accent">
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
      </CardBody>
    </Card>
  );
}

/** The daily counts the console's summary no longer opens a dialog for. */
function DailyCard() {
  const data = () => currentDailyStats();
  const error = () => currentStatsDetailsError();
  const span = () => {
    const stats = data();
    if (!stats) return undefined;
    return stats.dateFrom && stats.dateTo ? `${stats.dateFrom} – ${stats.dateTo}` : "每日完成";
  };
  return (
    <Card title="每日次数" note={span()} class="w-full" arrive={5}>
      <CardBody class="gap-2">
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
      </CardBody>
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
