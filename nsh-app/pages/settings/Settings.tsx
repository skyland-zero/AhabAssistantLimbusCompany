// The settings page: a quick-navigation column (200pt) beside one scrolling
// stack of cards, exactly as the GPUI client laid it out.
//
// The navigation highlights the section the reader is in: each card reports its
// offset inside the scroll content, and the scroll offset picks the last one
// that has started. Pressing an entry scrolls to it. This is screen state, so it
// lives here; every edited value lives in `settings.ts` (and, after P1, in Rust).
import type { JSX, ScrollRef } from "nsh";
import { Scroll, Text, View, signal } from "nsh";
import { Runtime } from "aalc/native";
import { Card, CardBody, Divider } from "../../components/Card";
import { Button } from "../../components/Button";
import { Reveal } from "../../components/Reveal";
import { currentFeedback } from "../../settings";
import { AboutCard } from "./cards/About";
import { AppearanceCard } from "./cards/Appearance";
import { HotkeysCard } from "./cards/Hotkeys";
import { NotificationsCard } from "./cards/Notifications";
import { ResourcesCard } from "./cards/Resources";
import { ExperimentalCard, SystemCard } from "./cards/System";
import { UpdatesCard } from "./cards/Updates";

/** The sections, in the order the cards appear (`SETTINGS_SECTIONS`). */
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

/** How far past a card's top edge still counts as "that section" (12pt). */
const SNAP = 12;

export function Settings() {
  let scroll: ScrollRef | undefined;
  const active = signal(0);
  // Each card's offset inside the scroll content, filled as it lays out.
  const offsets = signal<Record<number, number>>({});

  const record = (index: number, y: number) =>
    offsets.update((current) => ({ ...current, [index]: y }));

  const jumpTo = (index: number) => {
    active.set(index);
    scroll?.scrollTo({ y: Math.max(0, (offsets()[index] ?? 0) - SNAP), animated: true });
  };

  const follow = (event: { offset: { y: number } }) => {
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
      <Scroll ref={scroll} class="min-h-0 min-w-0 flex-1" onScroll={(e) => follow(e)}>
        <View class="flex-col gap-3 pb-3">
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
          <Divider />
          <Text class="text-center text-xs text-muted-foreground">zh-CN · AALC.nsh v{Runtime.appVersion()}</Text>
          <If when={currentFeedback()}>
            {(line) => <Text class="text-center text-xs text-success">{line()}</Text>}
          </If>
        </View>
      </Scroll>
    </View>
  );
}

/** A card and the offset it starts at, which the navigation reads. */
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

/** The quick-navigation column: one entry per card, the current one selected. */
function SettingsNav(props: { active: number; onJump: (index: number) => void }) {
  return (
    <Card title="快速导航" class="h-full w-50 flex-none">
      <CardBody class="gap-1">
        <Each of={SECTIONS}>
          {(label, index) => (
            <Button
              label={label()}
              variant={props.active === index() ? "secondary" : "ghost"}
              selected={props.active === index()}
              onPress={() => props.onJump(index())}
              class="w-full"
            />
          )}
        </Each>
      </CardBody>
    </Card>
  );
}
