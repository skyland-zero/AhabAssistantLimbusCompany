import i18n from "i18next";
import { initReactI18next } from "react-i18next";
import enUS from "./locales/en-US";
import zhCN from "./locales/zh-CN";

/** 初始化 i18n；语言由 appStore 持久化并在 App 启动时同步 */
export function initI18n(initialLang?: string): void {
  void i18n.use(initReactI18next).init({
    resources: {
      "zh-CN": { translation: zhCN },
      "en-US": { translation: enUS },
    },
    lng: initialLang ?? "zh-CN",
    fallbackLng: "zh-CN",
    interpolation: { escapeValue: false, prefix: "{", suffix: "}" },
  });
}

export function changeLanguage(lang: string): void {
  void i18n.changeLanguage(lang);
}
