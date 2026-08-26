/** Tauri 环境检测（浏览器 dev 模式下为 false） */
export function isTauri(): boolean {
  return "__TAURI_INTERNALS__" in window;
}
