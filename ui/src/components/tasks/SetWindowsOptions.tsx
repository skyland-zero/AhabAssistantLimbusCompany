import { useTranslation } from "react-i18next";
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
import type { SetWindowsConfig } from "@/services/ipc/types";

interface SetWindowsOptionsProps {
  config: SetWindowsConfig;
  onChange: (patch: Partial<SetWindowsConfig>) => void;
  disabled?: boolean;
}

export function SetWindowsOptions({ config, onChange, disabled }: SetWindowsOptionsProps) {
  const { t } = useTranslation();

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
            <Label className="text-xs font-medium">{t("tasks.windows.resolution")}</Label>
            <p className="text-[11px] text-muted-foreground">{t("tasks.windows.resolutionDesc")}</p>
          </div>
          <Select
            value={String(config.set_win_size)}
            onValueChange={(v) => onChange({ set_win_size: Number(v) })}
            disabled={disabled}
          >
            <SelectTrigger className="h-8 w-36 text-xs">
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value="720">1280 × 720</SelectItem>
              <SelectItem value="1080">1920 × 1080</SelectItem>
              <SelectItem value="1440">2560 × 1440</SelectItem>
            </SelectContent>
          </Select>
        </div>

        <div className="flex items-center justify-between gap-4">
          <div>
            <Label className="text-xs font-medium">{t("tasks.windows.position")}</Label>
            <p className="text-[11px] text-muted-foreground">{t("tasks.windows.positionDesc")}</p>
          </div>
          <Select
            value={config.set_win_position}
            onValueChange={(v) => onChange({ set_win_position: v })}
            disabled={disabled}
          >
            <SelectTrigger className="h-8 w-36 text-xs">
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value="0">{t("tasks.windows.posCenter")}</SelectItem>
              <SelectItem value="1">{t("tasks.windows.posLeft")}</SelectItem>
              <SelectItem value="2">{t("tasks.windows.posRight")}</SelectItem>
              <SelectItem value="3">{t("tasks.windows.posNone")}</SelectItem>
            </SelectContent>
          </Select>
        </div>

        <div className="flex items-center justify-between gap-4">
          <div>
            <Label className="text-xs font-medium">{t("tasks.windows.restoreWindow")}</Label>
            <p className="text-[11px] text-muted-foreground">
              {t("tasks.windows.restoreWindowDesc")}
            </p>
          </div>
          <Switch
            checked={config.set_reduce_miscontact}
            onCheckedChange={(v) => onChange({ set_reduce_miscontact: v })}
            disabled={disabled}
          />
        </div>
      </TabsContent>

      {/* 高级设置 */}
      <TabsContent value="advanced" className="mt-3 flex flex-col gap-3.5">
        <div className="flex items-center justify-between gap-4">
          <div>
            <Label className="text-xs font-medium">{t("tasks.windows.screenshotInterval")}</Label>
            <p className="text-[11px] text-muted-foreground">
              {t("tasks.windows.screenshotIntervalDesc")}
            </p>
          </div>
          <Select
            value={String(config.screenshot_interval)}
            onValueChange={(v) => onChange({ screenshot_interval: Number(v) })}
            disabled={disabled}
          >
            <SelectTrigger className="h-8 w-36 text-xs">
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value="0.2">0.2s ({t("tasks.fast")})</SelectItem>
              <SelectItem value="0.5">0.5s ({t("tasks.default")})</SelectItem>
              <SelectItem value="1">1.0s ({t("tasks.slow")})</SelectItem>
            </SelectContent>
          </Select>
        </div>

        <div className="flex items-center justify-between gap-4">
          <div>
            <Label className="text-xs font-medium">{t("tasks.windows.mouseInterval")}</Label>
            <p className="text-[11px] text-muted-foreground">
              {t("tasks.windows.mouseIntervalDesc")}
            </p>
          </div>
          <Select
            value={String(config.mouse_action_interval)}
            onValueChange={(v) => onChange({ mouse_action_interval: Number(v) })}
            disabled={disabled}
          >
            <SelectTrigger className="h-8 w-36 text-xs">
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value="0.1">0.1s</SelectItem>
              <SelectItem value="0.3">0.3s ({t("tasks.default")})</SelectItem>
              <SelectItem value="0.5">0.5s</SelectItem>
            </SelectContent>
          </Select>
        </div>

        <div className="flex items-center justify-between gap-4">
          <div>
            <Label className="text-xs font-medium">{t("tasks.windows.usePostMessage")}</Label>
            <p className="text-[11px] text-muted-foreground">
              {t("tasks.windows.usePostMessageDesc")}
            </p>
          </div>
          <Switch
            checked={config.use_post_message}
            onCheckedChange={(v) => onChange({ use_post_message: v })}
            disabled={disabled}
          />
        </div>
      </TabsContent>
    </Tabs>
  );
}
