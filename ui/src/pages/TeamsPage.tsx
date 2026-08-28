import { Pencil, Plus, Sparkles, Trash2, UsersRound } from "lucide-react";
import { lazy, Suspense, useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { EmptyState } from "@/components/common/EmptyState";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent } from "@/components/ui/card";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { ScrollArea } from "@/components/ui/scroll-area";
import { Tabs, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { cn } from "@/lib/utils";
import { getIpc } from "@/services/ipc/client";
import {
  createDefaultMirrorConfig,
  type SinnerInfo,
  type TeamDetail,
  type TeamPurpose,
} from "@/services/ipc/types";

type FilterPurpose = "all" | TeamPurpose;
const PURPOSES: TeamPurpose[] = ["mirror", "luxcavation", "general"];

const purposeKey: Record<TeamPurpose, string> = {
  mirror: "teams.purposeMirror",
  luxcavation: "teams.purposeLuxcavation",
  general: "teams.purposeGeneral",
};

const schemeKey: Record<string, string> = {
  burn: "teams.schemeBurn",
  bleed: "teams.schemeBleed",
  tremor: "teams.schemeTremor",
  rupture: "teams.schemeRupture",
  sinking: "teams.schemeSinking",
  poise: "teams.schemePoise",
  charge: "teams.schemeCharge",
  slash: "teams.schemeSlash",
  pierce: "teams.schemePierce",
  blunt: "teams.schemeBlunt",
};

// 编辑器体积较大，仅在用户真正打开编辑弹窗时加载。
const TeamEditModal = lazy(() =>
  import("@/components/teams/TeamEditModal").then(({ TeamEditModal }) => ({
    default: TeamEditModal,
  })),
);

export function TeamsPage() {
  const { t } = useTranslation();
  const [teams, setTeams] = useState<TeamDetail[]>([]);
  const [sinners, setSinners] = useState<SinnerInfo[]>([]);
  const [activeTab, setActiveTab] = useState<FilterPurpose>("all");
  /** null = 关闭；否则为正在编辑的队伍 */
  const [editing, setEditing] = useState<TeamDetail | null>(null);
  const [modalOpen, setModalOpen] = useState(false);
  const [deleteTarget, setDeleteTarget] = useState<TeamDetail | null>(null);

  useEffect(() => {
    void (async () => {
      const ipc = await getIpc();
      setTeams(await ipc.request<TeamDetail[]>("team.list"));
      setSinners(await ipc.request<SinnerInfo[]>("sinner.list"));
    })();
  }, []);

  const reload = async () => {
    const ipc = await getIpc();
    setTeams(await ipc.request<TeamDetail[]>("team.list"));
  };

  const openNew = () => {
    setEditing({
      id: "",
      name: "",
      purpose: activeTab === "all" ? "general" : activeTab,
      sinners: [],
      accessoryScheme: "burn",
      enabled: true,
      mirrorConfig: createDefaultMirrorConfig(),
    });
    setModalOpen(true);
  };

  const openEdit = (team: TeamDetail) => {
    setEditing(structuredClone(team));
    setModalOpen(true);
  };

  const save = async (savedTeam: TeamDetail) => {
    if (!savedTeam.name.trim()) return;
    await (await getIpc()).request("team.save", savedTeam);
    setModalOpen(false);
    setEditing(null);
    await reload();
  };

  const confirmRemove = async () => {
    if (!deleteTarget) return;
    await (await getIpc()).request("team.delete", { id: deleteTarget.id });
    setDeleteTarget(null);
    await reload();
  };

  const sinnerName = (id: string) => sinners.find((s) => s.id === id)?.name ?? id;

  const filteredTeams = activeTab === "all" ? teams : teams.filter((t) => t.purpose === activeTab);

  return (
    <div className="flex h-full min-h-0 flex-col">
      {/* 顶部栏：用途分类切换 Tab + 新建队伍按钮 */}
      <div className="flex shrink-0 items-center justify-between bg-card/30 px-4 py-2">
        <Tabs value={activeTab} onValueChange={(v) => setActiveTab(v as FilterPurpose)}>
          <TabsList className="h-8 p-0.5 bg-muted/60">
            <TabsTrigger value="all" className="h-7 px-3 text-xs data-[state=active]:bg-background">
              <span>{t("teams.purposeAll")}</span>
              <span className="ml-1.5 rounded-full bg-muted-foreground/15 px-1.5 py-0.2 text-[10px] font-mono">
                {teams.length}
              </span>
            </TabsTrigger>
            {PURPOSES.map((p) => {
              const count = teams.filter((t) => t.purpose === p).length;
              return (
                <TabsTrigger
                  key={p}
                  value={p}
                  className="h-7 px-3 text-xs data-[state=active]:bg-background"
                >
                  <span>{t(purposeKey[p])}</span>
                  {count > 0 && (
                    <span className="ml-1.5 rounded-full bg-muted-foreground/15 px-1.5 py-0.2 text-[10px] font-mono">
                      {count}
                    </span>
                  )}
                </TabsTrigger>
              );
            })}
          </TabsList>
        </Tabs>

        <Button
          size="sm"
          className="h-8 gap-1.5 bg-brand text-brand-foreground hover:bg-brand-hover text-xs"
          onClick={openNew}
        >
          <Plus className="size-3.5" /> {t("teams.newTeam")}
        </Button>
      </div>

      {/* 队伍卡片列表 */}
      <ScrollArea className="min-h-0 flex-1">
        <div className="p-4">
          {teams.length === 0 ? (
            <EmptyState
              icon={UsersRound}
              title={t("teams.emptyTitle")}
              description={t("teams.emptyDesc")}
            />
          ) : filteredTeams.length === 0 ? (
            <EmptyState
              icon={UsersRound}
              title={t("teams.emptyCategoryTitle")}
              description={t("teams.emptyCategoryDesc")}
            />
          ) : (
            <div className="grid grid-cols-1 gap-2 xl:grid-cols-2">
              {filteredTeams.map((team) => {
                const mc = team.mirrorConfig;
                const hasStarlight = mc?.opening_bonus?.some((b) => b > 0);
                const discardedCount = mc?.discard_systems
                  ? Object.values(mc.discard_systems).filter(Boolean).length
                  : 0;
                return (
                  <Card
                    key={team.id}
                    className={cn(
                      "py-0 gap-0 transition-colors duration-150 hover:bg-muted/30",
                      !team.enabled && "opacity-60",
                    )}
                  >
                    <CardContent className="flex items-start gap-3.5 px-3.5 py-3">
                      <div className="min-w-0 flex-1 space-y-2">
                        <div className="flex items-center gap-2 flex-wrap">
                          <span className="truncate text-sm font-semibold">{team.name}</span>
                          <Badge variant="secondary" className="h-5 text-[11px] font-normal">
                            {t(purposeKey[team.purpose])}
                          </Badge>
                          <Badge variant="outline" className="h-5 gap-1 text-[11px] font-normal">
                            <img
                              src={`/status_effects/${team.accessoryScheme || "burn"}.png`}
                              alt=""
                              className="size-3.5 object-contain"
                            />
                            <span>{t(schemeKey[team.accessoryScheme] ?? "teams.schemeBurn")}</span>
                          </Badge>
                          {!team.enabled && (
                            <Badge
                              variant="outline"
                              className="h-5 text-[11px] text-muted-foreground"
                            >
                              {t("themePacks.disabledPack")}
                            </Badge>
                          )}
                        </div>
                        <div className="flex flex-wrap items-center gap-1.5 text-[11px] text-muted-foreground">
                          <span>{t("teams.memberCount", { n: team.sinners.length })}</span>
                          {hasStarlight && (
                            <span className="inline-flex items-center text-amber-500 font-mono">
                              <Sparkles className="size-3 mr-0.5" />
                              {t("teams.starlightReady")}
                            </span>
                          )}
                          {mc?.second_system && (
                            <Badge
                              variant="secondary"
                              className="h-4.5 px-1.5 text-[10px] font-normal"
                            >
                              {t("teams.secondSystem")}
                            </Badge>
                          )}
                          {discardedCount > 0 && (
                            <Badge
                              variant="secondary"
                              className="h-4.5 px-1.5 text-[10px] font-normal text-destructive"
                            >
                              {t("teams.discardCount", { count: discardedCount })}
                            </Badge>
                          )}
                          {mc?.defense_for_solo && (
                            <Badge
                              variant="secondary"
                              className="h-4.5 px-1.5 text-[10px] font-normal text-brand"
                            >
                              {t("teams.soloPass")}
                            </Badge>
                          )}
                          {mc?.use_team_code && (
                            <Badge variant="outline" className="h-4.5 px-1.5 text-[10px] font-mono">
                              {t("teams.teamCode")}
                            </Badge>
                          )}
                        </div>
                        <div className="flex flex-wrap gap-1 pt-0.5">
                          {team.sinners.map((id, idx) => (
                            <Badge
                              key={id}
                              variant="outline"
                              className="h-5 px-1.5 text-[11px] font-normal gap-1"
                            >
                              <span className="font-mono text-[10px] text-brand font-bold">
                                #{idx + 1}
                              </span>
                              <span>{sinnerName(id)}</span>
                            </Badge>
                          ))}
                        </div>
                      </div>
                      <div className="flex shrink-0 gap-1">
                        <Button
                          size="icon"
                          variant="ghost"
                          className="size-8"
                          aria-label={t("teams.editTeam")}
                          onClick={() => openEdit(team)}
                        >
                          <Pencil className="size-4" />
                        </Button>
                        <Button
                          size="icon"
                          variant="ghost"
                          className="size-8"
                          aria-label={t("teams.delete")}
                          onClick={() => setDeleteTarget(team)}
                        >
                          <Trash2 className="size-4 text-destructive" />
                        </Button>
                      </div>
                    </CardContent>
                  </Card>
                );
              })}
            </div>
          )}
        </div>
      </ScrollArea>

      {/* 5-Tab 完整镜牢策略编辑模态框：按需加载，避免列表页常驻编辑器代码与状态 */}
      {modalOpen && editing && (
        <Suspense fallback={null}>
          <TeamEditModal
            open={modalOpen}
            team={editing}
            sinners={sinners}
            onClose={() => {
              setModalOpen(false);
              setEditing(null);
            }}
            onSave={(tData) => void save(tData)}
          />
        </Suspense>
      )}

      {/* 删除确认 - 组件库 Dialog */}
      <Dialog open={!!deleteTarget} onOpenChange={(open) => !open && setDeleteTarget(null)}>
        <DialogContent className="sm:max-w-[400px]">
          <DialogHeader>
            <DialogTitle>{t("teams.deleteConfirmTitle")}</DialogTitle>
            <DialogDescription>
              {deleteTarget
                ? t("teams.deleteConfirmDesc", { name: deleteTarget.name })
                : t("teams.deleteConfirm")}
            </DialogDescription>
          </DialogHeader>
          <DialogFooter className="gap-2 sm:gap-2">
            <Button
              variant="outline"
              size="sm"
              className="h-8 text-xs"
              onClick={() => setDeleteTarget(null)}
            >
              {t("teams.cancel")}
            </Button>
            <Button
              variant="destructive"
              size="sm"
              className="h-8 text-xs"
              onClick={() => void confirmRemove()}
            >
              {t("teams.delete")}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </div>
  );
}
