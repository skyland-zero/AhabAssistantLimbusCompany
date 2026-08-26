import { Copy, Hexagon, Minus, Square, X } from "lucide-react";
import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { isTauri } from "@/lib/env";

/** 自绘标题栏（无边框窗口），参考 MXU TitleBar */
export function TitleBar() {
  const { t } = useTranslation();
  const [isMaximized, setIsMaximized] = useState(false);

  useEffect(() => {
    if (!isTauri()) return;
    let unlisten: (() => void) | null = null;
    void (async () => {
      const { getCurrentWindow } = await import("@tauri-apps/api/window");
      const win = getCurrentWindow();
      setIsMaximized(await win.isMaximized());
      const dispose = await win.onResized(async () => {
        setIsMaximized(await win.isMaximized());
      });
      unlisten = dispose;
    })();
    return () => unlisten?.();
  }, []);

  const minimize = async () => {
    if (!isTauri()) return;
    (await import("@tauri-apps/api/window")).getCurrentWindow().minimize();
  };
  const toggleMaximize = async () => {
    if (!isTauri()) return;
    (await import("@tauri-apps/api/window")).getCurrentWindow().toggleMaximize();
  };
  const close = async () => {
    if (!isTauri()) return;
    (await import("@tauri-apps/api/window")).getCurrentWindow().close();
  };

  return (
    <header className="flex h-10 shrink-0 select-none items-center justify-between border-b border-border bg-card">
      {/* 左：logo + 标题（可拖拽区） */}
      <div data-tauri-drag-region className="flex h-full flex-1 items-center gap-2 pl-3">
        <Hexagon className="size-4 text-brand" strokeWidth={2.5} />
        <span data-tauri-drag-region className="text-xs font-medium text-muted-foreground">
          {t("app.name")}
        </span>
      </div>

      {/* 右：窗口控制按钮（Windows 风格） */}
      {isTauri() && (
        <div className="flex h-full">
          <TitleBarButton label={t("titlebar.minimize")} onClick={minimize}>
            <Minus className="size-4" />
          </TitleBarButton>
          <TitleBarButton
            label={isMaximized ? t("titlebar.unmaximize") : t("titlebar.maximize")}
            onClick={toggleMaximize}
          >
            {isMaximized ? (
              <Copy className="size-3.5 -scale-x-100" />
            ) : (
              <Square className="size-3" />
            )}
          </TitleBarButton>
          <TitleBarButton label={t("titlebar.close")} onClick={close} danger>
            <X className="size-4" />
          </TitleBarButton>
        </div>
      )}
    </header>
  );
}

function TitleBarButton({
  label,
  onClick,
  danger,
  children,
}: {
  label: string;
  onClick: () => void;
  danger?: boolean;
  children: React.ReactNode;
}) {
  return (
    <button
      type="button"
      aria-label={label}
      title={label}
      onClick={() => void onClick()}
      className={`flex h-full w-12 items-center justify-center text-muted-foreground transition-colors ${
        danger ? "hover:bg-red-600 hover:text-white" : "hover:bg-secondary hover:text-foreground"
      }`}
    >
      {children}
    </button>
  );
}
