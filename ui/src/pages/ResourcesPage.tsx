import { RefreshCw, SearchCheck } from "lucide-react";
import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { toast } from "sonner";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent } from "@/components/ui/card";
import { ScrollArea } from "@/components/ui/scroll-area";
import { getIpc } from "@/services/ipc/client";
import type { ResourceGroup, SyncProgressPayload } from "@/services/ipc/types";

export function ResourcesPage() {
  const { t } = useTranslation();
  const [groups, setGroups] = useState<ResourceGroup[]>([]);
  /** null = 空闲；否则为同步进度 0-100 */
  const [syncProgress, setSyncProgress] = useState<number | null>(null);

  useEffect(() => {
    let cancelled = false;
    let unsub: (() => void) | undefined;
    let completionTimer: ReturnType<typeof setTimeout> | null = null;

    void (async () => {
      const ipc = await getIpc();
      const nextGroups = await ipc.request<ResourceGroup[]>("resource.status");
      if (cancelled) return;
      setGroups(nextGroups);

      const dispose = ipc.on("resource.sync.progress", (payload) => {
        const { progress } = payload as SyncProgressPayload;
        setSyncProgress(progress);
        if (progress >= 100 && !cancelled && completionTimer === null) {
          // 进度走完后刷新最终状态并复位
          completionTimer = setTimeout(() => {
            completionTimer = null;
            if (cancelled) return;
            void (async () => {
              const groups = await (await getIpc()).request<ResourceGroup[]>("resource.status");
              if (cancelled) return;
              setGroups(groups);
              setSyncProgress(null);
              toast.success(t("resources.syncDone"));
            })();
          }, 300);
        }
      });
      if (cancelled) dispose();
      else unsub = dispose;
    })();

    return () => {
      cancelled = true;
      unsub?.();
      if (completionTimer) {
        clearTimeout(completionTimer);
        completionTimer = null;
      }
    };
  }, [t]);

  const checkUpdate = async () => {
    setGroups(await (await getIpc()).request<ResourceGroup[]>("resource.checkUpdate"));
  };

  const syncNow = async () => {
    await (await getIpc()).request("resource.sync.start");
  };

  const fmtTime = (ts: number | null) =>
    ts ? new Date(ts).toLocaleString() : t("resources.never");

  return (
    <div className="flex h-full min-h-0 flex-col">
      <div className="flex shrink-0 items-center justify-between border-b border-border/60 bg-card/30 px-5 py-2.5">
        <div className="flex items-center gap-2 text-xs text-muted-foreground">
          {syncProgress !== null && (
            <Badge variant="secondary" className="h-5 text-[11px] font-mono">
              {t("resources.syncing", { progress: syncProgress })}
            </Badge>
          )}
        </div>
        <div className="flex items-center gap-1.5">
          <Button
            size="sm"
            variant="outline"
            className="h-7 text-xs"
            onClick={() => void checkUpdate()}
          >
            <SearchCheck className="size-3.5" /> {t("resources.checkUpdate")}
          </Button>
          <Button
            size="sm"
            className="h-7 text-xs bg-brand text-brand-foreground hover:bg-brand-hover"
            disabled={syncProgress !== null}
            onClick={() => void syncNow()}
          >
            <RefreshCw className={cnSyncIcon(syncProgress)} /> {t("resources.syncNow")}
          </Button>
        </div>
      </div>

      <ScrollArea className="min-h-0 flex-1">
        <div className="p-6">
          <div className="grid max-w-3xl grid-cols-1 gap-3 lg:grid-cols-2">
            {groups.map((group) => {
              const hasUpdate =
                group.remoteVersion !== null && group.remoteVersion !== group.localVersion;
              return (
                <Card key={group.id} className="py-4">
                  <CardContent className="flex flex-col gap-3 px-5">
                    <div className="flex items-center justify-between">
                      <span className="text-sm font-medium">{group.name}</span>
                      {syncProgress !== null ? (
                        <Badge>{t("resources.syncing", { progress: syncProgress })}</Badge>
                      ) : hasUpdate ? (
                        <Badge className="bg-warning-light text-warning dark:bg-warning-dark/40">
                          {t("resources.updateAvailable", { version: group.remoteVersion })}
                        </Badge>
                      ) : (
                        <Badge className="bg-success-light text-success dark:bg-success-dark/40">
                          {t("resources.upToDate")}
                        </Badge>
                      )}
                    </div>
                    <dl className="grid grid-cols-[auto_1fr] gap-x-4 gap-y-1 text-xs">
                      <dt className="text-muted-foreground">{t("resources.localVersion")}</dt>
                      <dd className="font-mono">{group.localVersion}</dd>
                      <dt className="text-muted-foreground">{t("resources.remoteVersion")}</dt>
                      <dd className="font-mono">{group.remoteVersion ?? "—"}</dd>
                      <dt className="text-muted-foreground">{t("resources.lastSync")}</dt>
                      <dd className="font-mono">{fmtTime(group.lastSyncAt)}</dd>
                    </dl>
                    {syncProgress !== null && (
                      <div className="h-1 w-full overflow-hidden rounded-full bg-muted">
                        <div
                          className="h-full rounded-full bg-brand transition-[width]"
                          style={{ width: `${syncProgress}%` }}
                        />
                      </div>
                    )}
                  </CardContent>
                </Card>
              );
            })}
          </div>
        </div>
      </ScrollArea>
    </div>
  );
}

function cnSyncIcon(syncProgress: number | null): string | undefined {
  return syncProgress !== null ? "animate-spin" : undefined;
}
