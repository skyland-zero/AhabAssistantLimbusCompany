// Setting rows: the label/control grid every settings card uses.
//
// Two shapes, as the GPUI page had: a line (a label and its control on one
// baseline) and a row (a label with a description, and a control that may be
// wider). Both keep the control in a fixed trailing lane so labels line up
// across a card, and the label column may shrink while the control may not.
import type { JSX } from "nsh";
import { Text, View } from "nsh";

/** The vertical rhythm of one card's settings: 12pt between rows. */
export function SettingList(props: { children?: JSX.Element }) {
  return <View class="w-full flex-col gap-3">{props.children}</View>;
}

/** A label and its control on one line. */
export function SettingLine(props: { label: string; children?: JSX.Element }) {
  return (
    <View class="w-full flex-row items-center justify-between gap-4">
      <View class="min-w-0 flex-1">
        <Text class="text-xs font-medium text-foreground">{props.label}</Text>
      </View>
      <View class="flex-none">{props.children}</View>
    </View>
  );
}

/** A label with a description, and its control. */
export function SettingRow(props: {
  label: string;
  /** One line explaining what the setting does; left out when the label says it. */
  detail?: string;
  children?: JSX.Element;
}) {
  return (
    <View class="w-full flex-row items-center justify-between gap-4">
      <View class="min-w-0 flex-1 flex-col gap-1">
        <Text class="text-xs font-medium text-foreground">{props.label}</Text>
        <If when={props.detail}>
          {(detail) => <Text class="text-xs text-muted-foreground">{detail()}</Text>}
        </If>
      </View>
      <View class="flex-none">{props.children}</View>
    </View>
  );
}
