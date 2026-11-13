// The app's shared screen state: which page shows, and the appearance the user
// picked. Rust owns every value that must outlive the screen (execution state,
// teams, settings — P1 in MIGRATION.md); a route id and the appearance switch
// are screen-level, so they live here.
//
// Signals stay private: pages call the functions, they never write the signal.
import { setColorScheme, signal } from "nsh";
import { AppStore, I18n } from "aalc/native";
import { ACCENT_LABELS, DECOR_OF, SKIN_DEFAULT_ACCENT, applyTheme } from "./themes";
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

/** The persisted app-shell settings (`settings.json`). */
const appSettings = new AppStore();

/** The interface language, persisted with the rest of the shell settings. */
export type Language = "zhCn" | "enUs";

const language = signal<Language>(appSettings.language() as Language);

/** The interface language (tracked where it is read). */
export function currentLanguage(): Language {
  return language();
}

/** Chooses the interface language and persists it. */
export function setLanguage(next: Language): void {
  language.set(next);
  appSettings.setLanguage(next);
}

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

/** The catalog key for each title-bar destination. */
const PAGE_KEY: Record<string, string> = {
  home: "navHome",
  teams: "navTeams",
  "theme-packs": "navThemes",
  toolbox: "navToolbox",
  resources: "navResources",
  help: "navHelp",
  settings: "navSettings",
};

/** The page's title-bar label, or the gallery's development-only name. */
export function pageLabel(id: PageId): string {
  if (id === "gallery") return "组件对照";
  return I18n.text(language(), PAGE_KEY[id] ?? "navHome");
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

/** Mirrors the Rust execution state into the title bar's status dot. */
export function mirrorHomeStatus(next: string): void {
  homeStatus.set(next === "paused" ? "paused" : next === "running" || next === "starting" ? "running" : "idle");
}

/** The visual skin (surface, geometry, decoration), and the accent over it.
 *  Rust owns the stored pair; these fallbacks only cover a file whose ids the
 *  theme table does not know (an older or hand-edited value). */
const FALLBACK_SKIN: SkinName = "bettbox";
const storedSkin = appSettings.skin() as SkinName;
const initialSkin: SkinName = storedSkin in SKIN_DEFAULT_ACCENT ? storedSkin : FALLBACK_SKIN;
const storedAccent = appSettings.accent() as AccentName;
const skin = signal<SkinName>(initialSkin);
const accent = signal<AccentName>(
  storedAccent in ACCENT_LABELS ? storedAccent : SKIN_DEFAULT_ACCENT[initialSkin],
);
/** The colour theme's seed, or `""` for "follow the skin's accent". */
const seed = signal<string>(appSettings.themeSeed());

/** The skin in effect (tracked where it is read). */
export function currentSkin(): SkinName {
  return skin();
}

/** The accent in effect (tracked where it is read). */
export function currentAccent(): AccentName {
  return accent();
}

/** The colour theme in effect (`#RRGGBB`), or `null` for the skin's own. */
export function currentSeed(): string | null {
  return seed() || null;
}

/** The active decoration language: what `components/` branches on. */
export function currentDecor(): Decor {
  return DECOR_OF[skin()];
}

/** Applies the skin, accent and appearance the app starts with. `theme.json`
 *  bootstraps Bettbox alone, so the run starts from the composed theme instead
 *  of whatever `nsh.json` declared; the stored appearance has to be pushed the
 *  same way, or a user who picked dark gets the system's choice until the next
 *  toggle. Call once, when the root is created. */
export function initTheme(): void {
  applyTheme(skin(), accent(), currentSeed());
  // A stored light/dark wins over the platform's appearance; `system` leaves it
  // to the platform, and to a host that opened the window in its own scheme.
  if (appearance() !== "system") setColorScheme(appearance());
}

/** Switches the skin. The accent follows only while the user has not picked
 *  one: each skin ships with an accent it is designed around (Limbus is brass),
 *  and leaving crimson over limbus would paint its frame in the wrong ink. */
export function setSkin(next: SkinName): void {
  if (accent() === SKIN_DEFAULT_ACCENT[skin()]) accent.set(SKIN_DEFAULT_ACCENT[next]);
  skin.set(next);
  applyTheme(skin(), accent(), currentSeed());
  appSettings.setSkin(skin(), accent());
}

/** Switches the accent, leaving the skin alone. */
export function setAccent(next: AccentName): void {
  accent.set(next);
  applyTheme(skin(), accent(), currentSeed());
  appSettings.setSkin(skin(), accent());
}

/** Switches the colour theme; `null` follows the skin's accent again. The theme
 *  outranks the accent, so the swatches below it stop changing the look until it
 *  is cleared. */
export function setThemeSeed(next: string | null): void {
  seed.set(next ?? "");
  applyTheme(skin(), accent(), currentSeed());
  appSettings.setThemeSeed(next ?? "");
}

/** Light, dark, or the system's own appearance. */export type Appearance = "light" | "dark" | "system";

const appearance = signal<Appearance>(appSettings.appearance() as Appearance);

/** The appearance in effect (tracked where it is read). */
export function currentAppearance(): Appearance {
  return appearance();
}

/** Applies `next` at once, and remembers it. */
export function applyAppearance(next: Appearance): void {
  appearance.set(next);
  setColorScheme(next);
  appSettings.setAppearance(next);
}

/** Light → Dark → System → Light, as the title bar's toggle cycles it. */
export function cycleAppearance(): Appearance {
  const next = { light: "dark", dark: "system", system: "light" } as const;
  applyAppearance(next[appearance()]);
  return appearance();
}
