import { useEffect } from "react";
import { useTranslation } from "react-i18next";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Label } from "@/components/ui/label";
import { Separator } from "@/components/ui/separator";
import { Switch } from "@/components/ui/switch";
import { changeLanguage } from "@/i18n";
import { cn } from "@/lib/utils";
import { useAppStore } from "@/stores/appStore";
import { ACCENT_PRESETS, applyTheme, type ThemeMode } from "@/themes";

const MODES: ThemeMode[] = ["light", "dark", "system"];

export function SettingsPage() {
  const { t, i18n } = useTranslation();
  const themeMode = useAppStore((s) => s.themeMode);
  const setThemeMode = useAppStore((s) => s.setThemeMode);
  const accentId = useAppStore((s) => s.accentId);
  const setAccent = useAppStore((s) => s.setAccent);
  const language = useAppStore((s) => s.language);
  const setLanguage = useAppStore((s) => s.setLanguage);

  // mode=system 时跟随系统变化
  useEffect(() => {
    if (themeMode !== "system") return;
    const mq = window.matchMedia("(prefers-color-scheme: dark)");
    const onChange = () => applyTheme("system", accentId);
    mq.addEventListener("change", onChange);
    return () => mq.removeEventListener("change", onChange);
  }, [themeMode, accentId]);

  return (
    <div className="h-full overflow-y-auto p-6">
      <div className="mx-auto max-w-2xl">
        <h1 className="mb-6 text-xl font-semibold">{t("pages.settings.title")}</h1>

        {/* 外观 */}
        <Card>
          <CardHeader>
            <CardTitle className="text-base">外观 / Appearance</CardTitle>
          </CardHeader>
          <CardContent className="flex flex-col gap-5">
            <div className="flex items-center justify-between">
              <Label>主题模式</Label>
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
                    {m === "light" ? "浅色" : m === "dark" ? "深色" : "跟随系统"}
                  </button>
                ))}
              </div>
            </div>

            <Separator />

            <div className="flex items-center justify-between">
              <Label>强调色</Label>
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
              <Label>语言 / Language</Label>
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

        {/* 其余分区占位 */}
        {["热键", "通知", "更新"].map((section) => (
          <Card key={section} className="mt-4 opacity-60">
            <CardHeader>
              <CardTitle className="text-base">{section}</CardTitle>
            </CardHeader>
            <CardContent>
              <div className="flex items-center gap-3 py-2">
                <Switch disabled />
                <span className="text-sm text-muted-foreground">M4 里程碑接入（config.yaml）</span>
              </div>
            </CardContent>
          </Card>
        ))}

        <p className="mt-6 text-center text-xs text-muted-foreground" data-testid="lang-indicator">
          {i18n.language} · v0.1.0-m0
        </p>
      </div>
    </div>
  );
}
