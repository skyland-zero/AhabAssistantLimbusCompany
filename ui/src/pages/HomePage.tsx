import { CircleAlert, MonitorPlay, Play, Plus, ScrollText, Square } from "lucide-react";
import { useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent } from "@/components/ui/card";
import { ScrollArea } from "@/components/ui/scroll-area";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { cn } from "@/lib/utils";
import { getIpc } from "@/services/ipc/client";
import type { LogEntryPayload, QueueTask } from "@/services/ipc/types";

export function HomePage() {
  const { t } = useTranslation();
  const [tasks, setTasks] = useState<QueueTask[]>([]);
  const [logs, setLogs] = useState<(LogEntryPayload & { id: number })[]>([]);
  const logIdRef = useRef(0);
  const logsEndRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    let unsub: (() => void) | undefined;
    void (async () => {
      const ipc = await getIpc();
      setTasks(await ipc.request<QueueTask[]>("task.list"));
      unsub = ipc.on("log.entry", (payload) => {
        setLogs((prev) => [
          ...prev.slice(-199),
          { ...(payload as LogEntryPayload), id: ++logIdRef.current },
        ]);
      });
    })();
    return () => void unsub?.();
  }, []);

  useEffect(() => {
    if (logs.length === 0) return;
    logsEndRef.current?.scrollIntoView({ behavior: "smooth" });
  }, [logs]);

  return (
    <div className="flex h-full min-h-0">
      {/* 左：任务队列 */}
      <section className="flex min-w-0 flex-1 flex-col gap-3 p-4">
        <div className="flex items-center justify-between">
          <h1 className="text-lg font-semibold">{t("home.queueTitle")}</h1>
          <Button size="sm" className="bg-brand text-white hover:bg-brand-hover">
            <Plus className="size-4" /> {t("home.addTask")}
          </Button>
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
                    </div>
                    {task.detail && (
                      <p className="truncate text-xs text-muted-foreground">{task.detail}</p>
                    )}
                  </div>
                  <div className="flex gap-1.5">
                    {task.status !== "running" ? (
                      <Button size="icon" variant="ghost" aria-label="start" disabled>
                        <Play className="size-4 text-green-600" />
                      </Button>
                    ) : (
                      <Button size="icon" variant="ghost" aria-label="stop" disabled>
                        <Square className="size-4 text-red-500" />
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

      {/* 右：可折叠侧栏（M0 固定展开，Tab 切换，按 MXU 交互） */}
      <aside className="hidden w-[380px] shrink-0 border-l border-border md:block">
        <Tabs defaultValue="logs" className="flex h-full min-h-0 flex-col gap-0">
          <TabsList className="w-full justify-start rounded-none border-b bg-transparent p-0">
            <TabsTrigger
              value="screenshot"
              className="gap-1.5 rounded-none data-[state=active]:border-b-2"
            >
              <MonitorPlay className="size-4" /> {t("home.screenshotTab")}
            </TabsTrigger>
            <TabsTrigger
              value="logs"
              className="gap-1.5 rounded-none data-[state=active]:border-b-2"
            >
              <ScrollText className="size-4" /> {t("home.logsTab")}
            </TabsTrigger>
          </TabsList>

          <TabsContent value="screenshot" className="mt-0 min-h-0 flex-1">
            <div className="flex h-full items-center justify-center text-sm text-muted-foreground">
              实时画面（等待后端接入）
            </div>
          </TabsContent>

          <TabsContent value="logs" className="mt-0 min-h-0 flex-1">
            <ScrollArea className="h-full px-3 py-2 font-mono text-xs leading-relaxed">
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
          </TabsContent>
        </Tabs>
      </aside>
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
        status === "queued" && "bg-yellow-500",
        status === "failed" && "bg-red-500",
        status === "done" && "bg-green-500",
      )}
    />
  );
}

function LogMessage({ level, message }: { level: string; message: string }) {
  if (level === "error") {
    return (
      <span className="inline-flex items-center gap-1 text-red-500">
        <CircleAlert className="size-3 shrink-0" /> {message}
      </span>
    );
  }
  if (level === "warn") {
    return <span className="text-yellow-600 dark:text-yellow-400">{message}</span>;
  }
  return <span>{message}</span>;
}
