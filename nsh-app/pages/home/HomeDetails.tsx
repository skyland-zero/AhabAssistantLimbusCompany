// The console's detail dialogs: daily statistics, mirror history and the full
// run log. They read the same Rust-backed signals as the cards.
import { Scroll, Text, View } from "nsh";
import { Dialog } from "nsh/base";
import { Badge } from "../../components/Badge";
import { Button } from "../../components/Button";
import {
  closeLogDetails,
  closeStatsDetails,
  currentDailyStats,
  currentStatsDetails,
  currentStatsDetailsError,
  isLogDetailsOpen,
  logEntries,
  mirrorHistory,
} from "../../home";
import type { DailyStatEntry, MirrorHistoryEntry } from "../../home";

const DIALOG_CLASS =
  "m-auto h-130 max-h-full w-160 max-w-full rounded-(--radius-lg) border border-border bg-popover p-0 shadow-lg";

/** The two statistics dialogs share a shell and differ in their body. */
export function StatsDetailsDialog() {
  const mode = () => currentStatsDetails();
  return (
    <Dialog
      open={mode() !== null}
      onOpenChange={({ open }) => {
        if (!open) closeStatsDetails();
      }}
      closeOnEscape
      closeOnBackdropPress
      class={DIALOG_CLASS}
      backdropClass="bg-scrim"
      accessibilityLabel={mode() === "mirror" ? "镜牢完成记录" : "每日刷本明细"}
    >
      <View class="h-full min-h-0 flex-col">
        <View class="w-full flex-none flex-row items-center justify-between gap-3 border-b border-border px-5 py-3">
          <Text class="text-base font-semibold text-popover-foreground">
            {mode() === "mirror" ? "镜牢完成记录" : "每日刷本明细"}
          </Text>
          <Button
            label="关闭"
            variant="ghost"
            size="xs"
            testID="stats-details-close"
            onPress={() => closeStatsDetails()}
          />
        </View>
        <Scroll class="min-h-0 flex-1">
          <View class="w-full flex-col gap-3 p-5">
            <If when={mode() === "daily"}>
              <DailyStatsBody />
            </If>
            <If when={mode() === "mirror"}>
              <MirrorHistoryBody />
            </If>
          </View>
        </Scroll>
      </View>
    </Dialog>
  );
}

function DailyStatsBody() {
  const data = () => currentDailyStats();
  const error = () => currentStatsDetailsError();
  return (
    <If
      when={data()}
      else={
        <View class="items-center justify-center gap-2 py-10">
          <Text class="text-xs text-muted-foreground">
            {error() ?? "正在加载每日统计…"}
          </Text>
        </View>
      }
    >
      {(stats) => (
        <View class="w-full flex-col gap-3">
          <Text class="text-xs text-muted-foreground">
            {stats().dateFrom && stats().dateTo
              ? `${stats().dateFrom} – ${stats().dateTo}`
              : "每日完成"}
          </Text>
          <View class="w-full flex-row rounded-(--radius-sm) bg-muted px-2 py-1.5">
            <HeaderCell label="日期" />
            <HeaderCell label="经验本" />
            <HeaderCell label="纽本" />
            <HeaderCell label="镜牢" />
            <HeaderCell label="合计" />
          </View>
          <If
            when={stats().days.length > 0}
            else={
              <Text class="py-8 text-center text-xs text-muted-foreground">暂无每日数据</Text>
            }
          >
            <View class="w-full flex-col gap-1">
              <Each of={stats().days} key={(day) => day.date}>
                {(day) => <DailyStatsRow day={day()} />}
              </Each>
            </View>
          </If>
        </View>
      )}
    </If>
  );
}

function DailyStatsRow(props: { day: DailyStatEntry }) {
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
  return (
    <Text class="min-w-0 flex-1 text-[10px] text-muted-foreground">{props.label}</Text>
  );
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

function MirrorHistoryBody() {
  const history = () => mirrorHistory();
  return (
    <If
      when={history().length > 0}
      else={
        <Text class="py-8 text-center text-xs text-muted-foreground">暂无镜牢完成记录</Text>
      }
    >
      <View class="w-full flex-col gap-2">
        <Each of={history()} key={(entry) => entry.completedAt + entry.totalSeconds}>
          {(entry) => <MirrorHistoryRow entry={entry()} />}
        </Each>
      </View>
    </If>
  );
}

function MirrorHistoryRow(props: { entry: MirrorHistoryEntry }) {
  const entry = () => props.entry;
  const failed = () => entry().failed === true;
  return (
    <View class="w-full flex-col gap-1 rounded-(--radius) border border-border p-3">
      <View class="w-full flex-row items-center justify-between gap-2">
        <Text class="min-w-0 flex-1 text-xs text-foreground">
          {entry().completedAt.replace("T", " ")}
        </Text>
        <Badge label={entry().hardMode ? "困难" : "普通"} tone={entry().hardMode ? "warning" : "neutral"} />
        <Badge label={failed() ? "未完成" : "已完成"} tone={failed() ? "danger" : "success"} />
        <Text class="font-mono text-xs text-brand">{durationText(entry().totalSeconds)}</Text>
      </View>
      <View class="w-full flex-row flex-wrap items-center gap-2 text-[10px] text-muted-foreground">
        <Text class="text-[10px] text-foreground">
          {entry().team?.name ?? "未知队伍"}
          {entry().team?.number ? ` #${entry().team?.number}` : ""}
        </Text>
        <If when={entry().failureReason}>
          {(reason) => <Text class="text-[10px] text-danger">原因：{reason()}</Text>}
        </If>
      </View>
    </View>
  );
}

/** The full log dialog: every buffered line, with level and time. */
export function LogDetailsDialog() {
  return (
    <Dialog
      open={isLogDetailsOpen()}
      onOpenChange={({ open }) => {
        if (!open) closeLogDetails();
      }}
      closeOnEscape
      closeOnBackdropPress
      class={DIALOG_CLASS}
      backdropClass="bg-scrim"
      accessibilityLabel="运行日志明细"
    >
      <View class="h-full min-h-0 flex-col">
        <View class="w-full flex-none flex-row items-center justify-between gap-3 border-b border-border px-5 py-3">
          <View class="flex-row items-center gap-2">
            <Text class="text-base font-semibold text-popover-foreground">运行日志明细</Text>
            <Badge label={String(logEntries().length)} />
          </View>
          <Button
            label="关闭"
            variant="ghost"
            size="xs"
            testID="log-details-close"
            onPress={() => closeLogDetails()}
          />
        </View>
        <Scroll class="min-h-0 flex-1">
          <View class="w-full flex-col gap-1 p-4">
            <If
              when={logEntries().length > 0}
              else={
                <Text class="py-8 text-center text-xs text-muted-foreground">暂无日志</Text>
              }
            >
              <Each of={logEntries()} key={(entry) => `${entry.ts}-${entry.message}`}>
                {(entry) => (
                  <View class="w-full flex-row items-start gap-2 py-0.5 font-mono">
                    <Text class="w-16 flex-none text-[10px] text-muted-foreground">
                      {logTime(entry().ts)}
                    </Text>
                    <Badge label={entry().level} />
                    <Text class="min-w-0 flex-1 text-xs text-foreground">
                      {entry().message}
                    </Text>
                  </View>
                )}
              </Each>
            </If>
          </View>
        </Scroll>
      </View>
    </Dialog>
  );
}

/** `m:ss.cc`, matching the team statistics formatter. */
function durationText(seconds: number): string {
  if (seconds <= 0) return "暂无";
  const centiseconds = Math.round(seconds * 100);
  const minutes = Math.floor(centiseconds / 6000);
  const rest = centiseconds % 6000;
  const whole = Math.floor(rest / 100);
  const fraction = rest % 100;
  return `${minutes}:${String(whole).padStart(2, "0")}.${String(fraction).padStart(2, "0")}`;
}

/** `HH:mm:ss`, or `--:--:--` for a line without a timestamp. */
function logTime(ts: number): string {
  if (!Number.isFinite(ts) || ts <= 0) return "--:--:--";
  const date = new Date(ts);
  return date.toLocaleTimeString([], { hour12: false });
}
