import { Minus, Plus } from "lucide-react";
import { useTranslation } from "react-i18next";
import { Button } from "@/components/ui/button";
import { Label } from "@/components/ui/label";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { Switch } from "@/components/ui/switch";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import type { DailyTaskConfig, TeamSummary } from "@/services/ipc/types";

interface DailyTaskOptionsProps {
  config: DailyTaskConfig;
  teams: TeamSummary[];
  onChange: (patch: Partial<DailyTaskConfig>) => void;
  disabled?: boolean;
}

export function DailyTaskOptions({ config, teams, onChange, disabled }: DailyTaskOptionsProps) {
  const { t } = useTranslation();

  const changeExpCount = (delta: number) => {
    onChange({ set_EXP_count: Math.max(0, Math.min(99, config.set_EXP_count + delta)) });
  };

  const changeThreadCount = (delta: number) => {
    onChange({ set_thread_count: Math.max(0, Math.min(99, config.set_thread_count + delta)) });
  };

  const changeContinuousSelect = (delta: number) => {
    onChange({
      use_continuous_combat_select: Math.max(
        1,
        Math.min(10, config.use_continuous_combat_select + delta),
      ),
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
        {/* 经验本次数 */}
        <div className="flex items-center justify-between gap-4">
          <div>
            <Label className="text-xs font-medium">{t("tasks.daily.expCount")}</Label>
            <p className="text-[11px] text-muted-foreground">{t("tasks.daily.expCountDesc")}</p>
          </div>
          <div className="flex items-center gap-1.5">
            <Button
              type="button"
              variant="outline"
              size="icon"
              className="size-7"
              disabled={disabled || config.set_EXP_count <= 0}
              onClick={() => changeExpCount(-1)}
            >
              <Minus className="size-3.5" />
            </Button>
            <span className="w-9 text-center font-mono text-xs font-medium">
              {config.set_EXP_count}
            </span>
            <Button
              type="button"
              variant="outline"
              size="icon"
              className="size-7"
              disabled={disabled || config.set_EXP_count >= 99}
              onClick={() => changeExpCount(1)}
            >
              <Plus className="size-3.5" />
            </Button>
          </div>
        </div>

        {/* 纽本次数 */}
        <div className="flex items-center justify-between gap-4">
          <div>
            <Label className="text-xs font-medium">{t("tasks.daily.threadCount")}</Label>
            <p className="text-[11px] text-muted-foreground">{t("tasks.daily.threadCountDesc")}</p>
          </div>
          <div className="flex items-center gap-1.5">
            <Button
              type="button"
              variant="outline"
              size="icon"
              className="size-7"
              disabled={disabled || config.set_thread_count <= 0}
              onClick={() => changeThreadCount(-1)}
            >
              <Minus className="size-3.5" />
            </Button>
            <span className="w-9 text-center font-mono text-xs font-medium">
              {config.set_thread_count}
            </span>
            <Button
              type="button"
              variant="outline"
              size="icon"
              className="size-7"
              disabled={disabled || config.set_thread_count >= 99}
              onClick={() => changeThreadCount(1)}
            >
              <Plus className="size-3.5" />
            </Button>
          </div>
        </div>

        {/* 默认日常编队 */}
        <div className="flex items-center justify-between gap-4">
          <div>
            <Label className="text-xs font-medium">{t("tasks.daily.teamSelect")}</Label>
            <p className="text-[11px] text-muted-foreground">{t("tasks.daily.teamSelectDesc")}</p>
          </div>
          <Select
            value={String(config.daily_teams)}
            onValueChange={(v) => onChange({ daily_teams: Number(v) })}
            disabled={disabled}
          >
            <SelectTrigger className="h-8 w-36 text-xs">
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              {teams.length > 0 ? (
                teams.map((tm, idx) => (
                  <SelectItem key={tm.id} value={String(idx + 1)}>
                    {tm.name}
                  </SelectItem>
                ))
              ) : (
                <SelectItem value="1">{t("tasks.daily.team1")}</SelectItem>
              )}
            </SelectContent>
          </Select>
        </div>

        {/* 连续作战 */}
        <div className="flex items-center justify-between gap-4 rounded-md border border-border/50 bg-muted/20 p-2.5">
          <div>
            <Label className="text-xs font-medium">{t("tasks.daily.continuousCombat")}</Label>
            <p className="text-[11px] text-muted-foreground">
              {t("tasks.daily.continuousCombatDesc")}
            </p>
          </div>
          <div className="flex items-center gap-3">
            {config.use_continuous_combat && (
              <div className="flex items-center gap-1">
                <Button
                  type="button"
                  variant="outline"
                  size="icon"
                  className="size-6"
                  disabled={disabled || config.use_continuous_combat_select <= 1}
                  onClick={() => changeContinuousSelect(-1)}
                >
                  <Minus className="size-3" />
                </Button>
                <span className="w-6 text-center font-mono text-xs">
                  {config.use_continuous_combat_select}
                </span>
                <Button
                  type="button"
                  variant="outline"
                  size="icon"
                  className="size-6"
                  disabled={disabled || config.use_continuous_combat_select >= 10}
                  onClick={() => changeContinuousSelect(1)}
                >
                  <Plus className="size-3" />
                </Button>
              </div>
            )}
            <Switch
              checked={config.use_continuous_combat}
              onCheckedChange={(v) => onChange({ use_continuous_combat: v })}
              disabled={disabled}
            />
          </div>
        </div>
      </TabsContent>

      {/* 高级设置：针对性配队 */}
      <TabsContent value="advanced" className="mt-3 flex flex-col gap-4">
        {/* 经验本针对性配队 */}
        <div className="flex flex-col gap-2 rounded-md border border-border/50 bg-muted/20 p-2.5">
          <div className="flex items-center justify-between">
            <div>
              <Label className="text-xs font-medium">{t("tasks.daily.targetedExp")}</Label>
              <p className="text-[11px] text-muted-foreground">
                {t("tasks.daily.targetedExpDesc")}
              </p>
            </div>
            <Switch
              checked={config.targeted_teaming_EXP}
              onCheckedChange={(v) => onChange({ targeted_teaming_EXP: v })}
              disabled={disabled}
            />
          </div>

          {config.targeted_teaming_EXP && (
            <div className="mt-2 grid grid-cols-2 gap-2 pt-2 border-t border-border/40">
              <DayTeamSelector
                label={t("tasks.daily.expSlash")}
                value={config.EXP_day_1_2}
                teams={teams}
                onChange={(v) => onChange({ EXP_day_1_2: v })}
                disabled={disabled}
              />
              <DayTeamSelector
                label={t("tasks.daily.expPierce")}
                value={config.EXP_day_3_4}
                teams={teams}
                onChange={(v) => onChange({ EXP_day_3_4: v })}
                disabled={disabled}
              />
              <DayTeamSelector
                label={t("tasks.daily.expBlunt")}
                value={config.EXP_day_5_6}
                teams={teams}
                onChange={(v) => onChange({ EXP_day_5_6: v })}
                disabled={disabled}
              />
              <DayTeamSelector
                label={t("tasks.daily.expSun")}
                value={config.EXP_day_7}
                teams={teams}
                onChange={(v) => onChange({ EXP_day_7: v })}
                disabled={disabled}
              />
            </div>
          )}
        </div>

        {/* 纽本针对性配队 */}
        <div className="flex flex-col gap-2 rounded-md border border-border/50 bg-muted/20 p-2.5">
          <div className="flex items-center justify-between">
            <div>
              <Label className="text-xs font-medium">{t("tasks.daily.targetedThread")}</Label>
              <p className="text-[11px] text-muted-foreground">
                {t("tasks.daily.targetedThreadDesc")}
              </p>
            </div>
            <Switch
              checked={config.targeted_teaming_thread}
              onCheckedChange={(v) => onChange({ targeted_teaming_thread: v })}
              disabled={disabled}
            />
          </div>

          {config.targeted_teaming_thread && (
            <div className="mt-2 grid grid-cols-2 gap-2 pt-2 border-t border-border/40">
              <DayTeamSelector
                label={t("tasks.daily.threadMon")}
                value={config.thread_day_1}
                teams={teams}
                onChange={(v) => onChange({ thread_day_1: v })}
                disabled={disabled}
              />
              <DayTeamSelector
                label={t("tasks.daily.threadTue")}
                value={config.thread_day_2}
                teams={teams}
                onChange={(v) => onChange({ thread_day_2: v })}
                disabled={disabled}
              />
              <DayTeamSelector
                label={t("tasks.daily.threadWed")}
                value={config.thread_day_3}
                teams={teams}
                onChange={(v) => onChange({ thread_day_3: v })}
                disabled={disabled}
              />
              <DayTeamSelector
                label={t("tasks.daily.threadThu")}
                value={config.thread_day_4}
                teams={teams}
                onChange={(v) => onChange({ thread_day_4: v })}
                disabled={disabled}
              />
              <DayTeamSelector
                label={t("tasks.daily.threadFri")}
                value={config.thread_day_5}
                teams={teams}
                onChange={(v) => onChange({ thread_day_5: v })}
                disabled={disabled}
              />
              <DayTeamSelector
                label={t("tasks.daily.threadSat")}
                value={config.thread_day_6}
                teams={teams}
                onChange={(v) => onChange({ thread_day_6: v })}
                disabled={disabled}
              />
              <DayTeamSelector
                label={t("tasks.daily.threadSun")}
                value={config.thread_day_7}
                teams={teams}
                onChange={(v) => onChange({ thread_day_7: v })}
                disabled={disabled}
              />
            </div>
          )}
        </div>
      </TabsContent>
    </Tabs>
  );
}

function DayTeamSelector({
  label,
  value,
  teams,
  onChange,
  disabled,
}: {
  label: string;
  value: number;
  teams: TeamSummary[];
  onChange: (v: number) => void;
  disabled?: boolean;
}) {
  return (
    <div className="flex items-center justify-between gap-1 text-xs">
      <span className="truncate text-muted-foreground">{label}</span>
      <Select value={String(value)} onValueChange={(v) => onChange(Number(v))} disabled={disabled}>
        <SelectTrigger className="h-7 w-24 text-[11px]">
          <SelectValue />
        </SelectTrigger>
        <SelectContent>
          {teams.length > 0 ? (
            teams.map((tm, idx) => (
              <SelectItem key={tm.id} value={String(idx + 1)}>
                {tm.name}
              </SelectItem>
            ))
          ) : (
            <SelectItem value="1">队伍 1</SelectItem>
          )}
        </SelectContent>
      </Select>
    </div>
  );
}
