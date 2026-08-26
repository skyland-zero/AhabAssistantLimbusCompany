import {
  CircleAlert,
  Megaphone,
  MonitorPlay,
  PanelRightClose,
  PanelRightOpen,
  Play,
  Plus,
  ScrollText,
  Square,
  Trash2,
} from "lucide-react";
import { useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import Markdown from "react-markdown";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent } from "@/components/ui/card";
import { Dialog, DialogContent, DialogHeader, DialogTitle } from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { ScrollArea } from "@/components/ui/scroll-area";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { cn } from "@/lib/utils";
import { getIpc } from "@/services/ipc/client";
import type {
  AfterCompletionAction,
  LogEntryPayload,
  NoticeItem,
  QueueOptions,
  QueueTask,
  TaskProgressPayload,
  TaskStatusPayload,
} from "@/services/ipc/types";
import { useAppStore } from "@/stores/appStore";

const AC_ACTIONS: AfterCompletionAction[] = ["none", "close_game", "shutdown", "custom"];
const acLabelKey: Record<AfterCompletionAction, string> = {
  none: "home.acNone",
  close_game: "home.acCloseGame",
  shutdown: "home.acShutdown",
  custom: "home.acCustom",
};

const DISMISSED_KEY = "aalc-dismissed-notices";

function loadDismissed(): string[] {
  try {
    return JSON.parse(localStorage.getItem(DISMISSED_KEY) ?? "[]") as string[];
  } catch {
    return [];
  }
}

export function HomePage() {
  const { t } = useTranslation();
  const setCurrentPage = useAppStore((s) => s.setCurrentPage);
  const rightPanelCollapsed = useAppStore((s) => s.rightPanelCollapsed);
  const toggleRightPanel = useAppStore((s) => s.toggleRightPanel);
  const [tasks, setTasks] = useState<QueueTask[]>([]);
  const [teamNames, setTeamNames] = useState<Record<string, string>>({});
  const [options, setOptions] = useState<QueueOptions | null>(null);
  const [notices, setNotices] = useState<NoticeItem[]>([]);
  const [dismissed, setDismissed] = useState<string[]>(loadDismissed);
  const [readingNotice, setReadingNotice] = useState<NoticeItem | null>(null);
  /** 自定义命令对话框 */
  const [customOpen, setCustomOpen] = useState(false);
  const [customCmd, setCustomCmd] = useState("");
  /** 日志面板高度（拖拽分隔条调整） */
  const [logHeight, setLogHeight] = useState(240);
  const [logs, setLogs] = useState<(LogEntryPayload & { id: number })[]>([]);
  const logIdRef = useRef(0);
  const logsEndRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    let cancelled = false;
    const unsubs: Array<() => void> = [];
    void (async () => {
      const ipc = await getIpc();
      const [taskList, teams, opts, noticeList] = await Promise.all([
        ipc.request<QueueTask[]>("task.list"),
        ipc.request<{ id: string; name: string }[]>("team.list"),
        ipc.request<QueueOptions>("queue.getOptions"),
        ipc.request<NoticeItem[]>("notice.list"),
      ]);
      if (cancelled) return;
      setTasks(taskList);
      setTeamNames(Object.fromEntries(teams.map((tm) => [tm.id, tm.name])));
      setOptions(opts);
      setCustomCmd(opts.customCommand ?? "");
      setNotices(noticeList);

      const patchTask = (patch: Partial<QueueTask> & { id: string }) => {
        setTasks((prev) =>
          prev.map((task) => (task.id === patch.id ? { ...task, ...patch } : task)),
        );
      };

      unsubs.push(
        ipc.on("task.status", (payload) => {
          const { taskId, status } = payload as TaskStatusPayload;
          patchTask({ id: taskId, status });
        }),
        ipc.on("task.progress", (payload) => {
          const { taskId, progress } = payload as TaskProgressPayload;
          patchTask({ id: taskId, progress });
        }),
        ipc.on("log.entry", (payload) => {
          setLogs((prev) => [
            ...prev.slice(-199),
            { ...(payload as LogEntryPayload), id: ++logIdRef.current },
          ]);
        }),
      );
    })();
    return () => {
      cancelled = true;
      for (const unsub of unsubs) {
        unsub();
      }
    };
  }, []);

  useEffect(() => {
    if (logs.length === 0) return;
    logsEndRef.current?.scrollIntoView({ behavior: "smooth" });
  }, [logs]);

  const toggleRun = async (task: QueueTask) => {
    const ipc = await getIpc();
    await ipc.request(`task.${task.status === "running" ? "stop" : "start"}`, { id: task.id });
  };

  const changeAfterCompletion = async (action: AfterCompletionAction) => {
    if (!options) return;
    const next = { ...options, afterCompletion: action };
    setOptions(next);
    await (await getIpc()).request("queue.setOption", next);
    if (action === "custom") setCustomOpen(true);
  };

  const saveCustomCommand = async () => {
    if (!options) return;
    const next = { ...options, customCommand: customCmd };
    setOptions(next);
    await (await getIpc()).request("queue.setOption", next);
    setCustomOpen(false);
  };

  const dismissNotice = (id: string) => {
    const next = [...dismissed, id];
    setDismissed(next);
    localStorage.setItem(DISMISSED_KEY, JSON.stringify(next));
  };

  /** 拖拽分隔条调整日志面板高度（MXU 同款交互） */
  const startResize = (e: React.MouseEvent) => {
    e.preventDefault();
    const startY = e.clientY;
    const startH = logHeight;
    const onMove = (ev: MouseEvent) => {
      setLogHeight(Math.min(600, Math.max(120, startH - (ev.clientY - startY))));
    };
    const onUp = () => {
      window.removeEventListener("mousemove", onMove);
      window.removeEventListener("mouseup", onUp);
    };
    window.addEventListener("mousemove", onMove);
    window.addEventListener("mouseup", onUp);
  };

  const topNotice = notices.find((n) => !dismissed.includes(n.id));

  return (
    <div className="flex h-full min-h-0 flex-col">
      {/* 公告横幅（announcement_board 对应） */}
      {topNotice && (
        <div className="flex shrink-0 items-center gap-2 border-b border-border bg-warning-light px-4 py-1.5 text-xs dark:bg-warning/10">
          <Megaphone className="size-3.5 shrink-0 text-warning" />
          <span className="min-w-0 truncate">{topNotice.title}</span>
          <span className="ml-auto flex shrink-0 items-center gap-2">
            <button
              type="button"
              className="font-medium text-brand hover:underline"
              onClick={() => setReadingNotice(topNotice)}
            >
              {t("home.noticeView")}
            </button>
            <button
              type="button"
              className="text-muted-foreground hover:text-foreground"
              onClick={() => dismissNotice(topNotice.id)}
              aria-label={t("home.noticeDismiss")}
            >
              ✕
            </button>
          </span>
        </div>
      )}

      <div className="flex min-h-0 flex-1">
        {/* 左：任务队列 */}
        <section className="flex min-w-0 flex-1 flex-col gap-3 p-4">
          <div className="flex items-center justify-between gap-3">
            <h1 className="text-lg font-semibold">{t("home.queueTitle")}</h1>
            <div className="flex items-center gap-2">
              {/* 完成后动作 */}
              {options && (
                <>
                  <span className="text-xs text-muted-foreground">{t("home.afterCompletion")}</span>
                  <Select
                    value={options.afterCompletion}
                    onValueChange={(v) => void changeAfterCompletion(v as AfterCompletionAction)}
                  >
                    <SelectTrigger size="sm" className="w-32 text-xs">
                      <SelectValue />
                    </SelectTrigger>
                    <SelectContent>
                      {AC_ACTIONS.map((a) => (
                        <SelectItem key={a} value={a}>
                          {t(acLabelKey[a])}
                        </SelectItem>
                      ))}
                    </SelectContent>
                  </Select>
                </>
              )}
              <Button size="sm" className="bg-brand text-brand-foreground hover:bg-brand-hover">
                <Plus className="size-4" /> {t("home.addTask")}
              </Button>
            </div>
          </div>

          <ScrollArea className="min-h-0 flex-1 pr-2">
            <div className="flex flex-col gap-2">
              {tasks.length === 0 && (
                <p className="mt-16 text-center text-sm text-muted-foreground">
                  {t("home.emptyQueue")}
                </p>
              )}
              {tasks.map((task) => (
                <Card key={task.id} className="py-3">
                  <CardContent className="flex items-center gap-3 px-4">
                    <StatusDot status={task.status} />
                    <div className="min-w-0 flex-1">
                      <div className="flex items-center gap-2">
                        <span className="truncate text-sm font-medium">{task.name}</span>
                        {task.repeat !== null && <Badge variant="secondary">×{task.repeat}</Badge>}
                        {task.teamId && teamNames[task.teamId] && (
                          <button type="button" onClick={() => setCurrentPage("teams")}>
                            <Badge
                              variant="outline"
                              className="cursor-pointer text-xs font-normal hover:border-brand hover:text-brand"
                            >
                              {teamNames[task.teamId]}
                            </Badge>
                          </button>
                        )}
                      </div>
                      {task.detail && (
                        <p className="truncate text-xs text-muted-foreground">{task.detail}</p>
                      )}
                      {(task.progress > 0 || task.status === "running") && (
                        <div className="mt-1.5 h-1 w-full overflow-hidden rounded-full bg-muted">
                          <div
                            className={cn(
                              "h-full rounded-full transition-[width] duration-300",
                              task.status === "running" ? "bg-brand" : "bg-success",
                            )}
                            style={{ width: `${task.progress}%` }}
                          />
                        </div>
                      )}
                    </div>
                    <div className="flex gap-1.5">
                      {task.status === "running" ? (
                        <Button
                          size="icon"
                          variant="ghost"
                          aria-label={t("home.stopTask")}
                          onClick={() => void toggleRun(task)}
                        >
                          <Square className="size-4 text-danger" />
                        </Button>
                      ) : (
                        <Button
                          size="icon"
                          variant="ghost"
                          aria-label={t("home.startTask")}
                          onClick={() => void toggleRun(task)}
                        >
                          <Play className="size-4 text-success" />
                        </Button>
                      )}
                    </div>
                  </CardContent>
                </Card>
              ))}
            </div>
          </ScrollArea>

          <p className="text-xs text-muted-foreground">{t("home.mockNotice")}</p>
        </section>

        {/* 右：MXU 式面板 —— 上截图 / 下日志，可拖拽、可收起 */}
        {rightPanelCollapsed ? (
          <aside className="flex w-10 shrink-0 flex-col items-center gap-2 border-l border-border bg-card py-2">
            <Button
              variant="ghost"
              size="icon"
              onClick={toggleRightPanel}
              aria-label={t("home.showPanel")}
              title={t("home.showPanel")}
            >
              <PanelRightOpen className="size-4" />
            </Button>
            <span className="mt-1 text-xs tracking-wide text-muted-foreground [writing-mode:vertical-rl]">
              {t("home.logsTab")} · {logs.length}
            </span>
          </aside>
        ) : (
          <aside className="flex w-[380px] shrink-0 flex-col border-l border-border">
            {/* 上：实时画面 */}
            <div className="flex min-h-0 flex-1 flex-col">
              <div className="flex h-9 shrink-0 items-center justify-between border-b border-border px-3">
                <span className="flex items-center gap-1.5 text-xs font-medium text-muted-foreground">
                  <MonitorPlay className="size-3.5" /> {t("home.screenshotTitle")}
                </span>
                <Button
                  variant="ghost"
                  size="icon"
                  className="size-7"
                  onClick={toggleRightPanel}
                  aria-label={t("home.hidePanel")}
                  title={t("home.hidePanel")}
                >
                  <PanelRightClose className="size-4" />
                </Button>
              </div>
              <div className="flex min-h-0 flex-1 flex-col items-center justify-center gap-2 bg-muted/30 p-4 text-sm text-muted-foreground">
                <MonitorPlay className="size-10 opacity-20" strokeWidth={1.5} />
                <span>{t("home.screenshotPending")}</span>
              </div>
            </div>

            {/* 可拖拽分隔条 */}
            {/* biome-ignore lint/a11y/noStaticElementInteractions: 鼠标拖拽把手，调整日志面板高度 */}
            <div
              onMouseDown={startResize}
              className="h-1 shrink-0 cursor-row-resize bg-border transition-colors hover:bg-brand/60"
            />

            {/* 下：日志 */}
            <div style={{ height: logHeight }} className="flex shrink-0 flex-col">
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
                  className="h-6 gap-1 px-2 text-xs text-muted-foreground"
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
      </div>

      {/* 自定义命令对话框 */}
      <Dialog open={customOpen} onOpenChange={setCustomOpen}>
        <DialogContent className="max-w-md">
          <DialogHeader>
            <DialogTitle>{t("home.customCommand")}</DialogTitle>
          </DialogHeader>
          <Input
            value={customCmd}
            placeholder={t("home.customCommandPlaceholder")}
            onChange={(e) => setCustomCmd(e.target.value)}
          />
          <Button
            className="bg-brand text-brand-foreground hover:bg-brand-hover"
            onClick={() => void saveCustomCommand()}
          >
            OK
          </Button>
        </DialogContent>
      </Dialog>

      {/* 公告详情对话框 */}
      <Dialog
        open={readingNotice !== null}
        onOpenChange={(open) => !open && setReadingNotice(null)}
      >
        <DialogContent className="max-w-lg [&>button]:left-4">
          <DialogHeader>
            <DialogTitle>{readingNotice?.title}</DialogTitle>
          </DialogHeader>
          <ScrollArea className="max-h-[50vh] pr-2 text-sm leading-relaxed [&_li]:my-0.5 [&_ol]:list-decimal [&_ol]:pl-5 [&_p]:my-2 [&_strong]:font-semibold [&_ul]:list-disc [&_ul]:pl-5">
            <Markdown>{readingNotice?.content ?? ""}</Markdown>
          </ScrollArea>
        </DialogContent>
      </Dialog>
    </div>
  );
}

function StatusDot({ status }: { status: QueueTask["status"] }) {
  return (
    <span
      className={cn(
        "size-2 shrink-0 rounded-full",
        status === "running" && "animate-pulse bg-brand",
        status === "idle" && "bg-muted-foreground/40",
        status === "queued" && "bg-warning",
        status === "failed" && "bg-danger",
        status === "done" && "bg-success",
      )}
    />
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
