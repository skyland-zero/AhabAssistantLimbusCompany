// The console: the backend and the week's summary, the queue of task cards, and
// the device, preview and log rails on the right.
//
// The layout is the client's three-region console: a two-row status strip
// (this week's counts, then the backend link and the last mirror run), the
// window settings and the task queue under it, the run bar at the bottom, and a
// fixed rail for the device, the live preview and the log.
import type { JSX, ViewRef } from "nsh";
import { Image, Scroll, Text, View, effect, reduceMotion, signal, token } from "nsh";
import { Collapsible } from "nsh/base";
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
  afterCompletionLabel,
  afterCompletionMode,
  afterCompletionSettings,
  backendState,
  clearLog,
  clearTasks,
  consoleFeedback,
  consoleStats,
  consoleTasks,
  currentPreviewFrame,
  currentPreviewStatus,
  currentWindow,
  deviceList,
  deviceState,
  logEntries,
  logLines,
  mirrorTimeLabel,
  openAfterCompletion,
  openLogDetails,
  pollPreview,
  refreshDevices,
  selectAllTasks,
  selectDevice,
  setWindowSetting,
  setTaskParam,
  startQueue,
  toggleTask,
} from "../../home";
import type { Task, TaskParam } from "../../home";
import { navigate } from "../../store";
import { AfterCompletionDialog } from "./HomeCompletion";
import { LogDetailsDialog } from "./HomeDetails";

/** The rail's width: the device, preview and log column. */
const RAIL = "w-56";

/** The window sizes the task config accepts (`SetWindowsConfig.set_win_size`). */
const WINDOW_SIZES = [
  { value: "720", label: "1280 × 720" },
  { value: "1080", label: "1920 × 1080" },
  { value: "1440", label: "2560 × 1440" },
];

/** Where the run puts the game window before it starts. */
const WINDOW_POSITIONS = [
  { value: "center", label: "屏幕居中" },
  { value: "left_top", label: "靠左对齐" },
  { value: "right_top", label: "靠右对齐" },
  { value: "free", label: "保持原位" },
];

/** How often the run takes a screenshot, in the old client's four steps. */
const SCREENSHOT_INTERVALS = [
  { value: "0.1", label: "0.1s 极速" },
  { value: "0.2", label: "0.2s" },
  { value: "0.5", label: "0.5s 默认" },
  { value: "1.0", label: "1.0s 较慢" },
];

/** How long the run waits between mouse actions. */
const MOUSE_INTERVALS = [
  { value: "0.1", label: "0.1s" },
  { value: "0.3", label: "0.3s 默认" },
  { value: "0.5", label: "0.5s" },
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
          <StatusStrip />
          <Scroll class="min-h-0 flex-1">
            <View class="flex-col gap-2 pb-2">
              <WindowCard />
              <Each of={consoleTasks()} key={(task) => task.id}>
                {(task, index) => <TaskCard task={task()} index={index() + 1} />}
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
      <LogDetailsDialog />
      <AfterCompletionDialog />
    </PageRoot>
  );
}

/** The console's status strip: the week's counts, then the backend link and
 *  the last mirror run. One dense card instead of two tall ones, so the queue
 *  keeps the height the two cards used to take. */
function StatusStrip() {
  const stats = () => consoleStats();
  return (
    <Card class="w-full p-2.5">
      <View class="w-full flex-col gap-1">
        <View class="w-full flex-row items-center gap-2">
          <Icon name="target" size={14} color={token("brand")} />
          <Text class="text-xs font-medium text-foreground">周期统计（今日/本周）</Text>
          <View class="flex-1" />
          <StatChip label="经验本" value={`${stats().today.exp} / ${stats().week.exp}`} />
          <StatChip label="纽本" value={`${stats().today.thread} / ${stats().week.thread}`} />
          <StatChip label="镜牢（累计）" value={`${stats().today.mirror} / ${stats().week.mirror}`} />
          <Button
            label="明细"
            variant="ghost"
            size="xs"
            testID="stats-daily-details"
            onPress={() => navigate("stats")}
          />
        </View>
        <View class="w-full flex-row items-center gap-2">
          <Icon name="audioLines" size={14} color={token("brand")} />
          <Text class="text-xs font-medium text-foreground">Python 后端</Text>
          <Badge label={backendLabel()} />
          <Text class="min-w-0 flex-1 truncate text-xs text-muted-foreground">
            {backendDetail()}
          </Text>
          <Icon name="target" size={14} color={token("brand")} />
          <Text class="text-xs font-medium text-foreground">上次镜牢</Text>
          <If
            when={stats().lastMirror}
            else={<Text class="text-xs text-muted-foreground">暂无完成记录</Text>}
          >
            {(mirror) => (
              <Text class="max-w-40 min-w-0 truncate text-xs text-muted-foreground">
                {mirrorTimeLabel(mirror())}
              </Text>
            )}
          </If>
          <Button
            label="明细"
            variant="ghost"
            size="xs"
            testID="stats-last-mirror-details"
            onPress={() => navigate("stats")}
          />
        </View>
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

/** One counter of the strip: its name and the today/week pair. */
function StatChip(props: { label: string; value: string }) {
  return (
    <View class="flex-none flex-row items-center gap-1">
      <Text class="text-xs text-muted-foreground">{props.label}</Text>
      <Text class="text-xs text-foreground">{props.value}</Text>
    </View>
  );
}

/** The window and input settings the run will use. The queue's first row: a
 *  press on it opens its settings, inside the scroll the queue lives in. */
function WindowCard() {
  const open = signal(false);
  const window = () => currentWindow();
  return (
    <Card class="w-full p-0" arrive={0}>
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
        <Text class="text-xs text-muted-foreground">{`分辨率 ${window().resolution}`}</Text>
        <Text class="text-xs text-muted-foreground">
          {`异步输入 ${window().asyncInput ? "开" : "关"}`}
        </Text>
        <Chevron open={open()} />
      </Pressable>
      <Expansion open={open()}>
        <SettingList>
          <SettingLine label="窗口分辨率">
            <Select<string>
              accessibilityLabel="窗口分辨率"
              items={WINDOW_SIZES}
              value={window().size}
              onChange={(next) => setWindowSetting("size", next)}
              class="w-36"
            />
          </SettingLine>
          <SettingLine label="窗口位置">
            <Select<string>
              accessibilityLabel="窗口位置"
              items={WINDOW_POSITIONS}
              value={window().position}
              onChange={(next) => setWindowSetting("position", next)}
              class="w-36"
            />
          </SettingLine>
          <SettingLine label="减少误触">
            <Switch
              checked={window().reduceMiscontact}
              accessibilityLabel="减少误触"
              onChange={() => setWindowSetting("reduceMiscontact", !window().reduceMiscontact)}
            />
          </SettingLine>
          <SettingLine label="截图间隔">
            <Select<string>
              accessibilityLabel="截图间隔"
              items={SCREENSHOT_INTERVALS}
              value={window().screenshotInterval}
              onChange={(next) => setWindowSetting("screenshotInterval", next)}
              class="w-36"
            />
          </SettingLine>
          <SettingLine label="鼠标操作间隔">
            <Select<string>
              accessibilityLabel="鼠标操作间隔"
              items={MOUSE_INTERVALS}
              value={window().mouseInterval}
              onChange={(next) => setWindowSetting("mouseInterval", next)}
              class="w-36"
            />
          </SettingLine>
          <SettingLine label="异步 PostMessage 输入">
            <Switch
              checked={window().asyncInput}
              accessibilityLabel="异步 PostMessage 输入"
              onChange={() => setWindowSetting("asyncInput", !window().asyncInput)}
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

/** A card's expansion: a divider and the row's settings, under the row. It
 *  opens and closes with a native spring of its own height, so the cards under it
 *  ride along instead of jumping (reduced motion: both instant). */
function Expansion(props: {
  open: boolean;
  /** The body, for a card that brings it into view as it opens. */
  bodyRef?: (ref: ViewRef) => void;
  children?: JSX.Element;
}) {
  return (
    <Collapsible
      open={props.open}
      reveal
      content={
        <View class="w-full flex-col" ref={props.bodyRef}>
          <Rule />
          <View class="w-full flex-col gap-3 px-4 py-3">{props.children}</View>
        </View>
      }
    />
  );
}

/** One task card: its switch, its name, and the parameters it runs with. The
 *  row states the parameters and a press on it opens their controls; a card
 *  without settings (Ahab resonance) is only its switch and name. */
function TaskCard(props: { task: Task; index: number }) {
  const task = () => props.task;
  const open = signal(false);
  const hasSettings = () => task().params.length > 0;
  // The queue scrolls: a card near its end opens below the fold, so the press
  // asks the scroll view for the settings, the shortest way and animated.
  let body: ViewRef | undefined;
  const toggleOpen = () => {
    const next = !open();
    open.set(next);
    if (next) body?.reveal({ vertical: "nearest", animated: !reduceMotion() });
  };
  return (
    <Card class="w-full p-0" arrive={props.index}>
      <View class="w-full flex-row items-center gap-3 px-4 py-2.5">
        <Switch
          checked={task().enabled}
          accessibilityLabel={`启用 ${task().name}`}
          onChange={() => toggleTask(task().id)}
        />
        <If when={hasSettings()}>
          <Pressable
            accessibilityLabel={`${task().name}设置（${open() ? "已展开" : "已收起"}）`}
            testID={`task-toggle-${task().id}`}
            class="min-w-0 flex-1 flex-row items-center gap-3 rounded-(--radius) ps-1 py-0.5 hover:bg-accent active:bg-secondary"
            onPress={toggleOpen}
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
        </If>
        <If when={!hasSettings()}>
          <View class="min-w-0 flex-1 flex-row items-center gap-3">
            <Icon name={task().icon} size={16} color={token("brand")} />
            <Text class="min-w-0 truncate text-sm text-foreground">{task().name}</Text>
          </View>
        </If>
      </View>
      <If when={hasSettings()}>
        <Expansion open={open()} bodyRef={(ref) => (body = ref)}>
          <Each of={taskGroups(task())} key={(group) => group.title}>
            {(group) => (
              <View class="w-full flex-col gap-2">
                <If when={group().title}>
                  {(title) => (
                    <Text class="text-xs font-medium text-muted-foreground">{title()}</Text>
                  )}
                </If>
                <SettingList>
                  <Each of={group().params} key={(param) => param.id}>
                    {(param) => <ParamRow task={task().id} param={param()} />}
                  </Each>
                </SettingList>
              </View>
            )}
          </Each>
        </Expansion>
      </If>
    </Card>
  );
}

/** One setting row: a select for a choice, a switch for an on/off setting. */
function ParamRow(props: { task: Task["id"]; param: TaskParam }) {
  const param = () => props.param;
  // A card can offer the saved teams as its choices; the list loads with them.
  const items = () => {
    const options = param().options;
    return typeof options === "function" ? options() : (options ?? []);
  };
  return (
    <SettingLine label={param().label}>
      <If when={param().kind === "switch"}>
        <Switch
          checked={param().value === "on"}
          accessibilityLabel={param().label}
          onChange={() =>
            setTaskParam(props.task, param().id, param().value === "on" ? "off" : "on")
          }
        />
      </If>
      <If when={param().kind === "select"}>
        <Select<string>
          accessibilityLabel={param().label}
          items={items()}
          value={param().value}
          onChange={(next) => setTaskParam(props.task, param().id, next)}
          class="w-36"
        />
      </If>
    </SettingLine>
  );
}

/** A card's settings in the sections its parameters name, the way the client's
 *  options panel had 常规 and 高级. */
function taskGroups(task: Task): { title: string; params: TaskParam[] }[] {
  const groups: { title: string; params: TaskParam[] }[] = [];
  for (const param of task.params) {
    const title = param.group ?? "";
    const group = groups.find((entry) => entry.title === title);
    if (group) group.params.push(param);
    else groups.push({ title, params: [param] });
  }
  return groups;
}

/** A task's parameters as one line: `经验本 ×1 · 纽本 ×3 · 连战 关`. Only the
 *  ones a card marks as its summary, so a card with a dozen switches keeps a
 *  readable row. */
function taskSummary(task: Task): string {
  return task.params
    .filter((param) => param.summary)
    .map((param) => {
      if (param.kind === "switch") {
        return `${param.label} ${param.value === "on" ? "开" : "关"}`;
      }
      const options = typeof param.options === "function" ? param.options() : (param.options ?? []);
      const label = options.find((option) => option.value === param.value)?.label;
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
        <Button
          label={afterCompletionLabel(afterCompletionSettings())}
          variant="ghost"
          size="lg"
          testID="after-completion"
          onPress={() => openAfterCompletion()}
        >
          <Text class="rounded-(--radius-sm) bg-scrim/40 px-1.5 py-0.5 text-xs opacity-80">
            {afterCompletionMode(afterCompletionSettings())}
          </Text>
        </Button>
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
