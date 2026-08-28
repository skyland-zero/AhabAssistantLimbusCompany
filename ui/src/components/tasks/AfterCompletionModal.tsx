import { useState } from "react";
import { useTranslation } from "react-i18next";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Label } from "@/components/ui/label";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { Switch } from "@/components/ui/switch";
import type {
  AfterCompletionConfig,
  AfterExitAction,
  AfterPowerAction,
} from "@/services/ipc/types";

interface AfterCompletionModalProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  config: AfterCompletionConfig;
  onSave: (config: AfterCompletionConfig) => void;
}

export function AfterCompletionModal({
  open,
  onOpenChange,
  config,
  onSave,
}: AfterCompletionModalProps) {
  const { t } = useTranslation();

  const [actions, setActions] = useState<AfterExitAction[]>(config.actions);
  const [powerAction, setPowerAction] = useState<AfterPowerAction>(config.powerAction);

  const toggleAction = (act: AfterExitAction, checked: boolean) => {
    if (checked) {
      setActions((prev) => [...prev, act]);
    } else {
      setActions((prev) => prev.filter((a) => a !== act));
    }
  };

  const handleApply = (keepAsDefault: boolean) => {
    onSave({
      actions,
      powerAction,
      keepAfterCompletion: keepAsDefault,
    });
    onOpenChange(false);
  };

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-w-sm">
        <DialogHeader>
          <DialogTitle className="text-base">{t("afterCompletion.title")}</DialogTitle>
        </DialogHeader>

        <div className="flex flex-col gap-4 py-2">
          {/* 前置动作 */}
          <div className="flex flex-col gap-2">
            <Label className="text-xs font-medium text-muted-foreground">
              {t("afterCompletion.preActionsTitle")}
            </Label>
            <div className="flex flex-col gap-2 rounded-lg border border-border bg-card p-3">
              <div className="flex items-center justify-between">
                <span className="text-xs">{t("afterCompletion.exitGame")}</span>
                <Switch
                  checked={actions.includes("exit_game")}
                  onCheckedChange={(c) => toggleAction("exit_game", c)}
                />
              </div>
              <div className="flex items-center justify-between">
                <span className="text-xs">{t("afterCompletion.exitEmulator")}</span>
                <Switch
                  checked={actions.includes("exit_emulator")}
                  onCheckedChange={(c) => toggleAction("exit_emulator", c)}
                />
              </div>
              <div className="flex items-center justify-between">
                <span className="text-xs">{t("afterCompletion.exitAalc")}</span>
                <Switch
                  checked={actions.includes("exit_aalc")}
                  onCheckedChange={(c) => toggleAction("exit_aalc", c)}
                />
              </div>
            </div>
          </div>

          {/* 最终电源动作 */}
          <div className="flex flex-col gap-2">
            <Label className="text-xs font-medium text-muted-foreground">
              {t("afterCompletion.powerActionTitle")}
            </Label>
            <Select
              value={powerAction}
              onValueChange={(v) => setPowerAction(v as AfterPowerAction)}
            >
              <SelectTrigger className="h-9 text-xs">
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                <SelectItem value="none">{t("afterCompletion.powerNone")}</SelectItem>
                <SelectItem value="sleep">{t("afterCompletion.powerSleep")}</SelectItem>
                <SelectItem value="hibernate">{t("afterCompletion.powerHibernate")}</SelectItem>
                <SelectItem value="lock">{t("afterCompletion.powerLock")}</SelectItem>
                <SelectItem value="shutdown">{t("afterCompletion.powerShutdown")}</SelectItem>
              </SelectContent>
            </Select>
          </div>
        </div>

        <DialogFooter className="flex gap-2 sm:justify-end">
          <Button type="button" variant="outline" size="sm" onClick={() => handleApply(false)}>
            {t("afterCompletion.applyOnce")}
          </Button>
          <Button
            type="button"
            size="sm"
            className="bg-brand text-brand-foreground hover:bg-brand-hover"
            onClick={() => handleApply(true)}
          >
            {t("afterCompletion.saveDefault")}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
