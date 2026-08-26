import { useTranslation } from "react-i18next";
import { Label } from "@/components/ui/label";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import type { GetRewardConfig } from "@/services/ipc/types";

interface GetRewardOptionsProps {
  config: GetRewardConfig;
  onChange: (patch: Partial<GetRewardConfig>) => void;
  disabled?: boolean;
}

export function GetRewardOptions({ config, onChange, disabled }: GetRewardOptionsProps) {
  const { t } = useTranslation();

  return (
    <div className="flex flex-col gap-3 py-1">
      <div className="flex items-center justify-between gap-4">
        <div>
          <Label className="text-xs font-medium">{t("tasks.rewards.mode")}</Label>
          <p className="text-[11px] text-muted-foreground">{t("tasks.rewards.modeDesc")}</p>
        </div>
        <Select
          value={String(config.set_get_prize)}
          onValueChange={(v) => onChange({ set_get_prize: Number(v) })}
          disabled={disabled}
        >
          <SelectTrigger className="h-8 w-44 text-xs">
            <SelectValue />
          </SelectTrigger>
          <SelectContent>
            <SelectItem value="0">{t("tasks.rewards.all")}</SelectItem>
            <SelectItem value="1">{t("tasks.rewards.lunacyAndPass")}</SelectItem>
            <SelectItem value="2">{t("tasks.rewards.mailOnly")}</SelectItem>
          </SelectContent>
        </Select>
      </div>
    </div>
  );
}
