import { CheckSquare, Pause, Play, RotateCcw, Settings2, Square } from "lucide-react";
import { useTranslation } from "react-i18next";
import { formatAfterCompletionSummary } from "@/components/tasks/AfterCompletionModal";
import { Button } from "@/components/ui/button";
import { cn } from "@/lib/utils";
import type { AfterCompletionConfig, ExecutionState } from "@/services/ipc/types";

interface ExecutionToolbarProps {
  executionState: ExecutionState;
  afterCompletion: AfterCompletionConfig;
  onSelectAll: () => void;
  onClearAll: () => void;
  onOpenAfterCompletion: () => void;
  onStart: () => void;
  onStop: () => void;
  onPause: () => void;
  onResume: () => void;
  startShortcut?: string;
}

export function ExecutionToolbar({
  executionState,
  afterCompletion,
  onSelectAll,
  onClearAll,
  onOpenAfterCompletion,
  onStart,
  onStop,
  onPause,
  onResume,
  startShortcut = "F10",
}: ExecutionToolbarProps) {
  const { t } = useTranslation();
  const isRunning = executionState === "running";
  const isPaused = executionState === "paused";
  const isBusy = isRunning || isPaused;

  const afterSummary = formatAfterCompletionSummary(afterCompletion, t);

  return (
    <footer className="sticky bottom-0 z-10 flex flex-wrap items-center justify-between gap-3 border-t border-border bg-card/95 p-3 backdrop-blur-md">
      {/* 左侧：全选/清空 + 结束后操作（方案A） */}
      <div className="flex items-center gap-1.5">
        <Button
          type="button"
          variant="outline"
          size="sm"
          className="h-8 gap-1 px-2.5 text-xs"
          disabled={isBusy}
          onClick={onSelectAll}
        >
          <CheckSquare className="size-3.5" />
          <span>{t("tasks.toolbar.selectAll")}</span>
        </Button>
        <Button
          type="button"
          variant="outline"
          size="sm"
          className="h-8 gap-1 px-2.5 text-xs text-muted-foreground hover:text-foreground"
          disabled={isBusy}
          onClick={onClearAll}
        >
          <RotateCcw className="size-3.5" />
          <span>{t("tasks.toolbar.clearAll")}</span>
        </Button>
        <div className="mx-1 h-4 w-px bg-muted-foreground/25" />
        <Button
          type="button"
          variant="ghost"
          size="sm"
          className="h-8 max-w-[280px] gap-1.5 px-2.5 text-xs text-muted-foreground hover:text-foreground"
          onClick={onOpenAfterCompletion}
          title={t("afterCompletion.hint")}
        >
          <Settings2 className="size-3.5 shrink-0 text-brand" />
          <span className="truncate">{afterSummary}</span>
        </Button>
      </div>

      {/* 右侧：调度与执行总控按钮 (MXU 风格) */}
      <div className="flex items-center gap-2">
        {/* 暂停 / 继续按钮 */}
        {isBusy && (
          <Button
            type="button"
            variant="outline"
            size="sm"
            className="h-9 gap-1.5 px-3 text-xs"
            onClick={isPaused ? onResume : onPause}
          >
            {isPaused ? (
              <>
                <Play className="size-3.5 fill-current text-success" />
                <span>{t("tasks.toolbar.resume")}</span>
              </>
            ) : (
              <>
                <Pause className="size-3.5 text-warning" />
                <span>{t("tasks.toolbar.pause")}</span>
              </>
            )}
          </Button>
        )}

        {/* Link Start / Stop 主按钮 */}
        {isBusy ? (
          <Button
            type="button"
            size="sm"
            className="h-9 gap-2 bg-danger px-5 text-xs font-semibold text-white ring-2 ring-danger/30 hover:bg-danger/90"
            onClick={onStop}
          >
            <Square className="size-4 fill-current" />
            <span>Stop!</span>
          </Button>
        ) : (
          <Button
            type="button"
            size="sm"
            className={cn(
              "h-9 gap-2 px-5 text-xs font-semibold",
              "bg-brand text-brand-foreground hover:bg-brand-hover active:scale-[0.97] transition-all",
            )}
            onClick={onStart}
          >
            <Play className="size-4 fill-current" />
            <span>Link Start!</span>
            {startShortcut && (
              <kbd className="hidden md:inline-block rounded bg-black/20 px-1 py-0.5 text-[10px] font-mono font-normal">
                {startShortcut}
              </kbd>
            )}
          </Button>
        )}
      </div>
    </footer>
  );
}
