import { ExternalLink, SearchCheck } from "lucide-react";
import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { toast } from "sonner";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { Separator } from "@/components/ui/separator";
import { Switch } from "@/components/ui/switch";
import { changeLanguage } from "@/i18n";
import { isTauri } from "@/lib/env";
import { cn } from "@/lib/utils";
import { getIpc } from "@/services/ipc/client";
import type { HotkeyConfig, SystemSettingsConfig, UpdateInfo } from "@/services/ipc/types";
import { useAppStore } from "@/stores/appStore";
import { ACCENT_PRESETS, type ThemeMode } from "@/themes";

const MODES: ThemeMode[] = ["light", "dark", "system"];
const REPO_URL = "https://github.com/KIYI671/AhabAssistantLimbusCompany";

const MODIFIER_KEYS = new Set(["Control", "Shift", "Alt", "Meta"]);

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

  /* 热键配置 */
  const [hotkey, setHotkeyState] = useState<HotkeyConfig>({
    startStop: "F10",
    pauseResume: "F11",
    enabled: true,
  });
  const [capturingTarget, setCapturingTarget] = useState<"startStop" | "pauseResume" | null>(null);

  /* 系统设置配置 */
  const [sysSettings, setSysSettings] = useState<SystemSettingsConfig>({
    simulator: true,
    simulator_type: 0,
    simulator_port: 16384,
    start_emulator_timeout: 60,
    memory_protection: true,
    minimize_to_tray: true,
    autostart: false,
    experimental_keep_screen_awake: true,
    experimental_hdr_warning: true,
    update_prerelease_enable: false,
    update_source: "GitHub",
    mirrorchyan_cdk: "",
  });

  useEffect(() => {
    void (async () => {
      const ipc = await getIpc();
      const [hk, sys] = await Promise.all([
        ipc.request<HotkeyConfig>("hotkey.get"),
        ipc.request<SystemSettingsConfig>("systemSettings.get"),
      ]);
      if (hk) setHotkeyState(hk);
      if (sys) setSysSettings(sys);
    })();
  }, []);

  const saveHotkey = async (next: HotkeyConfig) => {
    setHotkeyState(next);
    await (await getIpc()).request("hotkey.set", next);
  };

  const saveSysSettings = async (patch: Partial<SystemSettingsConfig>) => {
    setSysSettings((prev) => {
      const next = { ...prev, ...patch };
      void (async () => {
        const ipc = await getIpc();
        await ipc.request("systemSettings.set", next);
        if (patch.minimize_to_tray !== undefined && isTauri()) {
          const { invoke } = await import("@tauri-apps/api/core");
          await invoke("set_minimize_to_tray", { enabled: patch.minimize_to_tray });
        }
      })();
      return next;
    });
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
      <div className="mx-auto max-w-2xl flex flex-col gap-5 pb-10">
        {/* 1. 外观个性化 */}
        <Card>
          <CardHeader className="py-3 px-4">
            <CardTitle className="text-sm font-semibold">{t("settings.appearance")}</CardTitle>
          </CardHeader>
          <CardContent className="flex flex-col gap-4 px-4 pb-4">
            <div className="flex items-center justify-between">
              <Label className="text-xs font-medium">{t("settings.themeMode")}</Label>
              <div className="flex gap-1 rounded-lg bg-muted p-1">
                {MODES.map((m) => (
                  <button
                    key={m}
                    type="button"
                    onClick={() => setThemeMode(m)}
                    className={cn(
                      "rounded-md px-3 py-1 text-xs capitalize transition-colors",
                      themeMode === m
                        ? "bg-background shadow-xs font-medium"
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
              <Label className="text-xs font-medium">{t("settings.accent")}</Label>
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
                      "size-6 rounded-full transition-transform",
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
              <Label className="text-xs font-medium">{t("settings.language")}</Label>
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
                        ? "bg-background shadow-xs font-medium"
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

        {/* 2. 全局热键 */}
        <Card>
          <CardHeader className="py-3 px-4">
            <CardTitle className="text-sm font-semibold">{t("settings.hotkeys")}</CardTitle>
          </CardHeader>
          <CardContent className="flex flex-col gap-3.5 px-4 pb-4">
            <div className="flex items-center justify-between">
              <div>
                <Label className="text-xs font-medium">{t("settings.hotkeyEnable")}</Label>
              </div>
              <Switch
                checked={hotkey.enabled}
                onCheckedChange={(enabled) => void saveHotkey({ ...hotkey, enabled })}
              />
            </div>

            <Separator />

            <div className="flex items-center justify-between gap-4">
              <Label className="text-xs font-medium">{t("settings.hotkeyStartStop")}</Label>
              <div className="flex items-center gap-2">
                <button
                  type="button"
                  tabIndex={0}
                  onClick={() => setCapturingTarget("startStop")}
                  onBlur={() => setCapturingTarget(null)}
                  onKeyDown={(e) => {
                    if (capturingTarget !== "startStop") return;
                    e.preventDefault();
                    const combo = comboFromEvent(e);
                    if (combo) {
                      void saveHotkey({ ...hotkey, startStop: combo });
                      setCapturingTarget(null);
                    }
                  }}
                  className={cn(
                    "min-w-32 rounded-md border border-input bg-background px-3 py-1 text-center font-mono text-xs transition-colors",
                    capturingTarget === "startStop"
                      ? "border-brand ring-2 ring-ring/40 text-brand"
                      : "hover:border-ring",
                    !hotkey.startStop && "text-muted-foreground",
                  )}
                >
                  {capturingTarget === "startStop"
                    ? t("settings.hotkeyHint")
                    : hotkey.startStop || t("settings.hotkeyStartStop")}
                </button>
                {hotkey.startStop && (
                  <Button
                    size="sm"
                    variant="ghost"
                    className="h-7 text-xs px-2"
                    onClick={() => void saveHotkey({ ...hotkey, startStop: "" })}
                  >
                    {t("settings.hotkeyClear")}
                  </Button>
                )}
              </div>
            </div>

            <div className="flex items-center justify-between gap-4">
              <Label className="text-xs font-medium">{t("settings.hotkeyPauseResume")}</Label>
              <div className="flex items-center gap-2">
                <button
                  type="button"
                  tabIndex={0}
                  onClick={() => setCapturingTarget("pauseResume")}
                  onBlur={() => setCapturingTarget(null)}
                  onKeyDown={(e) => {
                    if (capturingTarget !== "pauseResume") return;
                    e.preventDefault();
                    const combo = comboFromEvent(e);
                    if (combo) {
                      void saveHotkey({ ...hotkey, pauseResume: combo });
                      setCapturingTarget(null);
                    }
                  }}
                  className={cn(
                    "min-w-32 rounded-md border border-input bg-background px-3 py-1 text-center font-mono text-xs transition-colors",
                    capturingTarget === "pauseResume"
                      ? "border-brand ring-2 ring-ring/40 text-brand"
                      : "hover:border-ring",
                    !hotkey.pauseResume && "text-muted-foreground",
                  )}
                >
                  {capturingTarget === "pauseResume"
                    ? t("settings.hotkeyHint")
                    : hotkey.pauseResume || t("settings.hotkeyPauseResume")}
                </button>
                {hotkey.pauseResume && (
                  <Button
                    size="sm"
                    variant="ghost"
                    className="h-7 text-xs px-2"
                    onClick={() => void saveHotkey({ ...hotkey, pauseResume: "" })}
                  >
                    {t("settings.hotkeyClear")}
                  </Button>
                )}
              </div>
            </div>
          </CardContent>
        </Card>

        {/* 3. 模拟器与连接设置 */}
        <Card>
          <CardHeader className="py-3 px-4">
            <CardTitle className="text-sm font-semibold">{t("settings.simulator")}</CardTitle>
          </CardHeader>
          <CardContent className="flex flex-col gap-3.5 px-4 pb-4">
            <div className="flex items-center justify-between gap-4">
              <div>
                <Label className="text-xs font-medium">{t("settings.useSimulator")}</Label>
                <p className="text-[11px] text-muted-foreground">
                  {t("settings.useSimulatorDesc")}
                </p>
              </div>
              <Switch
                checked={sysSettings.simulator}
                onCheckedChange={(c) => void saveSysSettings({ simulator: c })}
              />
            </div>

            {sysSettings.simulator && (
              <>
                <Separator />
                <div className="flex items-center justify-between gap-4">
                  <div>
                    <Label className="text-xs font-medium">{t("settings.simulatorType")}</Label>
                    <p className="text-[11px] text-muted-foreground">
                      {t("settings.simulatorTypeDesc")}
                    </p>
                  </div>
                  <Select
                    value={String(sysSettings.simulator_type)}
                    onValueChange={(v) => void saveSysSettings({ simulator_type: Number(v) })}
                  >
                    <SelectTrigger className="h-8 w-44 text-xs">
                      <SelectValue />
                    </SelectTrigger>
                    <SelectContent>
                      <SelectItem value="0">{t("settings.mumu")}</SelectItem>
                      <SelectItem value="10">{t("settings.otherSimulator")}</SelectItem>
                    </SelectContent>
                  </Select>
                </div>

                <div className="flex items-center justify-between gap-4">
                  <div>
                    <Label className="text-xs font-medium">{t("settings.simulatorPort")}</Label>
                    <p className="text-[11px] text-muted-foreground">
                      {t("settings.simulatorPortDesc")}
                    </p>
                  </div>
                  <Input
                    type="number"
                    value={sysSettings.simulator_port}
                    onChange={(e) =>
                      void saveSysSettings({ simulator_port: Number(e.target.value) || 0 })
                    }
                    className="h-8 w-28 text-xs font-mono text-right"
                  />
                </div>

                {sysSettings.simulator_type === 0 && (
                  <div className="flex items-center justify-between gap-4">
                    <div>
                      <Label className="text-xs font-medium">{t("settings.emulatorTimeout")}</Label>
                      <p className="text-[11px] text-muted-foreground">
                        {t("settings.emulatorTimeoutDesc")}
                      </p>
                    </div>
                    <Input
                      type="number"
                      value={sysSettings.start_emulator_timeout}
                      onChange={(e) =>
                        void saveSysSettings({
                          start_emulator_timeout: Number(e.target.value) || 60,
                        })
                      }
                      className="h-8 w-28 text-xs font-mono text-right"
                    />
                  </div>
                )}
              </>
            )}
          </CardContent>
        </Card>

        {/* 4. 系统与防护设置 */}
        <Card>
          <CardHeader className="py-3 px-4">
            <CardTitle className="text-sm font-semibold">{t("settings.systemBehavior")}</CardTitle>
          </CardHeader>
          <CardContent className="flex flex-col gap-3.5 px-4 pb-4">
            <div className="flex items-center justify-between gap-4">
              <div>
                <Label className="text-xs font-medium">{t("settings.memoryProtection")}</Label>
                <p className="text-[11px] text-muted-foreground">
                  {t("settings.memoryProtectionDesc")}
                </p>
              </div>
              <Switch
                checked={sysSettings.memory_protection}
                onCheckedChange={(c) => void saveSysSettings({ memory_protection: c })}
              />
            </div>

            <Separator />

            <div className="flex items-center justify-between gap-4">
              <div>
                <Label className="text-xs font-medium">{t("settings.minimizeToTray")}</Label>
                <p className="text-[11px] text-muted-foreground">
                  {t("settings.minimizeToTrayDesc")}
                </p>
              </div>
              <Switch
                checked={sysSettings.minimize_to_tray}
                onCheckedChange={(c) => void saveSysSettings({ minimize_to_tray: c })}
              />
            </div>

            <Separator />

            <div className="flex items-center justify-between gap-4">
              <div>
                <Label className="text-xs font-medium">{t("settings.autostart")}</Label>
                <p className="text-[11px] text-muted-foreground">{t("settings.autostartDesc")}</p>
              </div>
              <Switch
                checked={sysSettings.autostart}
                onCheckedChange={(c) => void saveSysSettings({ autostart: c })}
              />
            </div>
          </CardContent>
        </Card>

        {/* 5. 实验性功能 */}
        <Card>
          <CardHeader className="py-3 px-4">
            <CardTitle className="text-sm font-semibold">{t("settings.experimental")}</CardTitle>
          </CardHeader>
          <CardContent className="flex flex-col gap-3.5 px-4 pb-4">
            <div className="flex items-center justify-between gap-4">
              <div>
                <Label className="text-xs font-medium">{t("settings.keepScreenAwake")}</Label>
                <p className="text-[11px] text-muted-foreground">
                  {t("settings.keepScreenAwakeDesc")}
                </p>
              </div>
              <Switch
                checked={sysSettings.experimental_keep_screen_awake}
                onCheckedChange={(c) => void saveSysSettings({ experimental_keep_screen_awake: c })}
              />
            </div>

            <Separator />

            <div className="flex items-center justify-between gap-4">
              <div>
                <Label className="text-xs font-medium">{t("settings.hdrWarning")}</Label>
                <p className="text-[11px] text-muted-foreground">{t("settings.hdrWarningDesc")}</p>
              </div>
              <Switch
                checked={sysSettings.experimental_hdr_warning}
                onCheckedChange={(c) => void saveSysSettings({ experimental_hdr_warning: c })}
              />
            </div>
          </CardContent>
        </Card>

        {/* 6. 更新源与 Mirror 酱 */}
        <Card>
          <CardHeader className="py-3 px-4">
            <CardTitle className="text-sm font-semibold">{t("settings.update")}</CardTitle>
          </CardHeader>
          <CardContent className="flex flex-col gap-3.5 px-4 pb-4">
            <div className="flex items-center justify-between gap-4">
              <div>
                <Label className="text-xs font-medium">{t("settings.updatePrerelease")}</Label>
                <p className="text-[11px] text-muted-foreground">
                  {t("settings.updatePrereleaseDesc")}
                </p>
              </div>
              <Switch
                checked={sysSettings.update_prerelease_enable}
                onCheckedChange={(c) => void saveSysSettings({ update_prerelease_enable: c })}
              />
            </div>

            <Separator />

            <div className="flex items-center justify-between gap-4">
              <div>
                <Label className="text-xs font-medium">{t("settings.updateSource")}</Label>
                <p className="text-[11px] text-muted-foreground">
                  {t("settings.updateSourceDesc")}
                </p>
              </div>
              <Select
                value={sysSettings.update_source}
                onValueChange={(v) =>
                  void saveSysSettings({ update_source: v as "GitHub" | "MirrorChyan" })
                }
              >
                <SelectTrigger className="h-8 w-36 text-xs">
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="GitHub">GitHub</SelectItem>
                  <SelectItem value="MirrorChyan">Mirror 酱</SelectItem>
                </SelectContent>
              </Select>
            </div>

            {sysSettings.update_source === "MirrorChyan" && (
              <div className="flex items-center justify-between gap-4">
                <div>
                  <Label className="text-xs font-medium">{t("settings.mirrorChyanCdk")}</Label>
                </div>
                <Input
                  value={sysSettings.mirrorchyan_cdk}
                  placeholder={t("settings.mirrorChyanCdkPlaceholder")}
                  onChange={(e) => void saveSysSettings({ mirrorchyan_cdk: e.target.value })}
                  className="h-8 w-60 text-xs font-mono"
                />
              </div>
            )}

            <div className="pt-1">
              <Button
                size="sm"
                variant="outline"
                className="h-8 text-xs gap-1.5"
                onClick={() => void checkUpdate()}
              >
                <SearchCheck className="size-3.5" /> {t("settings.checkUpdate")}
              </Button>
            </div>
          </CardContent>
        </Card>

        {/* 7. 关于 */}
        <Card>
          <CardHeader className="py-3 px-4">
            <CardTitle className="text-sm font-semibold">{t("settings.about")}</CardTitle>
          </CardHeader>
          <CardContent className="flex flex-col gap-3 px-4 pb-4">
            <div className="flex items-center justify-between">
              <Label className="text-xs font-medium">{t("settings.version")}</Label>
              <span className="font-mono text-xs text-muted-foreground">v{__APP_VERSION__}</span>
            </div>
            <Separator />
            <div className="flex items-center justify-between">
              <Label className="text-xs font-medium">{t("settings.repo")}</Label>
              <Button size="sm" variant="ghost" className="h-7 gap-1 text-xs" onClick={openRepo}>
                GitHub <ExternalLink className="size-3" />
              </Button>
            </div>
          </CardContent>
        </Card>

        <p className="text-center text-[11px] text-muted-foreground" data-testid="lang-indicator">
          {i18n.language} · Ahab Assistant Limbus Company v{__APP_VERSION__}
        </p>
      </div>
    </div>
  );
}
