import type { AfterCompletionConfig, AfterPowerAction } from "@/services/ipc/types";

/** 格式化生成当前策略摘要文本 */
export function formatAfterCompletionSummary(
  config: AfterCompletionConfig,
  t: (key: string) => string,
): string {
  const parts: string[] = [];
  if (config.actions.includes("exit_game")) parts.push(t("afterCompletion.shortGame"));
  if (config.actions.includes("exit_emulator")) parts.push(t("afterCompletion.shortEmulator"));
  if (config.actions.includes("exit_aalc")) parts.push(t("afterCompletion.shortAalc"));

  const powerMap: Record<AfterPowerAction, string> = {
    none: "",
    sleep: t("afterCompletion.powerSleep"),
    hibernate: t("afterCompletion.powerHibernate"),
    lock: t("afterCompletion.powerLock"),
    shutdown: t("afterCompletion.powerShutdown"),
  };

  const powerText = powerMap[config.powerAction];
  const modeText = config.keepAfterCompletion
    ? t("afterCompletion.defaultMode")
    : t("afterCompletion.onceMode");

  if (parts.length === 0 && !powerText) {
    return `${t("afterCompletion.doNothing")} (${modeText})`;
  }

  let text = "";
  if (parts.length > 0) {
    text = `${t("afterCompletion.exitPrefix")}${parts.join("与")}`;
    if (powerText) text += `后${powerText}`;
  } else if (powerText) {
    text = powerText;
  }

  return `${text} (${modeText})`;
}
