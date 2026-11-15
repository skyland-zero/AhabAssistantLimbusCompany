// The settings page: a quick-navigation column (200pt) beside one scrolling
// list of sections. B drops the per-section cards: each section is a page-level
// heading and rule followed by FlClash-style tonal groups, so the page reads as
// one settings surface rather than eight stacked cards.
//
// The navigation highlights the section the reader is in: each section reports
// its offset inside the scroll content, and the scroll offset picks the last one
// that has started. Pressing an entry scrolls to it. This is screen state, so it
// lives here; every edited value lives in `settings.ts` (and, after P1, in Rust).
import type { JSX, ScrollRef } from "nsh";
import { Scroll, Text, View, effect, signal } from "nsh";
import { Runtime } from "aalc/native";
import { Pressable } from "../../components/Button";
import { Rule } from "../../components/Card";
import { Reveal } from "../../components/Reveal";
import { currentFeedback } from "../../settings";
import { AboutCard } from "./cards/About";
import { AppearanceCard } from "./cards/Appearance";
import { HotkeysCard } from "./cards/Hotkeys";
import { NotificationsCard } from "./cards/Notifications";
import { ResourcesCard } from "./cards/Resources";
import { ExperimentalCard, SystemCard } from "./cards/System";
import { UpdatesCard } from "./cards/Updates";

/** The sections, in the order they appear. */
const SECTIONS = [
  "外观",
  "全局热键",
  "系统与防护",
  "实验性功能",
  "更新与源配置",
  "资源同步",
  "任务通知",
  "关于",
] as const;

/** How far past a section's top edge still counts as "that section" (12pt). */
const SNAP = 12;

export function Settings() {
  let scroll: ScrollRef | undefined;
  const active = signal(0);
  // A programmatic jump keeps its section selected until the scroll settles:
  // the last sections cannot reach the top of the viewport, so a bottom-of-list
  // scroll position would otherwise select an earlier one right after the press.
  const jumping = signal(false);
  let jumpTimer: number | undefined;
  // Each section's offset inside the scroll content, filled as it lays out.
  const offsets = signal<Record<number, number>>({});

  effect(() => () => clearTimeout(jumpTimer));

  const record = (index: number, y: number) =>
    offsets.update((current) => ({ ...current, [index]: y }));

  const jumpTo = (index: number) => {
    jumping.set(true);
    active.set(index);
    scroll?.scrollTo({ y: Math.max(0, (offsets()[index] ?? 0) - SNAP), animated: true });
    clearTimeout(jumpTimer);
    jumpTimer = setTimeout(() => jumping.set(false), 800);
  };

  const stopJumping = () => {
    jumping.set(false);
    clearTimeout(jumpTimer);
  };

  const follow = (event: {
    offset: { y: number };
    size: { height: number };
    contentSize: { height: number };
  }) => {
    if (jumping()) return;
    // At the end of the page the last sections sit above the viewport top; keep
    // the last one selected instead of pinning the previous section forever.
    if (
      event.contentSize.height > event.size.height + 1 &&
      event.offset.y + event.size.height >= event.contentSize.height - 2
    ) {
      if (active() !== SECTIONS.length - 1) active.set(SECTIONS.length - 1);
      return;
    }
    const y = event.offset.y + SNAP;
    let next = 0;
    for (const [index, top] of Object.entries(offsets())) {
      if (top <= y) next = Math.max(next, Number(index));
    }
    if (next !== active()) active.set(next);
  };

  return (
    <View class="size-full min-h-0 min-w-0 flex-row gap-2 p-2.5">
      <SettingsNav active={active()} onJump={jumpTo} />
      <Scroll
        ref={scroll}
        class="min-h-0 min-w-0 flex-1"
        onScroll={(e) => follow(e)}
        onScrollDragStart={stopJumping}
      >
        <View class="flex-col gap-6 pb-6 pe-1">
          <Section index={0} onOffset={record}>
            <AppearanceCard />
          </Section>
          <Section index={1} onOffset={record}>
            <HotkeysCard />
          </Section>
          <Section index={2} onOffset={record}>
            <SystemCard />
          </Section>
          <Section index={3} onOffset={record}>
            <ExperimentalCard />
          </Section>
          <Section index={4} onOffset={record}>
            <UpdatesCard />
          </Section>
          <Section index={5} onOffset={record}>
            <ResourcesCard />
          </Section>
          <Section index={6} onOffset={record}>
            <NotificationsCard />
          </Section>
          <Section index={7} onOffset={record}>
            <AboutCard />
          </Section>
        </View>
      </Scroll>
    </View>
  );
}

/** A section and the offset it starts at, which the navigation reads. */
function Section(props: {
  index: number;
  onOffset: (index: number, y: number) => void;
  children?: JSX.Element;
}) {
  return (
    <View onLayout={({ bounds }) => props.onOffset(props.index, bounds.origin.y)}>
      <Reveal index={props.index}>{props.children}</Reveal>
    </View>
  );
}

/** The quick-navigation column: one plain row per section, the current one
 *  selected with the same soft brand surface the title bar uses. */
function SettingsNav(props: { active: number; onJump: (index: number) => void }) {
  return (
    <View class="h-full w-50 flex-none flex-col gap-1 pe-1">
      <View role="header" class="w-full flex-row items-center px-2">
        <Text class="text-sm font-semibold text-foreground">快速导航</Text>
      </View>
      <Rule class="mt-2" />
      <View class="w-full flex-col gap-0.5 pt-1">
        <Each of={SECTIONS}>
          {(label, index) => (
            <Pressable
              accessibilityLabel={label()}
              selected={props.active === index()}
              onPress={() => props.onJump(index())}
              class={`w-full min-h-7 flex-row items-center rounded-(--radius) px-3 text-xs font-medium transition-colors duration-150 active:scale-[0.97] motion-reduce:transition-none motion-reduce:active:scale-100 focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-ring ${
                props.active === index()
                  ? "bg-brand/15 text-brand dark:bg-brand-light"
                  : "text-muted-foreground hover:bg-accent hover:text-foreground active:bg-secondary"
              }`}
            >
              <Text>{label()}</Text>
            </Pressable>
          )}
        </Each>
      </View>
      <View class="flex-1" />
      <View class="w-full flex-col gap-1 px-2 pt-2">
        <Text class="text-[10px] text-muted-foreground">
          zh-CN · AALC.nsh v{Runtime.appVersion()}
        </Text>
        <If when={currentFeedback()}>
          {(line) => <Text class="text-[10px] text-success">{line()}</Text>}
        </If>
      </View>
    </View>
  );
}
