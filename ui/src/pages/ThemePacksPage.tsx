import { ArrowDownWideNarrow, RotateCcw } from "lucide-react";
import { useEffect, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { PageHeader } from "@/components/common/PageHeader";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent } from "@/components/ui/card";
import { Slider } from "@/components/ui/slider";
import { Switch } from "@/components/ui/switch";
import { cn } from "@/lib/utils";
import { getIpc } from "@/services/ipc/client";
import type { ThemePack, ThemePackState } from "@/services/ipc/types";

export function ThemePacksPage() {
  const { t } = useTranslation();
  const [state, setState] = useState<ThemePackState | null>(null);
  const [sortByWeight, setSortByWeight] = useState(false);

  useEffect(() => {
    void (async () => {
      setState(await (await getIpc()).request<ThemePackState>("themePack.list"));
    })();
  }, []);

  /** 更新单个主题包后整体保存 */
  const patchPack = async (id: string, patch: Partial<ThemePack>) => {
    if (!state) return;
    const packs = state.packs.map((p) => (p.id === id ? { ...p, ...patch } : p));
    setState({ ...state, packs });
    await (await getIpc()).request("themePack.updateAll", { packs });
  };

  const setAllEnabled = async (enabled: boolean) => {
    if (!state) return;
    const packs = state.packs.map((p) => ({ ...p, enabled }));
    setState({ ...state, packs });
    await (await getIpc()).request("themePack.updateAll", { packs });
  };

  const resetWeights = async () => {
    const next = await (await getIpc()).request<ThemePackState>("themePack.resetWeights");
    setState(next);
  };

  const sortedPacks = useMemo(() => {
    if (!state) return [];
    const list = [...state.packs];
    if (sortByWeight) list.sort((a, b) => b.weight - a.weight);
    return list;
  }, [state, sortByWeight]);

  const totalWeight = useMemo(
    () => state?.packs.filter((p) => p.enabled).reduce((sum, p) => sum + p.weight, 0) ?? 0,
    [state],
  );

  if (!state) return null;

  return (
    <div className="flex h-full min-h-0 flex-col">
      <PageHeader title={t("pages.themes.title")} description={t("pages.themes.desc")}>
        <Button
          size="sm"
          variant={sortByWeight ? "secondary" : "outline"}
          onClick={() => setSortByWeight((v) => !v)}
        >
          <ArrowDownWideNarrow className="size-4" /> {t("themePacks.sortByWeight")}
        </Button>
        <Button size="sm" variant="outline" onClick={() => void setAllEnabled(true)}>
          {t("themePacks.enableAll")}
        </Button>
        <Button size="sm" variant="outline" onClick={() => void setAllEnabled(false)}>
          {t("themePacks.disableAll")}
        </Button>
        <Button size="sm" variant="ghost" onClick={() => void resetWeights()}>
          <RotateCcw className="size-4" /> {t("themePacks.resetWeights")}
        </Button>
      </PageHeader>

      {/* 困难镜牢提示条 */}
      {state.hardMirrorActive && (
        <div className="border-b border-border bg-warning-light px-6 py-2 text-xs text-warning dark:bg-warning-dark/30">
          ⚠ {t("themePacks.hardMirrorBanner")}
        </div>
      )}

      <div className="min-h-0 flex-1 overflow-y-auto p-6">
        <div className="flex flex-col gap-2">
          {sortedPacks.map((pack) => (
            <Card key={pack.id} className={cn("py-3", !pack.enabled && "opacity-60")}>
              <CardContent className="flex items-center gap-4 px-5">
                <Switch
                  checked={pack.enabled}
                  onCheckedChange={(v) => void patchPack(pack.id, { enabled: v })}
                  aria-label={t("themePacks.enabled")}
                />
                <div className="w-44 shrink-0">
                  <div className="flex items-center gap-2">
                    <span className="truncate text-sm font-medium">{pack.name}</span>
                    <Badge variant="secondary">{pack.tier}</Badge>
                  </div>
                </div>
                <div className="flex min-w-0 flex-1 items-center gap-3">
                  <span className="shrink-0 text-xs text-muted-foreground">
                    {t("themePacks.weight")}
                  </span>
                  <Slider
                    value={[pack.weight]}
                    max={10}
                    step={1}
                    disabled={!pack.enabled}
                    onValueChange={([v]) => void patchPack(pack.id, { weight: v })}
                  />
                  <span className="w-6 shrink-0 text-right font-mono text-xs tabular-nums">
                    {pack.weight}
                  </span>
                </div>
              </CardContent>
            </Card>
          ))}
        </div>

        <p className="mt-4 text-xs text-muted-foreground">
          {t("themePacks.totalWeight")}：<span className="font-mono">{totalWeight}</span>
        </p>
      </div>
    </div>
  );
}
