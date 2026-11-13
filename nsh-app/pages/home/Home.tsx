// The console: the backend and the week's summary, the queue of task cards, and
// the device, preview and log rails on the right.
//
// The layout is the client's three-region console: two cards across the top
// (backend link and this week's statistics), the window settings and the task
// queue under them, the run bar at the bottom, and a fixed rail for the device,
// the live preview and the log.
import { Image, Scroll, Text, View, effect, token } from "nsh";
import { Badge } from "../../components/Badge";
import { Button } from "../../components/Button";
import { Card } from "../../components/Card";
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
  cycleResolution,
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
  setTaskParam,
  startQueue,
  toggleAsyncInput,
  toggleTask,
} from "../../home";
import type { Task, TaskParam } from "../../home";
import { LogDetailsDialog, StatsDetailsDialog } from "./HomeDetails";

/** The rail's width: the device, preview and log column. */
const RAIL = "w-56";

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
              <If when={consoleFeedback()}>
                {(line) => <Text class="px-1 text-xs text-success">{line()}</Text>}
              </If>
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

/** The window and input settings the run will use. */
function WindowCard() {
  return (
    <Card class="w-full p-0">
      <View class="w-full flex-row items-center gap-2 px-4 py-3">
        <Badge label="SET" />
        <Icon name="settings2" size={14} />
        <Text class="text-xs font-medium text-foreground">窗口设置</Text>
        <View class="flex-1" />
        <WindowSetting
          label="分辨率"
          value={currentResolution()}
          testID="window-resolution"
          onPress={() => cycleResolution()}
        />
        <WindowSetting
          label="异步输入"
          value={isAsyncInput() ? "开" : "关"}
          testID="window-async-input"
          onPress={() => toggleAsyncInput()}
        />
      </View>
    </Card>
  );
}

function WindowSetting(props: {
  label: string;
  value: string;
  testID: string;
  onPress: () => void;
}) {
  return (
    <Button
      label={`${props.label} ${props.value}`}
      variant="ghost"
      size="xs"
      testID={props.testID}
      onPress={props.onPress}
    />
  );
}

/** One task card: its switch, its name, and the parameters it runs with. */
function TaskCard(props: { task: Task }) {
  const task = () => props.task;
  return (
    <Card class="w-full p-0">
      <View class="w-full flex-row items-center gap-3 px-4 py-2.5">
        <Switch
          checked={task().enabled}
          accessibilityLabel={`启用 ${task().name}`}
          onChange={() => toggleTask(task().id)}
        />
        <Icon name={task().icon} size={16} color={token("brand")} />
        <Text class="min-w-0 flex-1 text-sm text-foreground">{task().name}</Text>
        <View class="flex-row flex-wrap items-center justify-end gap-1.5">
          <Each of={task().params} key={(param) => param.id}>
            {(param) => <ParamSelect task={task().id} param={param()} />}
          </Each>
        </View>
      </View>
    </Card>
  );
}

/** One parameter: its label and its value in one compact chip. */
function ParamSelect(props: { task: Task["id"]; param: TaskParam }) {
  return (
    <View class="flex-row items-center gap-0.5 rounded-(--radius-sm) border border-input px-2 py-0.5">
      <Text class="text-xs text-muted-foreground">{props.param.label}</Text>
      <Select<string>
        accessibilityLabel={props.param.label}
        items={props.param.options}
        value={props.param.value}
        compact
        onChange={(next) => setTaskParam(props.task, props.param.id, next)}
      />
    </View>
  );
}

/** The bar under the queue: select all, clear, the completion action and start. */
function RunBar() {
  return (
    <Card class="w-full p-2">
      <View class="w-full flex-row items-center gap-2">
        <Button
          label="全选"
          icon="check"
          variant="outline"
          testID="select-all"
          onPress={() => selectAllTasks()}
        />
        <Button
          label="清空"
          icon="eraser"
          variant="outline"
          testID="clear-tasks"
          onPress={() => clearTasks()}
        />
        <View class="flex-1" />
        <Button label="什么也不干（本次）" variant="ghost" />
        <Button
          label="Link Start!"
          icon="play"
          testID="link-start"
          class="min-w-44"
          onPress={() => startQueue()}
        >
          <Text class="text-xs opacity-80">F10</Text>
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
