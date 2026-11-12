// The app's shared screen state: which page shows, and the appearance the user
// picked. Rust owns every value that must outlive the screen (execution state,
// teams, settings — P1 in MIGRATION.md); a route id and the appearance switch
// are screen-level, so they live here.
//
// Signals stay private: pages call the functions, they never write the signal.
import { setColorScheme, signal } from "nsh";
import { DECOR_OF, SKIN_DEFAULT_ACCENT, applyTheme } from "./themes";
import type { AccentName, Decor, SkinName } from "./themes";

/** A destination in the title bar, in the order the shell draws them. */
export const PAGES = [
  { id: "home", label: "主控台", icon: "home" },
  { id: "teams", label: "队伍管理", icon: "users" },
  { id: "theme-packs", label: "镜牢主题包", icon: "palette" },
  { id: "toolbox", label: "工具箱", icon: "wrench" },
  { id: "resources", label: "资源中心", icon: "package" },
  { id: "help", label: "帮助", icon: "help" },
] as const;

/** Every page the app shows: the six title-bar destinations, Settings (opened
 *  from the title bar's utility button) and the P2 component gallery. */
export type PageId = (typeof PAGES)[number]["id"] | "settings" | "gallery";

const page = signal<PageId>("home");

/** The page on screen (tracked where it is read). */
export function currentPage(): PageId {
  return page();
}

/** Shows `next`. */
export function navigate(next: PageId): void {
  page.set(next);
}

/** Whether `id` is the current page (tracked where it is read). */
export function isCurrent(id: PageId): boolean {
  return page() === id;
}

/** The page's title-bar label, or one of the two pages that are not in it. */
export function pageLabel(id: PageId): string {
  return PAGES.find((entry) => entry.id === id)?.label ?? (id === "settings" ? "设置" : "组件对照");
}

/** What the title bar's status dot shows: the console's execution state. */
export type HomeStatus = "idle" | "running" | "paused";

// P1 (MIGRATION.md) makes Rust the owner: this becomes a mirror fed by the
// execution state's `onChange`, and nothing in TSX writes it directly.
const homeStatus = signal<HomeStatus>("idle");

/** The console's execution state (tracked where it is read). */
export function currentHomeStatus(): HomeStatus {
  return homeStatus();
}

/** The visual skin (surface, geometry, decoration), and the accent over it. */
const skin = signal<SkinName>("default");
const accent = signal<AccentName>(SKIN_DEFAULT_ACCENT.default);

/** The skin in effect (tracked where it is read). */
export function currentSkin(): SkinName {
  return skin();
}

/** The accent in effect (tracked where it is read). */
export function currentAccent(): AccentName {
  return accent();
}

/** The active decoration language: what `components/` branches on. */
export function currentDecor(): Decor {
  return DECOR_OF[skin()];
}

/** Applies the skin and accent the app starts with. `theme.json` bootstraps
 *  only the default skin, so the run starts from the composed theme instead of
 *  whatever `nsh.json` declared. Call once, when the root is created. */
export function initTheme(): void {
  applyTheme(skin(), accent());
}

/** Switches the skin. The accent follows only while the user has not picked
 *  one: each skin ships with an accent it is designed around (Limbus is brass),
 *  and leaving crimson over limbus would paint its frame in the wrong ink. */
export function setSkin(next: SkinName): void {
  if (accent() === SKIN_DEFAULT_ACCENT[skin()]) accent.set(SKIN_DEFAULT_ACCENT[next]);
  skin.set(next);
  applyTheme(skin(), accent());
}

/** Switches the accent, leaving the skin alone. */
export function setAccent(next: AccentName): void {
  accent.set(next);
  applyTheme(skin(), accent());
}

/** Light, dark, or the system's own appearance. */export type Appearance = "light" | "dark" | "system";

const appearance = signal<Appearance>("system");

/** The appearance in effect (tracked where it is read). */
export function currentAppearance(): Appearance {
  return appearance();
}

/** Applies `next` at once, and remembers it. */
export function applyAppearance(next: Appearance): void {
  appearance.set(next);
  setColorScheme(next);
}

/** Light → Dark → System → Light, as the title bar's toggle cycles it. */
export function cycleAppearance(): Appearance {
  const next = { light: "dark", dark: "system", system: "light" } as const;
  applyAppearance(next[appearance()]);
  return appearance();
}
