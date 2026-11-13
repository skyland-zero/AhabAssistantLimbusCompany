// The console: the backend and the week's summary, the queue of task cards, and
// the device, preview and log rails on the right.
//
// The layout is the client's three-region console: two cards across the top
// (backend link and this week's statistics), the window settings and the task
// queue under them, the run bar at the bottom, and a fixed rail for the device,
// the live preview and the log.
import type { JSX } from "nsh";
import { Image, Scroll, Text, View, effect, signal, token } from "nsh";
import { Badge } from "../../components/Badge";
import { Button, Pressable } from "../../components/Button";
import { Card, Rule } from "../../components/Card";
import { SettingLine, SettingList } from "../../components/Field";
import { Icon } from "../../components/Icon";
import type { IconName } from "../../components/Icon";
import { PageRoot } from "../../components/Page";
import { Select } from "../../components/Select";
import { Switch } from "../../components/Switch";
import { t } from "../../settings";
import {
  backendState,
  clearLog,
  clearTasks,
  consoleFeedback,
  consoleStats,
  consoleTasks,
  currentPreviewFrame,
  currentPreviewStatus,
  currentResolution,
  deviceList,
  deviceState,
  isAsyncInput,
  logEntries,
  logLines,
  openLogDetails,
  openStatsDetails,
  pollPreview,
  refreshDevices,
  selectAllTasks,
  selectDevice,
  setResolution,
  setTaskParam,
  startQueue,
  toggleAsyncInput,
  toggleTask,
} from "../../home";
import type { Task } from "../../home";
import { LogDetailsDialog, StatsDetailsDialog } from "./HomeDetails";

/** The rail's width: the device, preview and log column. */
const RAIL = "w-56";

/** The two window widths the task config accepts (`SetWindowsConfig`). */
const RESOLUTIONS = [
  { value: "1080", label: "1920 × 1080" },
  { value: "1440", label: "2560 × 1440" },
];

export function Home() {
  // Poll the event queue while the console is on screen: preview frames and
  // log lines arrive as sidecar events and are mirrored into signals.
  effect(() => {
    const timer = setInterval(() => pollPreview(), 200);
    return () => clearInterval(timer);
  });
  return (
    <PageRoot>
      <View class="min-h-0 flex-1 flex-row gap-2">
        <View class="min-w-0 flex-1 flex-col gap-2">
          <View class="flex-row gap-2">
            <BackendCard />
            <StatsCard />
          </View>
          <WindowCard />
          <Scroll class="min-h-0 flex-1">
            <View class="flex-col gap-2 pb-2">
              <Each of={consoleTasks()} key={(task) => task.id}>
                {(task) => <TaskCard task={task()} />}
              </Each>
            </View>
          </Scroll>
          <RunBar />
        </View>
        <View class={`${RAIL} min-h-0 flex-none flex-col gap-2`}>
          <DeviceCard />
          <PreviewCard />
          <LogCard />
        </View>
      </View>
      <StatsDetailsDialog />
      <LogDetailsDialog />
    </PageRoot>
  );
}

/** The Python backend link. */
function BackendCard() {
  return (
    <Card class="w-50 flex-none p-3">
      <View class="flex-col gap-2">
        <View class="w-full flex-row items-center justify-between gap-2">
          <View class="flex-row items-center gap-1.5">
            <Icon name="audioLines" size={14} color={token("brand")} />
            <Text class="text-xs font-medium text-foreground">Python 后端</Text>
          </View>
          <Badge label={backendLabel()} />
        </View>
        <Text class="text-xs text-muted-foreground">{backendDetail()}</Text>
      </View>
    </Card>
  );
}

function backendLabel(): string {
  const state = backendState();
  if (state === "mock") return "Mock 模式";
  if (state === "starting") return "启动中";
  if (state === "ready") return "已连接";
  return "未连接";
}

function backendDetail(): string {
  const state = backendState();
  if (state === "mock") return "未启动 Python 后端";
  if (state === "starting") return "正在启动 Python 后端…";
  if (state === "ready") return "Python 后端已就绪";
  return "Python 后端启动失败";
}

/** This week's summary: how many runs finished, and the last mirror run. */
function StatsCard() {
  const stats = () => consoleStats();
  return (
    <Card class="min-w-0 flex-1 p-3">
      <View class="flex-col gap-2">
        <View class="w-full flex-row items-center justify-between gap-2">
          <View class="flex-row items-center gap-1.5">
            <Icon name="target" size={14} color={token("brand")} />
            <Text class="text-xs font-medium text-foreground">周期统计（今日/本周）</Text>
          </View>
          <Button
            label="明细"
            variant="ghost"
            size="xs"
            testID="stats-daily-details"
            onPress={() => openStatsDetails("daily")}
          />
        </View>
        <View class="w-full flex-row gap-4">
          <StatColumn
            label="经验本"
            value={`${stats().today.exp} / ${stats().week.exp}`}
          />
          <StatColumn
            label="纽本"
            value={`${stats().today.thread} / ${stats().week.thread}`}
          />
          <StatColumn
            label="镜牢（累计）"
            value={`${stats().today.mirror} / ${stats().week.mirror}`}
          />
        </View>
        <View class="w-full flex-row items-center justify-between gap-2 border-t border-border pt-2">
          <View class="flex-row items-center gap-1.5">
            <Icon name="target" size={14} color={token("brand")} />
            <Text class="text-xs font-medium text-foreground">上次镜牢</Text>
          </View>
          <Button
            label="明细"
            variant="ghost"
            size="xs"
            testID="stats-last-mirror-details"
            onPress={() => openStatsDetails("mirror")}
          />
        </View>
        <Text class="text-center text-xs text-muted-foreground">
          {stats().lastMirror ?? "暂无完成记录"}
        </Text>
      </View>
    </Card>
  );
}

function StatColumn(props: { label: string; value: string }) {
  return (
    <View class="flex-col gap-0.5">
      <Text class="text-xs text-muted-foreground">{props.label}</Text>
      <Text class="text-xs text-foreground">{props.value}</Text>
    </View>
  );
}

/** The window and input settings the run will use. The card's row states them
 *  and its expansion holds their controls: a press on the row opens and closes
 *  it, as every card of the queue below does. */
function WindowCard() {
  const open = signal(false);
  return (
    <Card class="w-full p-0">
      <Pressable
        accessibilityLabel={`窗口设置（${open() ? "已展开" : "已收起"}）`}
        testID="window-toggle"
        class="w-full flex-row items-center gap-2 rounded-(--radius-lg) px-4 py-3 hover:bg-accent active:bg-secondary"
        onPress={() => open.set(!open())}
      >
        <Badge label="SET" />
        <Icon name="settings2" size={14} />
        <Text class="text-xs font-medium text-foreground">窗口设置</Text>
        <View class="flex-1" />
        <Text class="text-xs text-muted-foreground">{`分辨率 ${currentResolution()}`}</Text>
        <Text class="text-xs text-muted-foreground">
          {`异步输入 ${isAsyncInput() ? "开" : "关"}`}
        </Text>
        <Chevron open={open()} />
      </Pressable>
      <Expansion open={open()}>
        <SettingList>
          <SettingLine label="窗口分辨率">
            <Select<string>
              accessibilityLabel="窗口分辨率"
              items={RESOLUTIONS}
              value={currentResolution() === "1440P" ? "1440" : "1080"}
              onChange={(next) => setResolution(next === "1440" ? 1440 : 1080)}
              class="w-36"
            />
          </SettingLine>
          <SettingLine label="异步 PostMessage 输入">
            <Switch
              checked={isAsyncInput()}
              accessibilityLabel="异步 PostMessage 输入"
              onChange={() => toggleAsyncInput()}
            />
          </SettingLine>
        </SettingList>
      </Expansion>
    </Card>
  );
}

/** The chevron of a card's row: it points down at what the press opens, and
 *  up once it is open. */
function Chevron(props: { open: boolean }) {
  return <Icon name="chevronDown" size={14} rotate={props.open ? 180 : 0} />;
}

/** A card's expansion: a divider and the row's settings, under the row. */
function Expansion(props: { open: boolean; children?: JSX.Element }) {
  return (
    <If when={props.open}>
      <View class="w-full flex-col">
        <Rule />
        <View class="w-full px-4 py-3">{props.children}</View>
      </View>
    </If>
  );
}

/** One task card: its switch, its name, and the parameters it runs with. The
 *  row states the parameters and a press on it opens their controls. */
function TaskCard(props: { task: Task }) {
  const task = () => props.task;
  const open = signal(false);
  return (
    <Card class="w-full p-0">
      <View class="w-full flex-row items-center gap-3 px-4 py-2.5">
        <Switch
          checked={task().enabled}
          accessibilityLabel={`启用 ${task().name}`}
          onChange={() => toggleTask(task().id)}
        />
        <Pressable
          accessibilityLabel={`${task().name}设置（${open() ? "已展开" : "已收起"}）`}
          testID={`task-toggle-${task().id}`}
          class="min-w-0 flex-1 flex-row items-center gap-3 rounded-(--radius) ps-1 py-0.5 hover:bg-accent active:bg-secondary"
          onPress={() => open.set(!open())}
        >
          <Icon name={task().icon} size={16} color={token("brand")} />
          <Text class="min-w-0 text-sm text-foreground">{task().name}</Text>
          <View class="flex-1" />
          <View class="flex-row items-center gap-2">
            <If when={!open()}>
              <Text class="text-xs text-muted-foreground">{taskSummary(task())}</Text>
            </If>
            <Chevron open={open()} />
          </View>
        </Pressable>
      </View>
      <Expansion open={open()}>
        <SettingList>
          <Each of={task().params} key={(param) => param.id}>
            {(param) => (
              <SettingLine label={param().label}>
                <Select<string>
                  accessibilityLabel={param().label}
                  items={param().options}
                  value={param().value}
                  onChange={(next) => setTaskParam(task().id, param().id, next)}
                  class="w-36"
                />
              </SettingLine>
            )}
          </Each>
        </SettingList>
      </Expansion>
    </Card>
  );
}

/** A task's parameters as one line: `经验本 ×1 · 纽本 ×3 · 连战 关`. */
function taskSummary(task: Task): string {
  return task.params
    .map((param) => {
      const label = param.options.find((option) => option.value === param.value)?.label;
      return `${param.label} ${label ?? param.value}`;
    })
    .join(" · ");
}

/** The bar under the queue: select all, clear, the completion action and start. */
function RunBar() {
  return (
    <Card class="w-full p-2.5">
      <View class="w-full flex-row items-center gap-2">
        <Button
          label="全选"
          icon="check"
          variant="outline"
          size="lg"
          testID="select-all"
          onPress={() => selectAllTasks()}
        />
        <Button
          label="清空"
          icon="eraser"
          variant="outline"
          size="lg"
          testID="clear-tasks"
          onPress={() => clearTasks()}
        />
        <View class="flex-1" />
        <Button label="什么也不干（本次）" variant="ghost" size="lg" />
        <Button
          label="Link Start!"
          icon="play"
          size="lg"
          testID="link-start"
          onPress={() => startQueue()}
        >
          <Text class="rounded-(--radius-sm) bg-scrim/40 px-1.5 py-0.5 text-xs opacity-80">F10</Text>
        </Button>
      </View>
    </Card>
  );
}

/** The device the console drives. */
function DeviceCard() {
  const options = () => [
    { value: "", label: "选择游戏窗口 / 模拟器" },
    ...deviceList().map((entry) => ({ value: entry.id, label: entry.name })),
  ];
  return (
    // The connected device is the console's current object, so its card takes
    // the skin's selected treatment.
    <Card class="w-full p-3" selected={deviceState() === "connected"}>
      <View class="flex-col gap-2">
        <View class="w-full flex-row items-center justify-between gap-2">
          <View class="flex-row items-center gap-1.5">
            <Icon name="monitor" size={14} />
            <Text class="text-xs font-medium text-foreground">设备连接</Text>
          </View>
          <Badge label={deviceLabel()} tone={deviceState() === "connected" ? "success" : "neutral"} />
        </View>
        <View class="w-full flex-row items-center gap-1">
          <View class="min-w-0 flex-1">
            <Select<string>
              accessibilityLabel="选择设备"
              items={options()}
              value=""
              onChange={(next) => selectDevice(next)}
            />
          </View>
          <Button label="刷新" icon="refresh" variant="ghost" size="xs" onPress={() => refreshDevices()} />
        </View>
      </View>
    </Card>
  );
}

function deviceLabel(): string {
  const state = deviceState();
  if (state === "connected") return "已连接";
  if (state === "connecting") return "连接中";
  return "未连接";
}

/** The preview status chip in the current language. */
function previewLabel(status: string): string {
  if (status === "running") return t("homeRunning");
  if (status === "starting") return t("previewStarting");
  if (status === "error") return t("previewError");
  return t("previewStopped");
}

/** The live preview: frames arrive from the sidecar once a device is connected. */
function PreviewCard() {
  const frame = () => currentPreviewFrame();
  return (
    <Card class={`${RAIL} w-full p-3`}>
      <View class="flex-col gap-2">
        <View class="w-full flex-row items-center justify-between gap-2">
          <View class="flex-row items-center gap-1.5">
            <Icon name="monitorPlay" size={14} />
            <Text class="text-xs font-medium text-foreground">实时画面</Text>
          </View>
          <Badge label={previewLabel(currentPreviewStatus())} />
        </View>
        <View class="h-40 w-full items-center justify-center overflow-hidden rounded-(--radius) bg-muted">
          <If
            when={frame()}
            else={
              <View class="items-center gap-2">
                <Icon name="monitorPlay" size={28} color={token("muted-foreground")} />
                <Text class="text-xs text-muted-foreground">连接设备后显示实时画面</Text>
              </View>
            }
          >
            {(src) => (
              <Image
                src={src()}
                style={{ width: 200, height: 160 }}
                fadeDuration={0}
              />
            )}
          </If>
        </View>
      </View>
    </Card>
  );
}

/** The run log. */
function LogCard() {
  return (
    <Card class="min-h-0 w-full flex-1 p-3">
      <View class="h-full flex-col gap-2">
        <View class="w-full flex-row items-center justify-between gap-2">
          <View class="flex-row items-center gap-1.5">
            <Icon name="terminal" size={14} />
            <Text class="text-xs font-medium text-foreground">运行日志</Text>
            <Badge label={String(logLines())} />
          </View>
          <View class="flex-row items-center gap-1">
            <Button
              label="明细"
              variant="ghost"
              size="xs"
              testID="log-details"
              onPress={() => openLogDetails()}
            />
            <Button
              label="清空"
              icon="eraser"
              variant="ghost"
              size="xs"
              testID="clear-log"
              onPress={() => clearLog()}
            />
          </View>
        </View>
        {/* The last action's line lives with the logs: the queue is a work
            list, not a message board. */}
        <If when={consoleFeedback()}>
          {(line) => <Text class="text-xs text-muted-foreground">{line()}</Text>}
        </If>
        <Scroll class="min-h-0 flex-1">
          <View class="flex-col gap-1">
            <If when={logLines() === 0}>
              <Text class="text-xs text-muted-foreground">暂无日志</Text>
            </If>
            <Each of={logEntries()} key={(entry) => `${entry.ts}-${entry.message}`}>
              {(entry) => (
                <View class="w-full flex-row items-start gap-1">
                  <Badge label={entry().level} />
                  <Text class="min-w-0 flex-1 text-xs text-muted-foreground">
                    {entry().message}
                  </Text>
                </View>
              )}
            </Each>
          </View>
        </Scroll>
      </View>
    </Card>
  );
}

/** Kept next to the page so a reader can see which glyphs the console names. */
export type HomeIcon = Extract<IconName, "audioLines" | "target" | "terminal" | "monitorPlay">;
