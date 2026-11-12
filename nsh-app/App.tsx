// The P0 shell: window chrome, the sidebar and one placeholder page host.
// The real pages land one at a time in P3 (MIGRATION.md); until then this file
// only has to prove the Rust ↔ TSX seam and the theme tokens.
import { computed, Scroll, signal, Text, View } from "nsh";
import { Button } from "nsh/base";
import { Runtime } from "aalc/native";
import { TitleBar } from "./shell/TitleBar";

/** The app's pages, in sidebar order. `id` becomes the route name in P3. */
const PAGES = [
  { id: "home", label: "主控台" },
  { id: "teams", label: "队伍" },
  { id: "settings", label: "设置" },
  { id: "theme-packs", label: "主题包" },
  { id: "help", label: "帮助" },
] as const;

type PageId = (typeof PAGES)[number]["id"];

function App() {
  // Screen state: which page shows. Rust owns every value that must outlive
  // the screen; a page id is not one of them.
  const page = signal<PageId>("home");
  const current = computed(() => PAGES.find((entry) => entry.id === page()) ?? PAGES[0]);

  // The Rust seam: read once, since the binary's version cannot change.
  const version = Runtime.appVersion();
  const platform = Runtime.platformOs();

  return (
    <View class="flex-1 bg-background">
      <TitleBar detail={`v${version} · ${platform}`} />
      <View class="flex-1 flex-row">
        <View class="w-[200px] flex-col gap-1 border-r border-border bg-card p-2">
          <Each of={PAGES}>
            {(entry) => (
              <Button
                onPress={() => page.set(entry().id)}
                class="rounded-(--radius) px-3 py-2 active:bg-muted"
              >
                <Text class="text-foreground">{entry().label}</Text>
              </Button>
            )}
          </Each>
        </View>
        <Scroll class="flex-1">
          <View class="flex-col gap-3 p-6">
            <Text class="text-lg font-semibold text-foreground">{current().label}</Text>
            <Text class="text-muted-foreground">页面迁入进度</Text>
            <Text class="text-sm text-muted-foreground">
              P0 骨架已完成：窗口壳、侧栏、主题 token、Rust 导出通道。{current().label}
              页面在 P3 按 MIGRATION.md 逐页迁入。
            </Text>
            <View class="flex-col gap-1 rounded-(--radius) border border-border bg-card p-3">
              <Text class="text-sm text-muted-foreground">后端（Rust）</Text>
              <Text class="text-foreground">
                aalc-native v{version} · {platform}
              </Text>
            </View>
          </View>
        </Scroll>
      </View>
    </View>
  );
}

mount(App);
