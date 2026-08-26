import { Pencil, Plus, Trash2, UsersRound } from "lucide-react";
import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { EmptyState } from "@/components/common/EmptyState";
import { PageHeader } from "@/components/common/PageHeader";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent } from "@/components/ui/card";
import {
  Dialog,
  DialogContent,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { Switch } from "@/components/ui/switch";
import { cn } from "@/lib/utils";
import { getIpc } from "@/services/ipc/client";
import type { SinnerInfo, TeamDetail, TeamPurpose } from "@/services/ipc/types";

const PURPOSES: TeamPurpose[] = ["mirror", "luxcavation", "general"];
const SCHEMES = ["burn", "tremor", "rupture", "sinking", "poise", "charge"] as const;

const purposeKey: Record<TeamPurpose, string> = {
  mirror: "teams.purposeMirror",
  luxcavation: "teams.purposeLuxcavation",
  general: "teams.purposeGeneral",
};
const schemeKey: Record<(typeof SCHEMES)[number], string> = {
  burn: "teams.schemeBurn",
  tremor: "teams.schemeTremor",
  rupture: "teams.schemeRupture",
  sinking: "teams.schemeSinking",
  poise: "teams.schemePoise",
  charge: "teams.schemeCharge",
};

export function TeamsPage() {
  const { t } = useTranslation();
  const [teams, setTeams] = useState<TeamDetail[]>([]);
  const [sinners, setSinners] = useState<SinnerInfo[]>([]);
  /** null = 关闭；否则为正在编辑的队伍（新队伍传空壳） */
  const [editing, setEditing] = useState<TeamDetail | null>(null);

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
      purpose: "general",
      sinners: [],
      accessoryScheme: "burn",
      enabled: true,
    });
  };

  const save = async () => {
    if (!editing?.name.trim()) return;
    await (await getIpc()).request("team.save", editing);
    setEditing(null);
    await reload();
  };

  const remove = async (team: TeamDetail) => {
    if (!window.confirm(t("teams.deleteConfirm"))) return;
    await (await getIpc()).request("team.delete", { id: team.id });
    await reload();
  };

  const toggleSinner = (id: string) => {
    if (!editing) return;
    setEditing({
      ...editing,
      sinners: editing.sinners.includes(id)
        ? editing.sinners.filter((s) => s !== id)
        : [...editing.sinners, id],
    });
  };

  const sinnerName = (id: string) => sinners.find((s) => s.id === id)?.name ?? id;

  return (
    <div className="flex h-full min-h-0 flex-col">
      <PageHeader title={t("pages.teams.title")} description={t("pages.teams.desc")}>
        <Button
          size="sm"
          className="bg-brand text-brand-foreground hover:bg-brand-hover"
          onClick={openNew}
        >
          <Plus className="size-4" /> {t("teams.newTeam")}
        </Button>
      </PageHeader>

      <div className="min-h-0 flex-1 overflow-y-auto p-6">
        {teams.length === 0 ? (
          <EmptyState
            icon={UsersRound}
            title={t("teams.emptyTitle")}
            description={t("teams.emptyDesc")}
          />
        ) : (
          <div className="grid grid-cols-1 gap-3 xl:grid-cols-2">
            {teams.map((team) => (
              <Card key={team.id} className={cn("py-4", !team.enabled && "opacity-60")}>
                <CardContent className="flex items-start gap-3 px-5">
                  <div className="min-w-0 flex-1">
                    <div className="flex items-center gap-2">
                      <span className="truncate text-sm font-medium">{team.name}</span>
                      <Badge variant="secondary">{t(purposeKey[team.purpose])}</Badge>
                      {!team.enabled && (
                        <Badge variant="outline">{t("themePacks.disabledPack")}</Badge>
                      )}
                    </div>
                    <p className="mt-1 truncate text-xs text-muted-foreground">
                      {t("teams.scheme")}：
                      {t(
                        schemeKey[team.accessoryScheme as keyof typeof schemeKey] ??
                          "teams.schemeBurn",
                      )}
                      {" · "}
                      {t("teams.memberCount", { n: team.sinners.length })}
                    </p>
                    <div className="mt-2 flex flex-wrap gap-1">
                      {team.sinners.map((id) => (
                        <Badge key={id} variant="outline" className="text-xs font-normal">
                          {sinnerName(id)}
                        </Badge>
                      ))}
                    </div>
                  </div>
                  <div className="flex shrink-0 gap-1">
                    <Button
                      size="icon"
                      variant="ghost"
                      aria-label={t("teams.editTeam")}
                      onClick={() => setEditing(structuredClone(team))}
                    >
                      <Pencil className="size-4" />
                    </Button>
                    <Button
                      size="icon"
                      variant="ghost"
                      aria-label={t("teams.delete")}
                      onClick={() => void remove(team)}
                    >
                      <Trash2 className="size-4 text-destructive" />
                    </Button>
                  </div>
                </CardContent>
              </Card>
            ))}
          </div>
        )}
      </div>

      {/* 编辑对话框 */}
      <Dialog open={editing !== null} onOpenChange={(open) => !open && setEditing(null)}>
        <DialogContent className="max-w-lg">
          <DialogHeader>
            <DialogTitle>{editing?.id ? t("teams.editTeam") : t("teams.newTeam")}</DialogTitle>
          </DialogHeader>

          {editing && (
            <div className="flex flex-col gap-4">
              <div className="flex flex-col gap-1.5">
                <Label>{t("teams.name")}</Label>
                <Input
                  value={editing.name}
                  placeholder={t("teams.namePlaceholder")}
                  onChange={(e) => setEditing({ ...editing, name: e.target.value })}
                />
              </div>

              <div className="grid grid-cols-2 gap-3">
                <div className="flex flex-col gap-1.5">
                  <Label>{t("teams.purpose")}</Label>
                  <Select
                    value={editing.purpose}
                    onValueChange={(v) => setEditing({ ...editing, purpose: v as TeamPurpose })}
                  >
                    <SelectTrigger>
                      <SelectValue />
                    </SelectTrigger>
                    <SelectContent>
                      {PURPOSES.map((p) => (
                        <SelectItem key={p} value={p}>
                          {t(purposeKey[p])}
                        </SelectItem>
                      ))}
                    </SelectContent>
                  </Select>
                </div>
                <div className="flex flex-col gap-1.5">
                  <Label>{t("teams.scheme")}</Label>
                  <Select
                    value={editing.accessoryScheme}
                    onValueChange={(v) => setEditing({ ...editing, accessoryScheme: v })}
                  >
                    <SelectTrigger>
                      <SelectValue />
                    </SelectTrigger>
                    <SelectContent>
                      {SCHEMES.map((s) => (
                        <SelectItem key={s} value={s}>
                          {t(schemeKey[s])}
                        </SelectItem>
                      ))}
                    </SelectContent>
                  </Select>
                </div>
              </div>

              <div className="flex flex-col gap-1.5">
                <Label>{t("teams.sinners")}</Label>
                <p className="text-xs text-muted-foreground">{t("teams.sinnersHint")}</p>
                <div className="grid grid-cols-3 gap-1.5">
                  {sinners.map((s) => {
                    const active = editing.sinners.includes(s.id);
                    return (
                      <button
                        key={s.id}
                        type="button"
                        onClick={() => toggleSinner(s.id)}
                        className={cn(
                          "rounded-md border px-2 py-1.5 text-xs transition-colors",
                          active
                            ? "border-brand bg-brand-light text-brand font-medium"
                            : "border-border text-muted-foreground hover:text-foreground",
                        )}
                      >
                        {s.name}
                      </button>
                    );
                  })}
                </div>
              </div>

              <div className="flex items-center justify-between">
                <Label>{t("teams.enabled")}</Label>
                <Switch
                  checked={editing.enabled}
                  onCheckedChange={(v) => setEditing({ ...editing, enabled: v })}
                />
              </div>
            </div>
          )}

          <DialogFooter>
            <Button variant="ghost" onClick={() => setEditing(null)}>
              {t("teams.cancel")}
            </Button>
            <Button
              className="bg-brand text-brand-foreground hover:bg-brand-hover"
              disabled={!editing?.name.trim()}
              onClick={() => void save()}
            >
              {t("teams.save")}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </div>
  );
}
