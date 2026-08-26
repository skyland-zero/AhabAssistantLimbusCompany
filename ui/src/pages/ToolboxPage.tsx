import { Camera, Crosshair, Pill, Play, Square } from "lucide-react";
import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { toast } from "sonner";
import { PageHeader } from "@/components/common/PageHeader";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent } from "@/components/ui/card";
import { cn } from "@/lib/utils";
import { getIpc } from "@/services/ipc/client";
import type { ToolId, ToolStatusPayload } from "@/services/ipc/types";

interface ToolMeta {
  id: ToolId;
  icon: typeof Crosshair;
  titleKey: string;
  descKey: string;
}

const TOOLS: ToolMeta[] = [
  {
    id: "infinite_battle",
    icon: Crosshair,
    titleKey: "toolbox.infiniteBattleTitle",
    descKey: "toolbox.infiniteBattleDesc",
  },
  {
    id: "enkephalin",
    icon: Pill,
    titleKey: "toolbox.enkephalinTitle",
    descKey: "toolbox.enkephalinDesc",
  },
  {
    id: "screenshot",
    icon: Camera,
    titleKey: "toolbox.screenshotTitle",
    descKey: "toolbox.screenshotDesc",
  },
];

export function ToolboxPage() {
  const { t } = useTranslation();
  const [running, setRunning] = useState<Record<string, boolean>>({});

  useEffect(() => {
    let unsub: (() => void) | undefined;
    void (async () => {
      const ipc = await getIpc();
      unsub = ipc.on("tool.status", (payload) => {
        const { toolId, running: isRunning } = payload as ToolStatusPayload;
        setRunning((prev) => ({ ...prev, [toolId]: isRunning }));
      });
    })();
    return () => void unsub?.();
  }, []);

  const toggleTool = async (tool: ToolMeta) => {
    const ipc = await getIpc();
    const method = running[tool.id] ? "tool.stop" : "tool.start";
    await ipc.request(method, { id: tool.id });
  };

  const takeScreenshot = async () => {
    await (await getIpc()).request("tool.screenshot");
    toast.success(t("toolbox.screenshotDone"));
  };

  return (
    <div className="flex h-full min-h-0 flex-col">
      <PageHeader title={t("pages.toolbox.title")} description={t("pages.toolbox.desc")} />

      <div className="min-h-0 flex-1 overflow-y-auto p-6">
        <div className="grid max-w-3xl grid-cols-1 gap-3 lg:grid-cols-3">
          {TOOLS.map((tool) => {
            const Icon = tool.icon;
            const isRunning = Boolean(running[tool.id]);
            const isScreenshot = tool.id === "screenshot";
            return (
              <Card key={tool.id} className="py-4">
                <CardContent className="flex flex-col items-start gap-3 px-4">
                  <div className="flex w-full items-center justify-between">
                    <Icon className="size-5 text-brand" />
                    {isRunning ? (
                      <Badge className="gap-1 bg-success-light text-success dark:bg-success-dark/40">
                        <span className="size-1.5 animate-pulse rounded-full bg-current" />
                        {t("toolbox.running")}
                      </Badge>
                    ) : (
                      <Badge variant="secondary">{isScreenshot ? "—" : t("toolbox.idle")}</Badge>
                    )}
                  </div>
                  <div>
                    <p className="text-sm font-medium">{t(tool.titleKey)}</p>
                    <p className="mt-1 text-xs leading-relaxed text-muted-foreground">
                      {t(tool.descKey)}
                    </p>
                  </div>
                  {isScreenshot ? (
                    <Button
                      size="sm"
                      variant="outline"
                      className="w-full"
                      onClick={() => void takeScreenshot()}
                    >
                      <Camera className="size-4" /> {t("toolbox.run")}
                    </Button>
                  ) : isRunning ? (
                    <Button
                      size="sm"
                      variant="outline"
                      className="w-full text-red-500 hover:text-red-600"
                      onClick={() => void toggleTool(tool)}
                    >
                      <Square className="size-4" /> {t("toolbox.stop")}
                    </Button>
                  ) : (
                    <Button
                      size="sm"
                      className="w-full bg-brand text-brand-foreground hover:bg-brand-hover"
                      onClick={() => void toggleTool(tool)}
                    >
                      <Play className="size-4" /> {t("toolbox.run")}
                    </Button>
                  )}
                </CardContent>
              </Card>
            );
          })}
        </div>

        <p className={cn("mt-4 text-xs text-muted-foreground")}>{t("toolbox.mockNotice")}</p>
      </div>
    </div>
  );
}
