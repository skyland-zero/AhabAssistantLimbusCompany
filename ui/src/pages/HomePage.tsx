import {
  CalendarCheck,
  CircleAlert,
  Compass,
  Gift,
  MonitorPlay,
  Radio,
  ScrollText,
  Sliders,
  Trash2,
  Zap,
} from "lucide-react";
import { useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { toast } from "sonner";
import { ConnectionPanel } from "@/components/connection/ConnectionPanel";
import { AfterCompletionModal } from "@/components/tasks/AfterCompletionModal";
import { BuyEnkephalinOptions } from "@/components/tasks/BuyEnkephalinOptions";
import { DailyTaskOptions } from "@/components/tasks/DailyTaskOptions";
import { ExecutionToolbar } from "@/components/tasks/ExecutionToolbar";
import { FixedTaskCard, type PreviewTag } from "@/components/tasks/FixedTaskCard";
import { GetRewardOptions } from "@/components/tasks/GetRewardOptions";
import { MirrorOptions } from "@/components/tasks/MirrorOptions";
import { SetWindowsOptions } from "@/components/tasks/SetWindowsOptions";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { ScrollArea } from "@/components/ui/scroll-area";
import { isTauri } from "@/lib/env";
import { cn } from "@/lib/utils";
import { getIpc } from "@/services/ipc/client";
import type {
  AfterCompletionConfig,
  ExecutionState,
  ExecutionStatusPayload,
  FixedTaskId,
  LogEntryPayload,
  MirrorProgressPayload,
  TasksConfig,
  TeamSummary,
} from "@/services/ipc/types";
import { useAppStore } from "@/stores/appStore";

export function HomePage() {
  const { t } = useTranslation();
  const rightPanelWidth = useAppStore((s) => s.rightPanelWidth);
  const rightPanelCollapsed = useAppStore((s) => s.rightPanelCollapsed);
  const setRightPanelWidth = useAppStore((s) => s.setRightPanelWidth);
  const setRightPanelCollapsed = useAppStore((s) => s.setRightPanelCollapsed);

  /* 任务完整配置与队伍数据 */
  const [tasksConfig, setTasksConfig] = useState<TasksConfig | null>(null);
  const [teams, setTeams] = useState<TeamSummary[]>([]);

  /* 执行状态与进度 */
  const [executionState, setExecutionState] = useState<ExecutionState>("idle");
  const [currentTaskId, setCurrentTaskId] = useState<FixedTaskId | null>(null);
  const [mirrorProgress, setMirrorProgress] = useState<MirrorProgressPayload | null>(null);

  /* 折叠展开状态 */
  const [expandedMap, setExpandedMap] = useState<Record<string, boolean>>({
    set_windows: false,
    daily_task: false,
    get_reward: false,
    buy_enkephalin: false,
    mirror: false,
  });

  /* 结束后操作弹窗 */
  const [afterModalOpen, setAfterModalOpen] = useState(false);

  /* 运行日志 */
  const [logs, setLogs] = useState<(LogEntryPayload & { id: number })[]>([]);
  const logIdRef = useRef(0);
  const logsEndRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    let cancelled = false;
    const unsubs: Array<() => void> = [];

    void (async () => {
      const ipc = await getIpc();
      const [cfg, teamList, exec] = await Promise.all([
        ipc.request<TasksConfig>("tasks.getConfig"),
        ipc.request<TeamSummary[]>("team.list"),
        ipc.request<ExecutionStatusPayload>("execution.getState"),
      ]);

      if (cancelled) return;
      setTasksConfig(cfg);
      setTeams(teamList);
      setExecutionState(exec.state);
      setCurrentTaskId(exec.currentTaskId);

      unsubs.push(
        ipc.on("execution.status", (payload) => {
          const { state, currentTaskId: curId } = payload as ExecutionStatusPayload;
          setExecutionState(state);
          setCurrentTaskId(curId);
          if (state === "idle") {
            setMirrorProgress(null);
          }
        }),
        ipc.on("execution.mirrorProgress", (payload) => {
          setMirrorProgress(payload as MirrorProgressPayload);
        }),
        ipc.on("log.entry", (payload) => {
          setLogs((prev) => [
            ...prev.slice(-299),
            { ...(payload as LogEntryPayload), id: ++logIdRef.current },
          ]);
        }),
      );
    })();

    return () => {
      cancelled = true;
      for (const unsub of unsubs) unsub();
    };
  }, []);

  useEffect(() => {
    if (logs.length === 0) return;
    logsEndRef.current?.scrollIntoView({ behavior: "smooth" });
  }, [logs]);

  /* 同步保存配置 */
  const updateTasksConfig = (patchFn: (prev: TasksConfig) => TasksConfig) => {
    setTasksConfig((prev) => {
      if (!prev) return prev;
      const next = patchFn(prev);
      void (async () => {
        const ipc = await getIpc();
        await ipc.request("tasks.setConfig", next);
      })();
      return next;
    });
  };

  const toggleTaskEnabled = (taskId: keyof TasksConfig["enabledTasks"]) => {
    updateTasksConfig((cfg) => ({
      ...cfg,
      enabledTasks: {
        ...cfg.enabledTasks,
        [taskId]: !cfg.enabledTasks[taskId],
      },
    }));
  };

  const toggleExpanded = (key: string) => {
    setExpandedMap((prev) => ({
      ...prev,
      [key]: !prev[key],
    }));
  };

  const handleSelectAll = () => {
    updateTasksConfig((cfg) => ({
      ...cfg,
      enabledTasks: {
        ...cfg.enabledTasks,
        daily_task: true,
        get_reward: true,
        buy_enkephalin: true,
        mirror: true,
      },
    }));
  };

  const handleClearAll = () => {
    updateTasksConfig((cfg) => ({
      ...cfg,
      enabledTasks: {
        ...cfg.enabledTasks,
        daily_task: false,
        get_reward: false,
        buy_enkephalin: false,
        mirror: false,
      },
    }));
  };

  const handleStart = async () => {
    if (!tasksConfig) return;
    const { daily_task, get_reward, buy_enkephalin, mirror } = tasksConfig.enabledTasks;
    if (!daily_task && !get_reward && !buy_enkephalin && !mirror) {
      toast.warning(t("home.warnNoTaskSelected"));
      return;
    }
    const ipc = await getIpc();
    await ipc.request("execution.start");
  };

  const handleStop = async () => {
    const ipc = await getIpc();
    await ipc.request("execution.stop");
  };

  const handlePause = async () => {
    const ipc = await getIpc();
    await ipc.request("execution.pause");
  };

  const handleResume = async () => {
    const ipc = await getIpc();
    await ipc.request("execution.resume");
  };

  const handleStartRef = useRef(handleStart);
  handleStartRef.current = handleStart;
  const handleStopRef = useRef(handleStop);
  handleStopRef.current = handleStop;

  /* 监听系统托盘事件 (开始/停止任务) */
  useEffect(() => {
    if (!isTauri()) return;
    let unlistenStart: (() => void) | null = null;
    let unlistenStop: (() => void) | null = null;

    void (async () => {
      const { listen } = await import("@tauri-apps/api/event");
      unlistenStart = await listen("tray-start-tasks", () => {
        void handleStartRef.current();
      });
      unlistenStop = await listen("tray-stop-tasks", () => {
        void handleStopRef.current();
      });
    })();

    return () => {
      unlistenStart?.();
      unlistenStop?.();
    };
  }, []);

  const handleSaveAfterCompletion = (afterConfig: AfterCompletionConfig) => {
    updateTasksConfig((cfg) => ({
      ...cfg,
      afterCompletion: afterConfig,
    }));
  };

  /* 拖拽分隔条调宽（对齐 MXU）：280~800px，向右拖到底(<160)折叠 */
  const startResize = () => {
    const MIN_LEFT_PANEL_WIDTH = 460;
    const onMove = (ev: MouseEvent) => {
      const newWidth = document.body.clientWidth - ev.clientX;
      if (newWidth < 160) {
        setRightPanelCollapsed(true);
        return;
      }
      const maxWidth = Math.min(800, document.body.clientWidth - MIN_LEFT_PANEL_WIDTH - 4);
      setRightPanelWidth(Math.max(280, Math.min(maxWidth, newWidth)));
      setRightPanelCollapsed(false);
    };
    const onUp = () => {
      document.removeEventListener("mousemove", onMove);
      document.removeEventListener("mouseup", onUp);
      document.body.style.cursor = "";
      document.body.style.userSelect = "";
    };
    document.addEventListener("mousemove", onMove);
    document.addEventListener("mouseup", onUp);
    document.body.style.cursor = "col-resize";
    document.body.style.userSelect = "none";
  };

  if (!tasksConfig) {
    return (
      <div className="flex h-full items-center justify-center text-xs text-muted-foreground">
        Loading tasks configuration...
      </div>
    );
  }

  const isBusy = executionState !== "idle";

  /* 选项预览 Tag 计算 (对齐 MXU OptionPreviewTag) */
  const windowTags: PreviewTag[] = [
    { label: "分辨率", value: `${tasksConfig.set_windows.set_win_size}P` },
    {
      label: "异步输入",
      value: tasksConfig.set_windows.use_post_message ? "开" : "关",
      highlight: tasksConfig.set_windows.use_post_message,
    },
  ];

  const dailyTags: PreviewTag[] = [
    { label: "经验本", value: `×${tasksConfig.daily_task.set_EXP_count}` },
    { label: "纽本", value: `×${tasksConfig.daily_task.set_thread_count}` },
    {
      label: "连战",
      value: tasksConfig.daily_task.use_continuous_combat
        ? `×${tasksConfig.daily_task.use_continuous_combat_select}`
        : "关",
      highlight: tasksConfig.daily_task.use_continuous_combat,
    },
  ];

  const rewardModeMap = ["全部", "狂气/通行证", "邮件"];
  const rewardTags: PreviewTag[] = [
    { label: "模式", value: rewardModeMap[tasksConfig.get_reward.set_get_prize] ?? "全部" },
  ];

  const enkephalinTags: PreviewTag[] = [
    { label: "换体", value: `${tasksConfig.buy_enkephalin.set_lunacy_to_enkephalin}次` },
    {
      label: "葛朗台",
      value: tasksConfig.buy_enkephalin.Dr_Grandet_mode ? "开" : "关",
      highlight: tasksConfig.buy_enkephalin.Dr_Grandet_mode,
    },
  ];

  const mirrorTags: PreviewTag[] = [
    {
      label: "坐牢",
      value: tasksConfig.mirror.infinite_dungeons
        ? "∞"
        : `${tasksConfig.mirror.set_mirror_count}次`,
      highlight: tasksConfig.mirror.infinite_dungeons,
    },
    {
      label: "难度",
      value: tasksConfig.mirror.hard_mirror ? "困难" : "普通",
      highlight: tasksConfig.mirror.hard_mirror,
    },
  ];

  return (
    <div className="flex h-full min-h-0 overflow-hidden bg-background">
      {/* 左侧：固定任务卡片流 + 底部工具栏 (Flex-1) */}
      <section className="flex min-w-0 flex-1 flex-col border-r border-border bg-background">
        {/* 顶部标题栏状态 */}
        <div className="flex shrink-0 items-center justify-between border-b border-border/60 px-5 py-3 bg-card/30">
          <div className="flex items-center gap-2.5">
            <h1 className="text-sm font-semibold text-foreground">{t("home.taskSectionTitle")}</h1>
            <Badge
              variant={isBusy ? "default" : "secondary"}
              className={cn(
                "h-5 text-[11px] font-normal transition-colors",
                isBusy && "bg-brand text-brand-foreground animate-pulse",
              )}
            >
              {executionState === "running" && currentTaskId
                ? t("home.runningStatus", { task: t(`tasks.titles.${currentTaskId}`) })
                : executionState === "paused"
                  ? t("home.pausedStatus")
                  : t("home.idleStatus")}
            </Badge>
          </div>
        </div>

        {/* 任务卡片滚动流 */}
        <ScrollArea className="min-h-0 flex-1 px-4 py-3">
          <div className="flex flex-col gap-2.5 pb-2">
            {/* 1. 窗口设置 (常开，无勾选框，纯配置入口) */}
            <FixedTaskCard
              id="set_windows"
              title={t("tasks.titles.set_windows")}
              icon={Sliders}
              enabled={true}
              canDisable={false}
              expanded={expandedMap.set_windows}
              isExecuting={currentTaskId === "set_windows"}
              previewTags={windowTags}
              onToggleExpanded={() => toggleExpanded("set_windows")}
            >
              <SetWindowsOptions
                config={tasksConfig.set_windows}
                onChange={(patch) =>
                  updateTasksConfig((c) => ({
                    ...c,
                    set_windows: { ...c.set_windows, ...patch },
                  }))
                }
                disabled={isBusy}
              />
            </FixedTaskCard>

            {/* 2. 日常任务 */}
            <FixedTaskCard
              id="daily_task"
              title={t("tasks.titles.daily_task")}
              icon={CalendarCheck}
              enabled={tasksConfig.enabledTasks.daily_task}
              expanded={expandedMap.daily_task}
              isExecuting={currentTaskId === "daily_task"}
              previewTags={dailyTags}
              onToggleEnabled={() => toggleTaskEnabled("daily_task")}
              onToggleExpanded={() => toggleExpanded("daily_task")}
            >
              <DailyTaskOptions
                config={tasksConfig.daily_task}
                teams={teams}
                onChange={(patch) =>
                  updateTasksConfig((c) => ({
                    ...c,
                    daily_task: { ...c.daily_task, ...patch },
                  }))
                }
                disabled={isBusy}
              />
            </FixedTaskCard>

            {/* 3. 领取奖励 */}
            <FixedTaskCard
              id="get_reward"
              title={t("tasks.titles.get_reward")}
              icon={Gift}
              enabled={tasksConfig.enabledTasks.get_reward}
              expanded={expandedMap.get_reward}
              isExecuting={currentTaskId === "get_reward"}
              previewTags={rewardTags}
              onToggleEnabled={() => toggleTaskEnabled("get_reward")}
              onToggleExpanded={() => toggleExpanded("get_reward")}
            >
              <GetRewardOptions
                config={tasksConfig.get_reward}
                onChange={(patch) =>
                  updateTasksConfig((c) => ({
                    ...c,
                    get_reward: { ...c.get_reward, ...patch },
                  }))
                }
                disabled={isBusy}
              />
            </FixedTaskCard>

            {/* 4. 狂气换体 */}
            <FixedTaskCard
              id="buy_enkephalin"
              title={t("tasks.titles.buy_enkephalin")}
              icon={Zap}
              enabled={tasksConfig.enabledTasks.buy_enkephalin}
              expanded={expandedMap.buy_enkephalin}
              isExecuting={currentTaskId === "buy_enkephalin"}
              previewTags={enkephalinTags}
              onToggleEnabled={() => toggleTaskEnabled("buy_enkephalin")}
              onToggleExpanded={() => toggleExpanded("buy_enkephalin")}
            >
              <BuyEnkephalinOptions
                config={tasksConfig.buy_enkephalin}
                onChange={(patch) =>
                  updateTasksConfig((c) => ({
                    ...c,
                    buy_enkephalin: { ...c.buy_enkephalin, ...patch },
                  }))
                }
                disabled={isBusy}
              />
            </FixedTaskCard>

            {/* 5. 坐牢设置 (镜牢) */}
            <FixedTaskCard
              id="mirror"
              title={t("tasks.titles.mirror")}
              icon={Compass}
              enabled={tasksConfig.enabledTasks.mirror}
              expanded={expandedMap.mirror}
              isExecuting={currentTaskId === "mirror"}
              previewTags={mirrorTags}
              onToggleEnabled={() => toggleTaskEnabled("mirror")}
              onToggleExpanded={() => toggleExpanded("mirror")}
            >
              <MirrorOptions
                config={tasksConfig.mirror}
                progress={mirrorProgress}
                onChange={(patch) =>
                  updateTasksConfig((c) => ({
                    ...c,
                    mirror: { ...c.mirror, ...patch },
                  }))
                }
                disabled={isBusy}
              />
            </FixedTaskCard>

            {/* 6. 亚哈共鸣 */}
            <FixedTaskCard
              id="resonate_with_Ahab"
              title={t("tasks.titles.resonate_with_Ahab")}
              icon={Radio}
              enabled={tasksConfig.enabledTasks.resonate_with_Ahab}
              expanded={false}
              isExecuting={currentTaskId === "resonate_with_Ahab"}
              previewTags={[
                {
                  label: "语录",
                  value: tasksConfig.enabledTasks.resonate_with_Ahab ? "开启" : "关闭",
                  highlight: tasksConfig.enabledTasks.resonate_with_Ahab,
                },
              ]}
              onToggleEnabled={() => toggleTaskEnabled("resonate_with_Ahab")}
            />
          </div>
        </ScrollArea>

        {/* 底部吸底控制栏 (Toolbar) */}
        <ExecutionToolbar
          executionState={executionState}
          afterCompletion={tasksConfig.afterCompletion}
          onSelectAll={handleSelectAll}
          onClearAll={handleClearAll}
          onOpenAfterCompletion={() => setAfterModalOpen(true)}
          onStart={handleStart}
          onStop={handleStop}
          onPause={handlePause}
          onResume={handleResume}
          startShortcut="F10"
        />
      </section>

      {/* 分隔条：对齐 MXU —— 拖动调宽，向右到底折叠 */}
      {/* biome-ignore lint/a11y/noStaticElementInteractions: 鼠标拖拽把手，调整面板宽度 */}
      <div
        onMouseDown={startResize}
        title={t("home.resizeHint")}
        className={cn(
          "group flex shrink-0 cursor-col-resize select-none items-center justify-center bg-transparent transition-all",
          rightPanelCollapsed ? "w-4 hover:bg-brand/20" : "w-1 hover:bg-brand/50",
        )}
      >
        <div className="h-8 w-[2px] rounded-full bg-border transition-colors group-hover:bg-brand" />
      </div>

      {/* 右侧信息面板：连接 → 截图 → 日志 */}
      {!rightPanelCollapsed && (
        <aside
          className="flex flex-col gap-3 overflow-x-hidden border-l border-transparent bg-background p-3"
          style={{ width: rightPanelWidth, minWidth: 240, flexShrink: 1 }}
        >
          {/* 设备连接 */}
          <ConnectionPanel />

          {/* 实时画面卡片 */}
          <div className="flex shrink-0 flex-col overflow-hidden rounded-lg border border-border bg-card">
            <div className="flex h-9 shrink-0 items-center justify-between px-3">
              <span className="flex items-center gap-1.5 text-xs font-medium text-muted-foreground">
                <MonitorPlay className="size-3.5" /> {t("home.screenshotTitle")}
              </span>
            </div>
            <div className="border-t border-border p-3">
              <div className="flex aspect-video w-full flex-col items-center justify-center gap-2 overflow-hidden rounded-md bg-muted text-xs text-muted-foreground">
                <MonitorPlay className="size-8 opacity-25" strokeWidth={1.5} />
                <span>{t("home.screenshotPending")}</span>
                <span className="font-mono text-[10px] opacity-50">16:9 · 1280×720</span>
              </div>
            </div>
          </div>

          {/* 日志卡片（占满剩余高度） */}
          <div className="flex min-h-0 flex-1 flex-col overflow-hidden rounded-lg border border-border bg-card">
            <div className="flex h-9 shrink-0 items-center justify-between border-b border-border px-3">
              <span className="flex items-center gap-1.5 text-xs font-medium text-muted-foreground">
                <ScrollText className="size-3.5" /> {t("home.logsTab")}
                <Badge variant="secondary" className="px-1.5 font-mono">
                  {logs.length}
                </Badge>
              </span>
              <Button
                variant="ghost"
                size="sm"
                className="h-6 gap-1 px-2 text-xs text-muted-foreground hover:text-foreground"
                onClick={() => setLogs([])}
              >
                <Trash2 className="size-3" /> {t("home.logsClear")}
              </Button>
            </div>
            <ScrollArea className="min-h-0 flex-1 px-3 py-2 font-mono text-xs leading-relaxed">
              {logs.map((log) => (
                <div key={log.id} className="flex gap-2 py-0.5">
                  <span className="shrink-0 text-muted-foreground">
                    {new Date(log.ts).toLocaleTimeString()}
                  </span>
                  <LogMessage level={log.level} message={log.message} />
                </div>
              ))}
              <div ref={logsEndRef} />
            </ScrollArea>
          </div>
        </aside>
      )}

      {/* 结束后操作配置弹窗 */}
      <AfterCompletionModal
        open={afterModalOpen}
        onOpenChange={setAfterModalOpen}
        config={tasksConfig.afterCompletion}
        onSave={handleSaveAfterCompletion}
      />
    </div>
  );
}

function LogMessage({ level, message }: { level: string; message: string }) {
  if (level === "error") {
    return (
      <span className="inline-flex items-center gap-1 text-danger">
        <CircleAlert className="size-3 shrink-0" /> {message}
      </span>
    );
  }
  if (level === "warn") {
    return <span className="text-warning">{message}</span>;
  }
  return <span>{message}</span>;
}
