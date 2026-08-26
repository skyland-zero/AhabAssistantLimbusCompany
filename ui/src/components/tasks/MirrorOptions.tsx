import { Minus, Plus } from "lucide-react";
import { useTranslation } from "react-i18next";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Label } from "@/components/ui/label";
import { Switch } from "@/components/ui/switch";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import type { MirrorConfig, MirrorProgressPayload } from "@/services/ipc/types";

interface MirrorOptionsProps {
  config: MirrorConfig;
  progress: MirrorProgressPayload | null;
  onChange: (patch: Partial<MirrorConfig>) => void;
  disabled?: boolean;
}

export function MirrorOptions({ config, progress, onChange, disabled }: MirrorOptionsProps) {
  const { t } = useTranslation();

  const changeMirrorCount = (delta: number) => {
    onChange({
      set_mirror_count: Math.max(1, Math.min(99, config.set_mirror_count + delta)),
    });
  };

  return (
    <Tabs defaultValue="general" className="w-full">
      <TabsList className="grid h-8 w-44 grid-cols-2 text-xs">
        <TabsTrigger value="general" className="text-xs">
          {t("tasks.tabs.general")}
        </TabsTrigger>
        <TabsTrigger value="advanced" className="text-xs">
          {t("tasks.tabs.advanced")}
        </TabsTrigger>
      </TabsList>

      {/* 常规设置 */}
      <TabsContent value="general" className="mt-3 flex flex-col gap-3.5">
        {/* 运行中进度条 */}
        {progress && (
          <div className="flex flex-col gap-1.5 rounded-md border border-brand/40 bg-brand/5 p-2.5">
            <div className="flex items-center justify-between text-xs">
              <span className="font-medium text-brand">
                {t("tasks.mirror.progressTitle", {
                  mode: progress.isHard ? t("tasks.mirror.hard") : t("tasks.mirror.normal"),
                })}
              </span>
              <Badge variant="outline" className="font-mono text-[11px]">
                {progress.current} / {progress.isInfinite ? "∞" : progress.total}
              </Badge>
            </div>
            <div className="h-1.5 w-full overflow-hidden rounded-full bg-muted">
              <div
                className="h-full rounded-full bg-brand transition-[width] duration-300"
                style={{
                  width: progress.isInfinite
                    ? "100%"
                    : `${Math.min(100, (progress.current / Math.max(1, progress.total)) * 100)}%`,
                }}
              />
            </div>
          </div>
        )}

        {/* 坐牢次数设置 */}
        <div className="flex items-center justify-between gap-4">
          <div>
            <Label className="text-xs font-medium">{t("tasks.mirror.count")}</Label>
            <p className="text-[11px] text-muted-foreground">{t("tasks.mirror.countDesc")}</p>
          </div>
          <div className="flex items-center gap-3">
            {!config.infinite_dungeons ? (
              <div className="flex items-center gap-1.5">
                <Button
                  type="button"
                  variant="outline"
                  size="icon"
                  className="size-7"
                  disabled={disabled || config.set_mirror_count <= 1}
                  onClick={() => changeMirrorCount(-1)}
                >
                  <Minus className="size-3.5" />
                </Button>
                <span className="w-9 text-center font-mono text-xs font-medium">
                  {config.set_mirror_count}
                </span>
                <Button
                  type="button"
                  variant="outline"
                  size="icon"
                  className="size-7"
                  disabled={disabled || config.set_mirror_count >= 99}
                  onClick={() => changeMirrorCount(1)}
                >
                  <Plus className="size-3.5" />
                </Button>
              </div>
            ) : (
              <Badge variant="secondary" className="px-2 font-mono text-xs">
                ∞ {t("tasks.mirror.infinite")}
              </Badge>
            )}
          </div>
        </div>

        {/* 无限坐牢开关 */}
        <div className="flex items-center justify-between gap-4">
          <div>
            <Label className="text-xs font-medium">{t("tasks.mirror.infiniteTitle")}</Label>
            <p className="text-[11px] text-muted-foreground">{t("tasks.mirror.infiniteDesc")}</p>
          </div>
          <Switch
            checked={config.infinite_dungeons}
            onCheckedChange={(v) => onChange({ infinite_dungeons: v })}
            disabled={disabled}
          />
        </div>

        {/* 困难镜牢开关 */}
        <div className="flex items-center justify-between gap-4">
          <div>
            <Label className="text-xs font-medium">{t("tasks.mirror.hardMirror")}</Label>
            <p className="text-[11px] text-muted-foreground">{t("tasks.mirror.hardMirrorDesc")}</p>
          </div>
          <Switch
            checked={config.hard_mirror}
            onCheckedChange={(v) => onChange({ hard_mirror: v })}
            disabled={disabled}
          />
        </div>
      </TabsContent>

      {/* 高级设置 */}
      <TabsContent value="advanced" className="mt-3 grid grid-cols-2 gap-3">
        <OptionToggle
          label={t("tasks.mirror.noWeeklyBonuses")}
          desc={t("tasks.mirror.noWeeklyBonusesDesc")}
          checked={config.no_weekly_bonuses}
          onChange={(v) => onChange({ no_weekly_bonuses: v })}
          disabled={disabled}
        />
        <OptionToggle
          label={t("tasks.mirror.floor3Exit")}
          desc={t("tasks.mirror.floor3ExitDesc")}
          checked={config.floor_3_exit}
          onChange={(v) => onChange({ floor_3_exit: v })}
          disabled={disabled}
        />
        <OptionToggle
          label={t("tasks.mirror.saveRewards")}
          desc={t("tasks.mirror.saveRewardsDesc")}
          checked={config.save_rewards}
          onChange={(v) => onChange({ save_rewards: v })}
          disabled={disabled}
        />
        <OptionToggle
          label={t("tasks.mirror.singleBonuses")}
          desc={t("tasks.mirror.singleBonusesDesc")}
          checked={config.hard_mirror_single_bonuses}
          onChange={(v) => onChange({ hard_mirror_single_bonuses: v })}
          disabled={disabled}
        />
        <OptionToggle
          label={t("tasks.mirror.selectEventPack")}
          desc={t("tasks.mirror.selectEventPackDesc")}
          checked={config.select_event_pack}
          onChange={(v) => onChange({ select_event_pack: v })}
          disabled={disabled}
        />
        <OptionToggle
          label={t("tasks.mirror.skipEventPack")}
          desc={t("tasks.mirror.skipEventPackDesc")}
          checked={config.skip_event_pack}
          onChange={(v) => onChange({ skip_event_pack: v })}
          disabled={disabled}
        />
        <OptionToggle
          label={t("tasks.mirror.reClaimRewards")}
          desc={t("tasks.mirror.reClaimRewardsDesc")}
          checked={config.re_claim_rewards}
          onChange={(v) => onChange({ re_claim_rewards: v })}
          disabled={disabled}
        />
        <OptionToggle
          label={t("tasks.mirror.notSkipCotton")}
          desc={t("tasks.mirror.notSkipCottonDesc")}
          checked={config.not_skip_whitegossypium}
          onChange={(v) => onChange({ not_skip_whitegossypium: v })}
          disabled={disabled}
        />
        <OptionToggle
          label={t("tasks.mirror.fightToLast")}
          desc={t("tasks.mirror.fightToLastDesc")}
          checked={config.fight_to_last_man}
          onChange={(v) => onChange({ fight_to_last_man: v })}
          disabled={disabled}
        />
        <OptionToggle
          label={t("tasks.mirror.keyboardNav")}
          desc={t("tasks.mirror.keyboardNavDesc")}
          checked={config.mirror_keyboard_navigation}
          onChange={(v) => onChange({ mirror_keyboard_navigation: v })}
          disabled={disabled}
        />
      </TabsContent>
    </Tabs>
  );
}

function OptionToggle({
  label,
  desc,
  checked,
  onChange,
  disabled,
}: {
  label: string;
  desc?: string;
  checked: boolean;
  onChange: (v: boolean) => void;
  disabled?: boolean;
}) {
  return (
    <div className="flex items-center justify-between gap-2 rounded-md border border-border/50 bg-muted/20 p-2 text-xs">
      <div className="min-w-0 flex-1">
        <span className="truncate font-medium">{label}</span>
        {desc && <p className="truncate text-[10px] text-muted-foreground">{desc}</p>}
      </div>
      <Switch checked={checked} onCheckedChange={onChange} disabled={disabled} />
    </div>
  );
}
