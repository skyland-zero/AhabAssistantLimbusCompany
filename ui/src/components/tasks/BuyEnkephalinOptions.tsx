import { Minus, Plus } from "lucide-react";
import { useTranslation } from "react-i18next";
import { Button } from "@/components/ui/button";
import { Label } from "@/components/ui/label";
import { Switch } from "@/components/ui/switch";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import type { BuyEnkephalinConfig } from "@/services/ipc/types";

interface BuyEnkephalinOptionsProps {
  config: BuyEnkephalinConfig;
  onChange: (patch: Partial<BuyEnkephalinConfig>) => void;
  disabled?: boolean;
}

export function BuyEnkephalinOptions({ config, onChange, disabled }: BuyEnkephalinOptionsProps) {
  const { t } = useTranslation();

  const changeTimes = (delta: number) => {
    onChange({
      set_lunacy_to_enkephalin: Math.max(0, Math.min(10, config.set_lunacy_to_enkephalin + delta)),
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
        <div className="flex items-center justify-between gap-4">
          <div>
            <Label className="text-xs font-medium">{t("tasks.enkephalin.buyTimes")}</Label>
            <p className="text-[11px] text-muted-foreground">
              {t("tasks.enkephalin.buyTimesDesc")}
            </p>
          </div>
          <div className="flex items-center gap-1.5">
            <Button
              type="button"
              variant="outline"
              size="icon"
              className="size-7"
              disabled={disabled || config.set_lunacy_to_enkephalin <= 0}
              onClick={() => changeTimes(-1)}
            >
              <Minus className="size-3.5" />
            </Button>
            <span className="w-9 text-center font-mono text-xs font-medium">
              {config.set_lunacy_to_enkephalin}
            </span>
            <Button
              type="button"
              variant="outline"
              size="icon"
              className="size-7"
              disabled={disabled || config.set_lunacy_to_enkephalin >= 10}
              onClick={() => changeTimes(1)}
            >
              <Plus className="size-3.5" />
            </Button>
          </div>
        </div>
      </TabsContent>

      {/* 高级设置 */}
      <TabsContent value="advanced" className="mt-3 flex flex-col gap-3.5">
        <div className="flex items-center justify-between gap-4">
          <div>
            <Label className="text-xs font-medium">{t("tasks.enkephalin.drGrandet")}</Label>
            <p className="text-[11px] text-muted-foreground">
              {t("tasks.enkephalin.drGrandetDesc")}
            </p>
          </div>
          <Switch
            checked={config.Dr_Grandet_mode}
            onCheckedChange={(v) => onChange({ Dr_Grandet_mode: v })}
            disabled={disabled}
          />
        </div>

        <div className="flex items-center justify-between gap-4">
          <div>
            <Label className="text-xs font-medium">{t("tasks.enkephalin.skipEnkephalin")}</Label>
            <p className="text-[11px] text-muted-foreground">
              {t("tasks.enkephalin.skipEnkephalinDesc")}
            </p>
          </div>
          <Switch
            checked={config.skip_enkephalin}
            onCheckedChange={(v) => onChange({ skip_enkephalin: v })}
            disabled={disabled}
          />
        </div>
      </TabsContent>
    </Tabs>
  );
}
