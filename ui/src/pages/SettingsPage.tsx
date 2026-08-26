import { ExternalLink, SearchCheck } from "lucide-react";
import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { toast } from "sonner";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Label } from "@/components/ui/label";
import { Separator } from "@/components/ui/separator";
import { Switch } from "@/components/ui/switch";
import { changeLanguage } from "@/i18n";
import { isTauri } from "@/lib/env";
import { cn } from "@/lib/utils";
import { getIpc } from "@/services/ipc/client";
import type { HotkeyConfig, UpdateInfo } from "@/services/ipc/types";
import { useAppStore } from "@/stores/appStore";
import { ACCENT_PRESETS, type ThemeMode } from "@/themes";

const MODES: ThemeMode[] = ["light", "dark", "system"];
const REPO_URL = "https://github.com/KIYI671/AhabAssistantLimbusCompany";

const MODIFIER_KEYS = new Set(["Control", "Shift", "Alt", "Meta"]);

/** 把键盘事件转成组合键字符串，如 Ctrl+Shift+Space */
function comboFromEvent(e: React.KeyboardEvent): string | null {
  if (MODIFIER_KEYS.has(e.key)) return null;
  const parts: string[] = [];
  if (e.ctrlKey) parts.push("Ctrl");
  if (e.metaKey) parts.push("Super");
  if (e.altKey) parts.push("Alt");
  if (e.shiftKey) parts.push("Shift");
  const key = e.key.length === 1 ? e.key.toUpperCase() : e.key;
  parts.push(key);
  return parts.join("+");
}

export function SettingsPage() {
  const { t, i18n } = useTranslation();
  const themeMode = useAppStore((s) => s.themeMode);
  const setThemeMode = useAppStore((s) => s.setThemeMode);
  const accentId = useAppStore((s) => s.accentId);
  const setAccent = useAppStore((s) => s.setAccent);
  const language = useAppStore((s) => s.language);
  const setLanguage = useAppStore((s) => s.setLanguage);

  // 注：主题应用与 system 模式监听统一在 App.tsx 中处理，页面只负责更新 store。

  /* 热键（mock 存储） */
  const [hotkey, setHotkeyState] = useState<HotkeyConfig>({ startStop: "", enabled: false });
  const [capturing, setCapturing] = useState(false);

  useEffect(() => {
    void (async () => {
      setHotkeyState(await (await getIpc()).request<HotkeyConfig>("hotkey.get"));
    })();
  }, []);

  const saveHotkey = async (next: HotkeyConfig) => {
    setHotkeyState(next);
    await (await getIpc()).request("hotkey.set", next);
  };

  const openRepo = () => {
    if (isTauri()) {
      void import("@tauri-apps/plugin-opener").then(({ openUrl }) => openUrl(REPO_URL));
    } else {
      window.open(REPO_URL, "_blank");
    }
  };

  const checkUpdate = async () => {
    const info = await (await getIpc()).request<UpdateInfo>("app.checkUpdate");
    if (info.updateAvailable) {
      toast.info(t("resources.updateAvailable", { version: info.latest }));
    } else {
      toast.success(t("settings.upToDate", { version: info.latest }));
    }
  };

  return (
    <div className="h-full overflow-y-auto p-6">
      <div className="mx-auto max-w-2xl">
        <h1 className="mb-6 text-xl font-semibold">{t("pages.settings.title")}</h1>

        {/* 外观 */}
        <Card>
          <CardHeader>
            <CardTitle className="text-base">{t("settings.appearance")}</CardTitle>
          </CardHeader>
          <CardContent className="flex flex-col gap-5">
            <div className="flex items-center justify-between">
              <Label>{t("settings.themeMode")}</Label>
              <div className="flex gap-1 rounded-lg bg-muted p-1">
                {MODES.map((m) => (
                  <button
                    key={m}
                    type="button"
                    onClick={() => setThemeMode(m)}
                    className={cn(
                      "rounded-md px-3 py-1 text-xs capitalize transition-colors",
                      themeMode === m
                        ? "bg-background shadow-sm font-medium"
                        : "text-muted-foreground hover:text-foreground",
                    )}
                  >
                    {m === "light"
                      ? t("settings.light")
                      : m === "dark"
                        ? t("settings.dark")
                        : t("settings.system")}
                  </button>
                ))}
              </div>
            </div>

            <Separator />

            <div className="flex items-center justify-between">
              <Label>{t("settings.accent")}</Label>
              <div className="flex items-center gap-2">
                {ACCENT_PRESETS.map((preset) => (
                  <button
                    key={preset.id}
                    type="button"
                    aria-label={preset.name}
                    title={preset.name}
                    onClick={() => setAccent(preset.id)}
                    style={{ backgroundColor: preset.light.brand }}
                    className={cn(
                      "size-7 rounded-full transition-transform",
                      accentId === preset.id
                        ? "scale-110 ring-2 ring-ring ring-offset-2 ring-offset-card"
                        : "opacity-70 hover:opacity-100",
                    )}
                  />
                ))}
              </div>
            </div>

            <Separator />

            <div className="flex items-center justify-between">
              <Label>{t("settings.language")}</Label>
              <div className="flex gap-1 rounded-lg bg-muted p-1">
                {(["zh-CN", "en-US"] as const).map((lang) => (
                  <button
                    key={lang}
                    type="button"
                    onClick={() => {
                      setLanguage(lang);
                      changeLanguage(lang);
                    }}
                    className={cn(
                      "rounded-md px-3 py-1 text-xs transition-colors",
                      language === lang
                        ? "bg-background shadow-sm font-medium"
                        : "text-muted-foreground hover:text-foreground",
                    )}
                  >
                    {lang === "zh-CN" ? "简体中文" : "English"}
                  </button>
                ))}
              </div>
            </div>
          </CardContent>
        </Card>

        {/* 热键 */}
        <Card className="mt-4">
          <CardHeader>
            <CardTitle className="text-base">{t("settings.hotkeys")}</CardTitle>
          </CardHeader>
          <CardContent className="flex flex-col gap-4">
            <div className="flex items-center justify-between">
              <Label>{t("settings.hotkeyEnable")}</Label>
              <Switch
                checked={hotkey.enabled}
                onCheckedChange={(enabled) => void saveHotkey({ ...hotkey, enabled })}
              />
            </div>
            <div className="flex items-center justify-between gap-4">
              <Label>{t("settings.hotkeyStartStop")}</Label>
              <div className="flex items-center gap-2">
                <button
                  type="button"
                  tabIndex={0}
                  onClick={() => setCapturing(true)}
                  onBlur={() => setCapturing(false)}
                  onKeyDown={(e) => {
                    if (!capturing) return;
                    e.preventDefault();
                    const combo = comboFromEvent(e);
                    if (combo) {
                      void saveHotkey({ ...hotkey, startStop: combo });
                      setCapturing(false);
                    }
                  }}
                  className={cn(
                    "min-w-36 rounded-md border border-input bg-background px-3 py-1.5 text-center font-mono text-xs transition-colors",
                    capturing ? "border-brand ring-2 ring-ring/40 text-brand" : "hover:border-ring",
                    !hotkey.startStop && "text-muted-foreground",
                  )}
                >
                  {capturing
                    ? t("settings.hotkeyHint")
                    : hotkey.startStop || t("settings.hotkeyStartStop")}
                </button>
                {hotkey.startStop && (
                  <Button
                    size="sm"
                    variant="ghost"
                    className="text-xs"
                    onClick={() => void saveHotkey({ ...hotkey, startStop: "" })}
                  >
                    {t("settings.hotkeyClear")}
                  </Button>
                )}
              </div>
            </div>
          </CardContent>
        </Card>

        {/* 通知（占位） */}
        <Card className="mt-4 opacity-60">
          <CardHeader>
            <CardTitle className="text-base">{t("settings.notifications")}</CardTitle>
          </CardHeader>
          <CardContent>
            <div className="flex items-center gap-3 py-2">
              <Switch disabled />
              <span className="text-sm text-muted-foreground">{t("settings.configPending")}</span>
            </div>
          </CardContent>
        </Card>

        {/* 更新 */}
        <Card className="mt-4">
          <CardHeader>
            <CardTitle className="text-base">{t("settings.update")}</CardTitle>
          </CardHeader>
          <CardContent>
            <Button size="sm" variant="outline" onClick={() => void checkUpdate()}>
              <SearchCheck className="size-4" /> {t("settings.checkUpdate")}
            </Button>
          </CardContent>
        </Card>

        {/* 关于 */}
        <Card className="mt-4">
          <CardHeader>
            <CardTitle className="text-base">{t("settings.about")}</CardTitle>
          </CardHeader>
          <CardContent className="flex flex-col gap-3">
            <div className="flex items-center justify-between">
              <Label>{t("settings.version")}</Label>
              <span className="font-mono text-sm">v{__APP_VERSION__}-m0</span>
            </div>
            <Separator />
            <div className="flex items-center justify-between">
              <Label>{t("settings.repo")}</Label>
              <Button size="sm" variant="ghost" className="gap-1.5 text-xs" onClick={openRepo}>
                GitHub <ExternalLink className="size-3.5" />
              </Button>
            </div>
          </CardContent>
        </Card>

        <p className="mt-6 text-center text-xs text-muted-foreground" data-testid="lang-indicator">
          {i18n.language} · v{__APP_VERSION__}-m0
        </p>
      </div>
    </div>
  );
}
