// The pieces every team-editor section shares: the card surface, the dense
// option grid, a label/control row and the two whole-surface choices (a gift
// system and a sinner portrait).
import type { JSX } from "nsh";
import { Image, Text, View } from "nsh";
import { Card } from "../../../components/Card";
import { Pressable } from "../../../components/Button";
import { SINNER_NAMES } from "../../../teams";
import { SINNER_PORTRAITS, SYSTEM_ICONS } from "./assets";
import type { SystemName } from "../../../teams-mirror";

/** A section of the editor: the client's plain card with a muted title. */
export function EditorCard(props: {
  title?: string;
  /** A title whose text changes with state (a count); takes precedence. */
  titleBinding?: () => string;
  children?: JSX.Element;
  class?: string;
}) {
  return (
    <Card class={`w-full p-3 ${props.class ?? ""}`}>
      <View class={`flex-col ${props.title || props.titleBinding ? "gap-2" : ""}`}>
        <If when={props.titleBinding}>
          <Text class="text-xs font-medium text-muted-foreground">
            {props.titleBinding ? props.titleBinding() : ""}
          </Text>
        </If>
        <If when={!props.titleBinding && props.title}>
          <Text class="text-xs font-medium text-muted-foreground">{props.title ?? ""}</Text>
        </If>
        {props.children}
      </View>
    </Card>
  );
}

/** A two-column grid of dense options, each on its own faint surface. */
export function OptionGrid(props: { children?: JSX.Element }) {
  return <View class="grid w-full grid-cols-2 gap-2">{props.children}</View>;
}

/** One option in an `OptionGrid`. */
export function OptionCell(props: { children?: JSX.Element }) {
  return (
    <View class="min-w-0 rounded-(--radius) bg-secondary p-2">{props.children}</View>
  );
}

/** A label and the control that changes it. */
export function ControlRow(props: { label: string; control: JSX.Element }) {
  return (
    <View class="w-full flex-row items-center justify-between gap-3 py-1">
      <Text class="min-w-0 flex-1 truncate text-xs text-foreground">{props.label}</Text>
      {props.control}
    </View>
  );
}

/** A short explanation under a control or title. */
export function Hint(props: { children: string }) {
  return <Text class="text-[10px] leading-4 text-muted-foreground">{props.children}</Text>;
}

/** A section title inside a card, between groups of rows. */
export function SectionTitle(props: { children: string }) {
  return <Text class="text-xs font-medium text-muted-foreground">{props.children}</Text>;
}

/** One gift system: its plate, its name and the selected state. */
export function SystemChoice(props: {
  system: SystemName;
  label: string;
  selected: () => boolean;
  destructive?: boolean;
  onPress: () => void;
  testID?: string;
}) {
  const tone = () => {
    if (!props.selected()) return "border-border bg-card text-foreground hover:bg-accent";
    return props.destructive
      ? "border-danger bg-danger-light text-danger font-medium"
      : "border-brand bg-brand-light text-brand font-medium";
  };
  return (
    <Pressable
      accessibilityLabel={props.label}
      selected={props.selected()}
      onPress={props.onPress}
      testID={props.testID}
      class={`h-8.5 w-full min-w-0 flex-row items-center gap-2 rounded-(--radius) border px-2 transition-colors duration-150 ${tone()}`}
    >
      <Image
        src={SYSTEM_ICONS[props.system]}
        accessibilityElementsHidden
        style={{ width: 18, height: 18 }}
      />
      <Text class="min-w-0 truncate text-xs">{props.label}</Text>
    </Pressable>
  );
}

/** One sinner: the portrait and the slot number when the team carries them. */
export function SinnerCard(props: {
  sinner: string;
  slot: () => number | null;
  onPress: () => void;
}) {
  return (
    <Pressable
      accessibilityLabel={SINNER_NAMES[props.sinner] ?? props.sinner}
      selected={props.slot() !== null}
      onPress={props.onPress}
      testID={`team-sinner-${props.sinner}`}
      class={`h-31 w-full min-w-0 flex-col items-center justify-between rounded-(--radius-lg) border py-2 transition-colors duration-150 ${
        props.slot() !== null
          ? "border-brand bg-brand-light"
          : "border-border bg-secondary hover:bg-accent"
      }`}
    >
      <View class="relative size-14 flex-none">
        <Image
          src={SINNER_PORTRAITS[props.sinner]}
          accessibilityElementsHidden
          style={{ width: 56, height: 56 }}
        />
        <If when={props.slot() !== null}>
          <View class="absolute top-0 right-0 rounded-(--radius-sm) bg-brand px-1">
            <Text class="text-[10px] font-medium text-brand-foreground">#{props.slot()}</Text>
          </View>
        </If>
      </View>
      <View
        class={`h-5.5 w-full items-center justify-center rounded-(--radius-sm) px-1 ${
          props.slot() !== null ? "bg-brand-light" : "bg-card"
        }`}
      >
        <Text
          class={`text-[11px] font-medium ${
            props.slot() !== null ? "text-brand" : "text-foreground"
          }`}
        >
          {SINNER_NAMES[props.sinner] ?? props.sinner}
        </Text>
      </View>
    </Pressable>
  );
}
