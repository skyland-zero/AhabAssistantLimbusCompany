import { StrictMode } from "react";
import ReactDOM from "react-dom/client";
import App from "./App";
import "./index.css";
import { initI18n } from "@/i18n";

// 从持久化设置读取初始语言（zustand persist 存于 localStorage）
let initialLang: string | undefined;
try {
  const raw = localStorage.getItem("ahab-ui-settings");
  if (raw) initialLang = JSON.parse(raw)?.state?.language;
} catch {
  // ignore malformed storage
}
initI18n(initialLang);

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <StrictMode>
    <App />
  </StrictMode>,
);
