import { ClipboardCopy, ClipboardPaste, Plus, Sparkles, X } from "lucide-react";
import { useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { toast } from "sonner";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { ScrollArea } from "@/components/ui/scroll-area";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { Switch } from "@/components/ui/switch";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { cn } from "@/lib/utils";
import {
  createDefaultMirrorConfig,
  type SinnerInfo,
  type TeamDetail,
  type TeamMirrorConfig,
  type TeamPurpose,
} from "@/services/ipc/types";

export const TEAM_SYSTEMS = [
  { id: "burn", index: 0, labelKey: "teams.schemeBurn", icon: "/status_effects/burn.png" },
  { id: "bleed", index: 1, labelKey: "teams.schemeBleed", icon: "/status_effects/bleed.png" },
  { id: "tremor", index: 2, labelKey: "teams.schemeTremor", icon: "/status_effects/tremor.png" },
  { id: "rupture", index: 3, labelKey: "teams.schemeRupture", icon: "/status_effects/rupture.png" },
  { id: "sinking", index: 4, labelKey: "teams.schemeSinking", icon: "/status_effects/sinking.png" },
  { id: "poise", index: 5, labelKey: "teams.schemePoise", icon: "/status_effects/poise.png" },
  { id: "charge", index: 6, labelKey: "teams.schemeCharge", icon: "/status_effects/charge.png" },
  { id: "slash", index: 7, labelKey: "teams.schemeSlash", icon: "/status_effects/slash.png" },
  { id: "pierce", index: 8, labelKey: "teams.schemePierce", icon: "/status_effects/pierce.png" },
  { id: "blunt", index: 9, labelKey: "teams.schemeBlunt", icon: "/status_effects/blunt.png" },
] as const;

export const STARLIGHT_ITEMS = [
  {
    index: 0,
    nameZh: "初始之星",
    nameEn: "Star of the Beginning",
    cost: 10,
    descZh: "初始经费增加，卡包/饰品展出+1，免费普通刷新",
  },
  {
    index: 1,
    nameZh: "积聚的星云",
    nameEn: "Cumulating Starcloud",
    cost: 10,
    descZh: "进阶经费利息+10%~30%，售卖饰品经费加成",
  },
  {
    index: 2,
    nameZh: "星际漫游",
    nameEn: "Interstellar Travel",
    cost: 20,
    descZh: "卡包出现+1，卡包刷新+2~4，未记录卡包等级提升",
  },
  {
    index: 3,
    nameZh: "流星",
    nameEn: "Star-shower",
    cost: 20,
    descZh: "初始经费+400~700，初始饰品可选择数+1",
  },
  {
    index: 4,
    nameZh: "双星商店",
    nameEn: "Binary Star-shop",
    cost: 30,
    descZh: "展出饰品+1，战斗经费+20%~40%，高阶饰品概率提升",
  },
  {
    index: 5,
    nameZh: "卫星商店",
    nameEn: "Moon Star-shop",
    cost: 30,
    descZh: "免费关键词刷新，进入第1层送1~3件1级饰品",
  },
  {
    index: 6,
    nameZh: "星云的宠爱",
    nameEn: "Favor of the Starcloud",
    cost: 40,
    descZh: "进入第1层人格等级+3，通关阶段人格等级提升",
  },
  {
    index: 7,
    nameZh: "星芒的引导",
    nameEn: "Guidance of the Starlight",
    cost: 40,
    descZh: "最大速度+2~3，拼点威力/伤害强化/守护提升",
  },
  {
    index: 8,
    nameZh: "偶然的彗星",
    nameEn: "Accidental Comet",
    cost: 50,
    descZh: "进商店赠送合成/售卖专用饰品，赠送对应关键词3/4级饰品",
  },
  {
    index: 9,
    nameZh: "全面的可能性",
    nameEn: "All-round Possibility",
    cost: 60,
    descZh: "开局自选3级饰品，获得残影饰品",
  },
] as const;

const PURPOSES: TeamPurpose[] = ["mirror", "luxcavation", "general"];

const purposeKey: Record<TeamPurpose, string> = {
  mirror: "teams.purposeMirror",
  luxcavation: "teams.purposeLuxcavation",
  general: "teams.purposeGeneral",
};

const getSinnerAvatar = (id: string) => `/sinners/${id}.png`;
interface TeamEditModalProps {
  open: boolean;
  team: TeamDetail | null;
  sinners: SinnerInfo[];
  onClose: () => void;
  onSave: (team: TeamDetail) => void;
}

export function TeamEditModal({ open, team, sinners, onClose, onSave }: TeamEditModalProps) {
  const { t, i18n } = useTranslation();
  const isEn = i18n.language.startsWith("en");

  const [form, setForm] = useState<TeamDetail>(() => ({
    id: team?.id || "",
    name: team?.name || "",
    purpose: team?.purpose || "general",
    sinners: team?.sinners || [],
    accessoryScheme: team?.accessoryScheme || "burn",
    enabled: team?.enabled ?? true,
    mirrorConfig: team?.mirrorConfig || createDefaultMirrorConfig(),
  }));

  const [activeTab, setActiveTab] = useState("basic");
  const [observeInput, setObserveInput] = useState("");
  const [jsonInput, setJsonInput] = useState("");
  const [jsonImportOpen, setJsonImportOpen] = useState(false);

  // Sync state when incoming team changes
  const initialTeamId = team?.id;
  useMemo(() => {
    if (team) {
      setForm({
        ...team,
        mirrorConfig: team.mirrorConfig ? { ...team.mirrorConfig } : createDefaultMirrorConfig(),
      });
    }
  }, [team, initialTeamId]);

  const mirrorConfig = form.mirrorConfig || createDefaultMirrorConfig();

  const updateMirror = (patch: Partial<TeamMirrorConfig>) => {
    setForm((prev) => ({
      ...prev,
      mirrorConfig: {
        ...(prev.mirrorConfig || createDefaultMirrorConfig()),
        ...patch,
      },
    }));
  };

  const updateDiscardSystem = (key: keyof TeamMirrorConfig["discard_systems"], val: boolean) => {
    updateMirror({
      discard_systems: {
        ...mirrorConfig.discard_systems,
        [key]: val,
      },
    });
  };

  const toggleSinner = (id: string) => {
    const isSelected = form.sinners.includes(id);
    if (isSelected) {
      setForm((prev) => ({
        ...prev,
        sinners: prev.sinners.filter((s) => s !== id),
      }));
    } else {
      if (form.sinners.length >= 12) return;
      setForm((prev) => ({
        ...prev,
        sinners: [...prev.sinners, id],
      }));
    }
  };

  const clearSinners = () => {
    setForm((prev) => ({ ...prev, sinners: [] }));
  };

  const calculateStarlightCost = useMemo(() => {
    const bonus = mirrorConfig.opening_bonus || [];
    return STARLIGHT_ITEMS.reduce((acc, item, i) => {
      const lvl = bonus[i] || 0;
      return acc + item.cost * lvl;
    }, 0);
  }, [mirrorConfig.opening_bonus]);

  const setAllStarlight = (level: number) => {
    updateMirror({
      opening_bonus: Array(10).fill(level),
    });
  };

  const handleAddObserveGift = () => {
    const trimmed = observeInput.trim();
    if (!trimmed) return;
    if (mirrorConfig.observe_ego_gift_selected.includes(trimmed)) return;
    updateMirror({
      observe_ego_gift_selected: [...mirrorConfig.observe_ego_gift_selected, trimmed],
    });
    setObserveInput("");
  };

  const handleRemoveObserveGift = (tag: string) => {
    updateMirror({
      observe_ego_gift_selected: mirrorConfig.observe_ego_gift_selected.filter((s) => s !== tag),
    });
  };

  const handleCopyJson = async () => {
    try {
      await navigator.clipboard.writeText(JSON.stringify(form, null, 2));
      toast.success(t("teams.modal.exportSuccess"));
    } catch {
      toast.error(t("teams.modal.importError"));
    }
  };

  const handlePasteJson = () => {
    try {
      const parsed = JSON.parse(jsonInput.trim());
      if (parsed && typeof parsed === "object" && parsed.name) {
        setForm({
          id: form.id || parsed.id || "",
          name: parsed.name || form.name,
          purpose: parsed.purpose || form.purpose,
          sinners: Array.isArray(parsed.sinners) ? parsed.sinners : form.sinners,
          accessoryScheme: parsed.accessoryScheme || form.accessoryScheme,
          enabled: parsed.enabled ?? form.enabled,
          mirrorConfig: parsed.mirrorConfig
            ? { ...createDefaultMirrorConfig(), ...parsed.mirrorConfig }
            : form.mirrorConfig,
        });
        setJsonImportOpen(false);
        setJsonInput("");
        toast.success(t("teams.modal.importSuccess"));
      } else {
        toast.error(t("teams.modal.importError"));
      }
    } catch {
      toast.error(t("teams.modal.importError"));
    }
  };

  const handleSave = () => {
    if (!form.name.trim()) return;
    onSave(form);
  };

  return (
    <Dialog open={open} onOpenChange={(v) => !v && onClose()}>
      <DialogContent className="max-w-3xl h-[620px] max-h-[88vh] flex flex-col p-0 gap-0 overflow-hidden">
        {/* Header */}
        <DialogHeader className="px-6 pt-5 pb-3 border-b border-border/60 shrink-0">
          <div className="flex items-center justify-between">
            <DialogTitle className="text-base font-semibold">
              {team?.id ? t("teams.editTeam") : t("teams.newTeam")}
            </DialogTitle>
            <div className="flex items-center gap-2">
              <Button
                variant="outline"
                size="sm"
                className="h-7 text-xs gap-1"
                onClick={handleCopyJson}
                title={t("teams.modal.exportJson")}
              >
                <ClipboardCopy className="size-3.5" />
                <span>{t("teams.modal.exportJson")}</span>
              </Button>
            </div>
          </div>
        </DialogHeader>

        {/* Tab Selection */}
        <Tabs
          value={activeTab}
          onValueChange={setActiveTab}
          className="flex-1 flex flex-col min-h-0"
        >
          <div className="px-6 py-2 border-b border-border/40 bg-card/20 shrink-0">
            <TabsList className="h-8 p-0.5 bg-muted/60">
              <TabsTrigger value="basic" className="h-7 px-3 text-xs">
                {t("teams.tabs.basic")}
              </TabsTrigger>
              <TabsTrigger value="shop" className="h-7 px-3 text-xs">
                {t("teams.tabs.shop")}
              </TabsTrigger>
              <TabsTrigger value="combat" className="h-7 px-3 text-xs">
                {t("teams.tabs.combat")}
              </TabsTrigger>
              <TabsTrigger value="starlight" className="h-7 px-3 text-xs">
                {t("teams.tabs.starlight")}
                {calculateStarlightCost > 0 && (
                  <Badge
                    variant="secondary"
                    className="ml-1.5 h-4 px-1 text-[10px] font-mono text-amber-500"
                  >
                    {calculateStarlightCost}
                  </Badge>
                )}
              </TabsTrigger>
              <TabsTrigger value="advanced" className="h-7 px-3 text-xs">
                {t("teams.tabs.advanced")}
              </TabsTrigger>
            </TabsList>
          </div>

          {/* Tab Contents Scroll Area */}
          <ScrollArea className="flex-1 min-h-0">
            <div className="px-6 py-4 space-y-4">
              {/* 1. Basic & Formation */}
              <TabsContent value="basic" className="m-0 space-y-4">
                <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
                  <div className="space-y-1.5">
                    <Label className="text-xs font-medium">{t("teams.name")}</Label>
                    <Input
                      value={form.name}
                      placeholder={t("teams.namePlaceholder")}
                      onChange={(e) => setForm({ ...form, name: e.target.value })}
                      className="h-8 text-xs"
                    />
                  </div>

                  <div className="space-y-1.5">
                    <Label className="text-xs font-medium">{t("teams.purpose")}</Label>
                    <Select
                      value={form.purpose}
                      onValueChange={(v) => setForm({ ...form, purpose: v as TeamPurpose })}
                    >
                      <SelectTrigger className="h-8 text-xs">
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
                </div>

                <div className="space-y-1.5">
                  <Label className="text-xs font-medium">{t("teams.scheme")}</Label>
                  <div className="grid grid-cols-5 gap-2">
                    {TEAM_SYSTEMS.map((sys) => {
                      const isSelected = form.accessoryScheme === sys.id;
                      return (
                        <button
                          key={sys.id}
                          type="button"
                          onClick={() => {
                            setForm({ ...form, accessoryScheme: sys.id });
                            updateMirror({ team_system: sys.index });
                          }}
                          className={cn(
                            "flex items-center gap-2 p-2 rounded-lg border text-xs font-medium transition-all text-left",
                            isSelected
                              ? "border-brand bg-brand/10 text-brand font-semibold shadow-xs"
                              : "border-border/60 hover:bg-muted/50 text-muted-foreground hover:text-foreground",
                          )}
                        >
                          <img
                            src={sys.icon}
                            alt={sys.id}
                            className="size-5 shrink-0 object-contain"
                          />
                          <span className="truncate">{t(sys.labelKey)}</span>
                        </button>
                      );
                    })}
                  </div>
                </div>

                {/* Sinner Deployment Order */}
                <div className="space-y-2 rounded-lg border border-border/60 bg-card/20 p-3.5">
                  <div className="flex items-center justify-between">
                    <div>
                      <Label className="text-xs font-medium">{t("teams.sinners")}</Label>
                      <p className="text-[11px] text-muted-foreground">{t("teams.sinnersHint")}</p>
                    </div>
                    <div className="flex items-center gap-2">
                      <Badge variant="outline" className="text-[11px] font-mono">
                        {t("teams.sinnersCount", { count: form.sinners.length })}
                      </Badge>
                      {form.sinners.length > 0 && (
                        <Button
                          type="button"
                          variant="ghost"
                          size="sm"
                          className="h-6 text-[11px] text-destructive hover:text-destructive px-2"
                          onClick={clearSinners}
                        >
                          {t("teams.clearSinners")}
                        </Button>
                      )}
                    </div>
                  </div>

                  <div className="grid grid-cols-3 sm:grid-cols-4 md:grid-cols-6 gap-2 pt-1">
                    {sinners.map((s) => {
                      const orderIndex = form.sinners.indexOf(s.id);
                      const isSelected = orderIndex !== -1;
                      return (
                        <button
                          key={s.id}
                          type="button"
                          onClick={() => toggleSinner(s.id)}
                          className={cn(
                            "relative flex flex-col items-center justify-center p-2 rounded-lg border text-xs transition-all select-none group gap-1.5",
                            isSelected
                              ? "border-brand bg-brand/10 text-foreground font-semibold shadow-xs ring-1 ring-brand/30"
                              : "border-border/60 hover:bg-muted/40 text-muted-foreground hover:text-foreground",
                          )}
                        >
                          {isSelected && (
                            <span className="absolute top-1 right-1 size-4.5 rounded-full bg-brand text-brand-foreground text-[10px] font-mono font-bold flex items-center justify-center">
                              #{orderIndex + 1}
                            </span>
                          )}
                          <img
                            src={getSinnerAvatar(s.id)}
                            alt={s.name}
                            className="size-12 rounded-md object-cover border border-border/20 bg-muted/20"
                            onError={(e) => {
                              (e.currentTarget as HTMLImageElement).style.display = "none";
                            }}
                          />
                          <span className="truncate w-full text-center leading-none text-[11px]">
                            {s.name}
                          </span>
                        </button>
                      );
                    })}
                  </div>
                </div>

                {/* Team Code & Fixed Use */}
                <div className="grid grid-cols-1 md:grid-cols-2 gap-3 pt-1">
                  <div className="p-3 rounded-lg border border-border/50 bg-card/20 space-y-2">
                    <div className="flex items-center justify-between">
                      <Label className="text-xs font-medium">{t("teams.modal.useTeamCode")}</Label>
                      <Switch
                        checked={mirrorConfig.use_team_code}
                        onCheckedChange={(v) => updateMirror({ use_team_code: v })}
                      />
                    </div>
                    <p className="text-[11px] text-muted-foreground">
                      {t("teams.modal.useTeamCodeDesc")}
                    </p>
                    {mirrorConfig.use_team_code && (
                      <Input
                        value={mirrorConfig.team_code}
                        placeholder={t("teams.modal.teamCodePlaceholder")}
                        onChange={(e) => updateMirror({ team_code: e.target.value })}
                        className="h-8 text-xs font-mono"
                      />
                    )}
                  </div>

                  <div className="p-3 rounded-lg border border-border/50 bg-card/20 space-y-2">
                    <div className="flex items-center justify-between">
                      <Label className="text-xs font-medium">{t("teams.modal.fixedUse")}</Label>
                      <Switch
                        checked={mirrorConfig.fixed_team_use}
                        onCheckedChange={(v) => updateMirror({ fixed_team_use: v })}
                      />
                    </div>
                    <p className="text-[11px] text-muted-foreground">
                      {t("teams.modal.fixedUseDesc")}
                    </p>
                    {mirrorConfig.fixed_team_use && (
                      <Select
                        value={String(mirrorConfig.fixed_team_use_select)}
                        onValueChange={(v) => updateMirror({ fixed_team_use_select: Number(v) })}
                      >
                        <SelectTrigger className="h-8 text-xs">
                          <SelectValue />
                        </SelectTrigger>
                        <SelectContent>
                          <SelectItem value="0">{t("teams.modal.fixedUseHard")}</SelectItem>
                          <SelectItem value="1">{t("teams.modal.fixedUseNormal")}</SelectItem>
                          <SelectItem value="2">{t("teams.modal.fixedUseAll")}</SelectItem>
                        </SelectContent>
                      </Select>
                    )}
                  </div>
                </div>

                <div className="flex items-center justify-between pt-1">
                  <Label className="text-xs font-medium">{t("teams.enabled")}</Label>
                  <Switch
                    checked={form.enabled}
                    onCheckedChange={(v) => setForm({ ...form, enabled: v })}
                  />
                </div>
              </TabsContent>

              {/* 2. Shop & Fusion */}
              <TabsContent value="shop" className="m-0 space-y-4">
                <div className="space-y-1.5">
                  <Label className="text-xs font-medium">{t("teams.modal.shopStrategy")}</Label>
                  <Select
                    value={String(mirrorConfig.shop_strategy)}
                    onValueChange={(v) => updateMirror({ shop_strategy: Number(v) })}
                  >
                    <SelectTrigger className="h-8 text-xs">
                      <SelectValue />
                    </SelectTrigger>
                    <SelectContent>
                      <SelectItem value="0">{t("teams.modal.shopStrategyDefault")}</SelectItem>
                      <SelectItem value="1">{t("teams.modal.shopStrategyConservative")}</SelectItem>
                      <SelectItem value="2">{t("teams.modal.shopStrategyAggressive")}</SelectItem>
                    </SelectContent>
                  </Select>
                </div>

                {/* Discarded Systems (舍弃体系) */}
                <div className="space-y-2 rounded-lg border border-border/60 bg-card/20 p-3.5">
                  <div>
                    <Label className="text-xs font-medium">{t("teams.modal.discardSystems")}</Label>
                    <p className="text-[11px] text-muted-foreground">
                      {t("teams.modal.discardSystemsDesc")}
                    </p>
                  </div>
                  <div className="grid grid-cols-5 gap-2 pt-1">
                    {TEAM_SYSTEMS.map((sys) => {
                      const sysKey = sys.id as keyof TeamMirrorConfig["discard_systems"];
                      const isDiscarded = mirrorConfig.discard_systems[sysKey];
                      return (
                        <button
                          key={sys.id}
                          type="button"
                          onClick={() => updateDiscardSystem(sysKey, !isDiscarded)}
                          className={cn(
                            "flex items-center gap-1.5 p-2 rounded-lg border text-xs font-medium transition-all select-none",
                            isDiscarded
                              ? "border-destructive/60 bg-destructive/10 text-destructive font-semibold"
                              : "border-border/60 hover:bg-muted/40 text-muted-foreground hover:text-foreground",
                          )}
                        >
                          <img
                            src={sys.icon}
                            alt={sys.id}
                            className="size-4 shrink-0 object-contain"
                          />
                          <span className="truncate">{t(sys.labelKey)}</span>
                          {isDiscarded && <span className="ml-auto text-[10px]">✕</span>}
                        </button>
                      );
                    })}
                  </div>
                </div>

                {/* Basic Shop Action Restrictions */}
                <div className="space-y-2 rounded-lg border border-border/60 bg-card/20 p-3.5">
                  <Label className="text-xs font-medium">{t("teams.modal.shopRestrictions")}</Label>
                  <div className="grid grid-cols-2 sm:grid-cols-3 gap-3 pt-1">
                    <label className="flex items-center gap-2 text-xs text-foreground cursor-pointer">
                      <Switch
                        checked={mirrorConfig.do_not_heal}
                        onCheckedChange={(v) => updateMirror({ do_not_heal: v })}
                      />
                      <span>{t("teams.modal.doNotHeal")}</span>
                    </label>
                    <label className="flex items-center gap-2 text-xs text-foreground cursor-pointer">
                      <Switch
                        checked={mirrorConfig.do_not_buy}
                        onCheckedChange={(v) => updateMirror({ do_not_buy: v })}
                      />
                      <span>{t("teams.modal.doNotBuy")}</span>
                    </label>
                    <label className="flex items-center gap-2 text-xs text-foreground cursor-pointer">
                      <Switch
                        checked={mirrorConfig.do_not_fuse}
                        onCheckedChange={(v) => updateMirror({ do_not_fuse: v })}
                      />
                      <span>{t("teams.modal.doNotFuse")}</span>
                    </label>
                    <label className="flex items-center gap-2 text-xs text-foreground cursor-pointer">
                      <Switch
                        checked={mirrorConfig.do_not_sell}
                        onCheckedChange={(v) => updateMirror({ do_not_sell: v })}
                      />
                      <span>{t("teams.modal.doNotSell")}</span>
                    </label>
                    <label className="flex items-center gap-2 text-xs text-foreground cursor-pointer">
                      <Switch
                        checked={mirrorConfig.do_not_enhance}
                        onCheckedChange={(v) => updateMirror({ do_not_enhance: v })}
                      />
                      <span>{t("teams.modal.doNotEnhance")}</span>
                    </label>
                  </div>
                </div>

                {/* Advanced Fusion Strategy */}
                <div className="space-y-3 rounded-lg border border-border/60 bg-card/20 p-3.5">
                  <Label className="text-xs font-medium">{t("teams.modal.fusionSettings")}</Label>
                  <div className="grid grid-cols-1 sm:grid-cols-2 gap-3">
                    <label className="flex items-center gap-2 text-xs text-foreground cursor-pointer">
                      <Switch
                        checked={mirrorConfig.only_aggressive_fuse}
                        onCheckedChange={(v) => updateMirror({ only_aggressive_fuse: v })}
                      />
                      <span>{t("teams.modal.onlyAggressiveFuse")}</span>
                    </label>
                    <label className="flex items-center gap-2 text-xs text-foreground cursor-pointer">
                      <Switch
                        checked={mirrorConfig.do_not_system_fuse}
                        onCheckedChange={(v) => updateMirror({ do_not_system_fuse: v })}
                      />
                      <span>{t("teams.modal.doNotSystemFuse")}</span>
                    </label>
                    <label className="flex items-center gap-2 text-xs text-foreground cursor-pointer">
                      <Switch
                        checked={mirrorConfig.only_system_fuse}
                        onCheckedChange={(v) => updateMirror({ only_system_fuse: v })}
                      />
                      <span>{t("teams.modal.onlySystemFuse")}</span>
                    </label>
                    <label className="flex items-center gap-2 text-xs text-foreground cursor-pointer">
                      <Switch
                        checked={mirrorConfig.aggressive_also_enhance}
                        onCheckedChange={(v) => updateMirror({ aggressive_also_enhance: v })}
                      />
                      <span>{t("teams.modal.aggressiveAlsoEnhance")}</span>
                    </label>
                    <label className="flex items-center gap-2 text-xs text-foreground cursor-pointer">
                      <Switch
                        checked={mirrorConfig.aggressive_save_systems}
                        onCheckedChange={(v) => updateMirror({ aggressive_save_systems: v })}
                      />
                      <span>{t("teams.modal.aggressiveSaveSystems")}</span>
                    </label>
                  </div>

                  <div className="pt-1 flex items-center justify-between gap-3">
                    <div className="flex items-center gap-2">
                      <Switch
                        checked={mirrorConfig.after_level_IV}
                        onCheckedChange={(v) => updateMirror({ after_level_IV: v })}
                      />
                      <Label className="text-xs font-normal">{t("teams.modal.afterLevelIV")}</Label>
                    </div>
                    {mirrorConfig.after_level_IV && (
                      <Select
                        value={String(mirrorConfig.after_level_IV_select)}
                        onValueChange={(v) => updateMirror({ after_level_IV_select: Number(v) })}
                      >
                        <SelectTrigger className="h-7 w-40 text-xs">
                          <SelectValue />
                        </SelectTrigger>
                        <SelectContent>
                          <SelectItem value="0">{t("teams.modal.afterLevelIVStop")}</SelectItem>
                          <SelectItem value="1">{t("teams.modal.afterLevelIVContinue")}</SelectItem>
                          <SelectItem value="2">{t("teams.modal.afterLevelIVEnhance")}</SelectItem>
                        </SelectContent>
                      </Select>
                    )}
                  </div>
                </div>

                {/* Shop Refresh & Floor Ignore */}
                <div className="grid grid-cols-1 sm:grid-cols-2 gap-3">
                  <div className="p-3 rounded-lg border border-border/50 bg-card/20 space-y-2">
                    <Label className="text-xs font-medium">{t("teams.modal.shopRefresh")}</Label>
                    <div className="flex items-center justify-between text-xs">
                      <span>{t("teams.modal.maxKeywordRefresh")}</span>
                      <Input
                        type="number"
                        min={0}
                        max={10}
                        value={mirrorConfig.max_keyword_refresh}
                        onChange={(e) =>
                          updateMirror({ max_keyword_refresh: Number(e.target.value) || 0 })
                        }
                        className="h-7 w-20 text-xs"
                      />
                    </div>
                    <div className="flex items-center justify-between text-xs">
                      <span>{t("teams.modal.maxNormalRefresh")}</span>
                      <Input
                        type="number"
                        min={0}
                        max={10}
                        value={mirrorConfig.max_normal_refresh}
                        onChange={(e) =>
                          updateMirror({ max_normal_refresh: Number(e.target.value) || 0 })
                        }
                        className="h-7 w-20 text-xs"
                      />
                    </div>
                  </div>

                  <div className="p-3 rounded-lg border border-border/50 bg-card/20 space-y-2">
                    <Label className="text-xs font-medium">
                      {t("teams.modal.ignoreShopFloors")}
                    </Label>
                    <div className="grid grid-cols-5 gap-1.5 pt-1">
                      {[1, 2, 3, 4, 5].map((fl, idx) => {
                        const ignored = mirrorConfig.ignore_shop[idx] || false;
                        return (
                          <button
                            key={fl}
                            type="button"
                            onClick={() => {
                              const copy = [...mirrorConfig.ignore_shop];
                              copy[idx] = !ignored;
                              updateMirror({ ignore_shop: copy });
                            }}
                            className={cn(
                              "py-1.5 rounded border text-xs font-mono transition-all",
                              ignored
                                ? "border-destructive bg-destructive/15 text-destructive font-bold"
                                : "border-border/60 hover:bg-muted/50 text-muted-foreground",
                            )}
                          >
                            {fl}F
                          </button>
                        );
                      })}
                    </div>
                  </div>
                </div>
              </TabsContent>

              {/* 3. Second System & Combat */}
              <TabsContent value="combat" className="m-0 space-y-4">
                {/* Second System */}
                <div className="space-y-3 rounded-lg border border-border/60 bg-card/20 p-3.5">
                  <div className="flex items-center justify-between">
                    <div>
                      <Label className="text-xs font-medium">{t("teams.modal.secondSystem")}</Label>
                      <p className="text-[11px] text-muted-foreground">
                        {t("teams.modal.secondSystemDesc")}
                      </p>
                    </div>
                    <Switch
                      checked={mirrorConfig.second_system}
                      onCheckedChange={(v) => updateMirror({ second_system: v })}
                    />
                  </div>

                  {mirrorConfig.second_system && (
                    <div className="space-y-3 pt-2 border-t border-border/40">
                      <div className="grid grid-cols-2 gap-3">
                        <div className="space-y-1">
                          <Label className="text-[11px] text-muted-foreground">
                            {t("teams.modal.secondSystemSelect")}
                          </Label>
                          <Select
                            value={String(mirrorConfig.second_system_select)}
                            onValueChange={(v) => updateMirror({ second_system_select: Number(v) })}
                          >
                            <SelectTrigger className="h-8 text-xs">
                              <SelectValue />
                            </SelectTrigger>
                            <SelectContent>
                              {TEAM_SYSTEMS.map((s) => (
                                <SelectItem key={s.id} value={String(s.index)}>
                                  {t(s.labelKey)}
                                </SelectItem>
                              ))}
                            </SelectContent>
                          </Select>
                        </div>

                        <div className="space-y-1">
                          <Label className="text-[11px] text-muted-foreground">
                            {t("teams.modal.secondSystemStartFloor")}
                          </Label>
                          <Select
                            value={String(mirrorConfig.second_system_setting)}
                            onValueChange={(v) =>
                              updateMirror({ second_system_setting: Number(v) })
                            }
                          >
                            <SelectTrigger className="h-8 text-xs">
                              <SelectValue />
                            </SelectTrigger>
                            <SelectContent>
                              {[2, 3, 4, 5].map((fl) => (
                                <SelectItem key={fl} value={String(fl)}>
                                  {t("teams.modal.floorN", { n: fl })}
                                </SelectItem>
                              ))}
                            </SelectContent>
                          </Select>
                        </div>
                      </div>

                      <div className="space-y-1.5">
                        <Label className="text-[11px] text-muted-foreground">
                          {t("teams.modal.secondSystemActions")}
                        </Label>
                        <div className="grid grid-cols-2 sm:grid-cols-4 gap-2">
                          <label className="flex items-center gap-1.5 text-xs text-foreground cursor-pointer">
                            <Switch
                              checked={mirrorConfig.second_system_fuse_IV}
                              onCheckedChange={(v) => updateMirror({ second_system_fuse_IV: v })}
                            />
                            <span>{t("teams.modal.secondSystemFuseIV")}</span>
                          </label>
                          <label className="flex items-center gap-1.5 text-xs text-foreground cursor-pointer">
                            <Switch
                              checked={mirrorConfig.second_system_buy}
                              onCheckedChange={(v) => updateMirror({ second_system_buy: v })}
                            />
                            <span>{t("teams.modal.secondSystemBuy")}</span>
                          </label>
                          <label className="flex items-center gap-1.5 text-xs text-foreground cursor-pointer">
                            <Switch
                              checked={mirrorConfig.second_system_select_reward}
                              onCheckedChange={(v) =>
                                updateMirror({ second_system_select_reward: v })
                              }
                            />
                            <span>{t("teams.modal.secondSystemSelectReward")}</span>
                          </label>
                          <label className="flex items-center gap-1.5 text-xs text-foreground cursor-pointer">
                            <Switch
                              checked={mirrorConfig.second_system_power_up}
                              onCheckedChange={(v) => updateMirror({ second_system_power_up: v })}
                            />
                            <span>{t("teams.modal.secondSystemPowerUp")}</span>
                          </label>
                        </div>
                      </div>
                    </div>
                  )}
                </div>

                {/* Combat Preferences & Skills */}
                <div className="space-y-3 rounded-lg border border-border/60 bg-card/20 p-3.5">
                  <Label className="text-xs font-medium">
                    {t("teams.modal.combatPreferences")}
                  </Label>
                  <div className="grid grid-cols-1 sm:grid-cols-2 gap-3">
                    <label className="flex items-center gap-2 text-xs text-foreground cursor-pointer">
                      <Switch
                        checked={mirrorConfig.avoid_skill_3}
                        onCheckedChange={(v) => {
                          updateMirror({
                            avoid_skill_3: v,
                            prioritize_skill_3: v ? false : mirrorConfig.prioritize_skill_3,
                          });
                        }}
                      />
                      <span>{t("teams.modal.avoidSkill3")}</span>
                    </label>
                    <label className="flex items-center gap-2 text-xs text-foreground cursor-pointer">
                      <Switch
                        checked={mirrorConfig.prioritize_skill_3}
                        onCheckedChange={(v) => {
                          updateMirror({
                            prioritize_skill_3: v,
                            avoid_skill_3: v ? false : mirrorConfig.avoid_skill_3,
                          });
                        }}
                      />
                      <span>{t("teams.modal.prioritizeSkill3")}</span>
                    </label>
                    <label className="flex items-center gap-2 text-xs text-foreground cursor-pointer">
                      <Switch
                        checked={mirrorConfig.re_formation_each_floor}
                        onCheckedChange={(v) => updateMirror({ re_formation_each_floor: v })}
                      />
                      <span>{t("teams.modal.reFormationEachFloor")}</span>
                    </label>
                  </div>
                </div>

                {/* Defense & Sacrificial Solo */}
                <div className="space-y-3 rounded-lg border border-border/60 bg-card/20 p-3.5">
                  <Label className="text-xs font-medium">
                    {t("teams.modal.defenseStrategies")}
                  </Label>
                  <div className="space-y-2.5">
                    <label className="flex items-center gap-2 text-xs text-foreground cursor-pointer">
                      <Switch
                        checked={mirrorConfig.defense_first_round}
                        onCheckedChange={(v) => {
                          updateMirror({
                            defense_first_round: v,
                            defense_for_solo: v ? false : mirrorConfig.defense_for_solo,
                          });
                        }}
                      />
                      <span>{t("teams.modal.defenseFirstRound")}</span>
                    </label>

                    <div className="space-y-1.5 pt-1">
                      <div className="flex items-center justify-between">
                        <label className="flex items-center gap-2 text-xs text-foreground cursor-pointer">
                          <Switch
                            checked={mirrorConfig.defense_for_solo}
                            onCheckedChange={(v) => {
                              updateMirror({
                                defense_for_solo: v,
                                defense_first_round: v ? false : mirrorConfig.defense_first_round,
                              });
                            }}
                          />
                          <span>{t("teams.modal.defenseForSolo")}</span>
                        </label>
                        {mirrorConfig.defense_for_solo && (
                          <Select
                            value={String(mirrorConfig.defense_for_solo_turns)}
                            onValueChange={(v) =>
                              updateMirror({ defense_for_solo_turns: Number(v) })
                            }
                          >
                            <SelectTrigger className="h-7 w-28 text-xs">
                              <SelectValue />
                            </SelectTrigger>
                            <SelectContent>
                              {[1, 2, 3, 4, 5].map((tNum) => (
                                <SelectItem key={tNum} value={String(tNum)}>
                                  {t("teams.modal.defenseTurns", { n: tNum })}
                                </SelectItem>
                              ))}
                            </SelectContent>
                          </Select>
                        )}
                      </div>
                      <p className="text-[11px] text-muted-foreground ml-8">
                        {t("teams.modal.defenseForSoloDesc")}
                      </p>
                    </div>
                  </div>
                </div>

                {/* Skill Replacement */}
                <div className="p-3.5 rounded-lg border border-border/60 bg-card/20 space-y-2">
                  <div className="flex items-center justify-between">
                    <Label className="text-xs font-medium">
                      {t("teams.modal.skillReplacement")}
                    </Label>
                    <Switch
                      checked={mirrorConfig.skill_replacement}
                      onCheckedChange={(v) => updateMirror({ skill_replacement: v })}
                    />
                  </div>
                  {mirrorConfig.skill_replacement && (
                    <div className="pt-2 grid grid-cols-2 gap-3">
                      <div className="space-y-1">
                        <Label className="text-[11px] text-muted-foreground">
                          {t("teams.modal.skillReplacementMode")}
                        </Label>
                        <Select
                          value={String(mirrorConfig.skill_replacement_mode)}
                          onValueChange={(v) => updateMirror({ skill_replacement_mode: Number(v) })}
                        >
                          <SelectTrigger className="h-8 text-xs">
                            <SelectValue />
                          </SelectTrigger>
                          <SelectContent>
                            <SelectItem value="0">{t("teams.modal.replace1to2")}</SelectItem>
                            <SelectItem value="1">{t("teams.modal.replace1to3")}</SelectItem>
                          </SelectContent>
                        </Select>
                      </div>
                    </div>
                  )}
                </div>
              </TabsContent>

              {/* 4. Starlight Bonus */}
              <TabsContent value="starlight" className="m-0 space-y-4">
                <div className="flex items-center justify-between p-3.5 rounded-lg border border-border/60 bg-card/20">
                  <div>
                    <Label className="text-xs font-medium">{t("teams.modal.useStarlight")}</Label>
                    <p className="text-[11px] text-muted-foreground">
                      {t("teams.modal.useStarlightDesc")}
                    </p>
                  </div>
                  <Switch
                    checked={mirrorConfig.use_starlight}
                    onCheckedChange={(v) => updateMirror({ use_starlight: v })}
                  />
                </div>

                {/* Quick Batch Level & Total Calculation */}
                <div className="flex flex-wrap items-center justify-between gap-2 p-3 rounded-lg border border-border/50 bg-muted/30">
                  <div className="flex items-center gap-1.5 text-xs">
                    <span className="text-muted-foreground">{t("teams.modal.setAllLevel")}</span>
                    {[0, 1, 2, 3].map((lvl) => (
                      <Button
                        key={lvl}
                        type="button"
                        variant="outline"
                        size="sm"
                        className="h-6 px-2 text-[11px]"
                        onClick={() => setAllStarlight(lvl)}
                      >
                        {lvl === 0
                          ? t("teams.modal.levelOff")
                          : lvl === 1
                            ? t("teams.modal.levelBase")
                            : lvl === 2
                              ? t("teams.modal.levelPlus")
                              : t("teams.modal.levelPlusPlus")}
                      </Button>
                    ))}
                  </div>

                  <div className="flex items-center gap-2">
                    <Badge
                      variant="outline"
                      className="text-xs font-mono font-semibold bg-amber-500/10 text-amber-500 border-amber-500/30"
                    >
                      <Sparkles className="size-3 mr-1" />
                      {t("teams.modal.starlightTotalCost", { cost: calculateStarlightCost })}
                    </Badge>
                  </div>
                </div>

                {/* 10 Starlight Bonus Items */}
                <div className="grid grid-cols-1 md:grid-cols-2 gap-2.5">
                  {STARLIGHT_ITEMS.map((item) => {
                    const currentLevel = mirrorConfig.opening_bonus[item.index] || 0;
                    return (
                      <div
                        key={item.index}
                        className={cn(
                          "p-2.5 rounded-lg border transition-all space-y-1.5",
                          currentLevel > 0
                            ? "border-amber-500/40 bg-amber-500/5 shadow-xs"
                            : "border-border/60 bg-card/20",
                        )}
                      >
                        <div className="flex items-center justify-between">
                          <div className="flex items-center gap-1.5">
                            <span className="text-xs font-semibold">
                              {isEn ? item.nameEn : item.nameZh}
                            </span>
                            <span className="text-[10px] font-mono text-muted-foreground">
                              ({item.cost} ★)
                            </span>
                          </div>
                          {/* 4-level segmented switch */}
                          <div className="flex items-center rounded border border-border/60 p-0.5 bg-background text-[10px] font-mono">
                            {[0, 1, 2, 3].map((lvl) => (
                              <button
                                key={lvl}
                                type="button"
                                onClick={() => {
                                  const copy = [...mirrorConfig.opening_bonus];
                                  copy[item.index] = lvl;
                                  updateMirror({ opening_bonus: copy });
                                }}
                                className={cn(
                                  "px-1.5 py-0.5 rounded transition-all",
                                  currentLevel === lvl
                                    ? "bg-amber-500 text-white font-bold"
                                    : "text-muted-foreground hover:text-foreground",
                                )}
                              >
                                {lvl === 0 ? "0" : lvl === 1 ? "1" : lvl === 2 ? "2+" : "3++"}
                              </button>
                            ))}
                          </div>
                        </div>
                        <p className="text-[10px] text-muted-foreground line-clamp-2 leading-tight">
                          {item.descZh}
                        </p>
                      </div>
                    );
                  })}
                </div>
              </TabsContent>

              {/* 5. Observe & Advanced */}
              <TabsContent value="advanced" className="m-0 space-y-4">
                {/* Observe EGO Gifts */}
                <div className="space-y-3 rounded-lg border border-border/60 bg-card/20 p-3.5">
                  <div className="flex items-center justify-between">
                    <div>
                      <Label className="text-xs font-medium">
                        {t("teams.modal.observeEgoGift")}
                      </Label>
                      <p className="text-[11px] text-muted-foreground">
                        {t("teams.modal.observeEgoGiftDesc")}
                      </p>
                    </div>
                    <Switch
                      checked={mirrorConfig.observe_ego_gift}
                      onCheckedChange={(v) => updateMirror({ observe_ego_gift: v })}
                    />
                  </div>

                  {mirrorConfig.observe_ego_gift && (
                    <div className="space-y-2 pt-2 border-t border-border/40">
                      <div className="flex gap-2">
                        <Input
                          value={observeInput}
                          placeholder={t("teams.modal.observePlaceholder")}
                          onChange={(e) => setObserveInput(e.target.value)}
                          onKeyDown={(e) =>
                            e.key === "Enter" && (e.preventDefault(), handleAddObserveGift())
                          }
                          className="h-8 text-xs flex-1"
                        />
                        <Button
                          type="button"
                          size="sm"
                          className="h-8 px-3 text-xs bg-brand text-brand-foreground"
                          onClick={handleAddObserveGift}
                        >
                          <Plus className="size-3.5 mr-1" />
                          <span>添加</span>
                        </Button>
                      </div>

                      <div className="flex flex-wrap gap-1.5 pt-1">
                        {mirrorConfig.observe_ego_gift_selected.map((gift) => (
                          <Badge
                            key={gift}
                            variant="secondary"
                            className="h-6 gap-1 px-2 text-xs font-normal"
                          >
                            <span>{gift}</span>
                            <button
                              type="button"
                              onClick={() => handleRemoveObserveGift(gift)}
                              className="text-muted-foreground hover:text-destructive"
                            >
                              <X className="size-3" />
                            </button>
                          </Badge>
                        ))}
                      </div>
                    </div>
                  )}
                </div>

                {/* Theme Pack Custom Weight */}
                <div className="p-3.5 rounded-lg border border-border/60 bg-card/20 flex items-center justify-between">
                  <div>
                    <Label className="text-xs font-medium">
                      {t("teams.modal.themePackWeight")}
                    </Label>
                    <p className="text-[11px] text-muted-foreground">
                      {t("teams.modal.themePackWeightDesc")}
                    </p>
                  </div>
                  <Switch
                    checked={mirrorConfig.use_custom_theme_pack_weight}
                    onCheckedChange={(v) => updateMirror({ use_custom_theme_pack_weight: v })}
                  />
                </div>

                {/* Import / Paste JSON Panel */}
                <div className="p-3.5 rounded-lg border border-border/60 bg-card/20 space-y-2">
                  <div className="flex items-center justify-between">
                    <Label className="text-xs font-medium">
                      {t("teams.modal.configImportExport")}
                    </Label>
                    <Button
                      type="button"
                      variant="outline"
                      size="sm"
                      className="h-7 text-xs gap-1"
                      onClick={() => setJsonImportOpen(!jsonImportOpen)}
                    >
                      <ClipboardPaste className="size-3.5" />
                      <span>{t("teams.modal.importJson")}</span>
                    </Button>
                  </div>
                  {jsonImportOpen && (
                    <div className="space-y-2 pt-2 border-t border-border/40">
                      <textarea
                        value={jsonInput}
                        onChange={(e) => setJsonInput(e.target.value)}
                        placeholder="Paste Team JSON here..."
                        className="w-full h-24 p-2 rounded border border-border bg-background text-xs font-mono resize-none"
                      />
                      <div className="flex justify-end gap-2">
                        <Button
                          type="button"
                          variant="ghost"
                          size="sm"
                          className="h-7 text-xs"
                          onClick={() => setJsonImportOpen(false)}
                        >
                          {t("teams.cancel")}
                        </Button>
                        <Button
                          type="button"
                          size="sm"
                          className="h-7 text-xs bg-brand text-brand-foreground"
                          onClick={handlePasteJson}
                        >
                          {t("teams.save")}
                        </Button>
                      </div>
                    </div>
                  )}
                </div>
              </TabsContent>
            </div>
          </ScrollArea>
        </Tabs>

        {/* Footer */}
        <DialogFooter className="px-6 py-3 border-t border-border/60 bg-card/30 shrink-0">
          <Button variant="ghost" size="sm" className="h-8 text-xs" onClick={onClose}>
            {t("teams.cancel")}
          </Button>
          <Button
            size="sm"
            className="h-8 text-xs bg-brand text-brand-foreground hover:bg-brand-hover"
            disabled={!form.name.trim()}
            onClick={handleSave}
          >
            {t("teams.save")}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
