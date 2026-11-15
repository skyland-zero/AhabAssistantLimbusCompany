// Setting rows: the label/control grid the console and the settings page use.
//
// Two shapes, as the GPUI page had: a line (a label and its control on one
// baseline) and a row (a label with a description, and a control that may be
// wider). Both keep the control in a fixed trailing lane so labels line up
// across a surface, and the label column may shrink while the control may not.
import type { JSX } from "nsh";
import { Text, View } from "nsh";

/** The vertical rhythm of one group's settings: 12pt between rows. Two columns
 *  keep the same row rhythm, with a wider column gap so neighbouring rows read
 *  apart; a list stays a column unless its caller asks for the split. */
export function SettingList(props: { columns?: 1 | 2; children?: JSX.Element }) {
  const layout = () =>
    props.columns === 2 ? "grid grid-cols-2 gap-x-5 gap-y-3" : "flex-col gap-3";
  return <View class={`w-full ${layout()}`}>{props.children}</View>;
}

/** A group of settings on its own tonal surface. It is a surface, not a second
 *  card: no border and no shadow, just the muted fill and a continuous radius.
 *  Two columns share one 1pt rule down the middle, inside a 20pt gutter, so a
 *  control at the end of one column is never read as the next column's label.
 *  A single column keeps the surface and drops the rule. */
export function SettingPanel(props: { columns?: 1 | 2; class?: string; children?: JSX.Element }) {
  return (
    <View class={`w-full rounded-(--radius-lg) bg-muted p-2 ${props.class ?? ""}`}>
      <View class="relative w-full">
        <View
          class="pointer-events-none absolute inset-y-0 left-1/2 w-px -translate-x-1/2 bg-input"
          classList={{ hidden: props.columns !== 2 }}
        />
        <SettingList columns={props.columns}>{props.children}</SettingList>
      </View>
    </View>
  );
}

/** A titled group of settings on the settings page. The group title sits
 *  outside the tonal surface; the surface itself gets a hairline because the
 *  page background and `muted` are nearly the same in the flat light skins
 *  (default, Bettbox), where a fill alone would disappear. */
export function SettingsGroup(props: {
  title?: string;
  /** A trailing detail of the group title: a status badge, a count. */
  aside?: JSX.Element;
  columns?: 1 | 2;
  children?: JSX.Element;
}) {
  const titled = () => props.title !== undefined || props.aside !== undefined;
  return (
    <View class="w-full flex-col gap-2">
      <If when={titled()}>
        <View class="w-full min-h-4 flex-row items-center gap-2">
          <Text class="min-w-0 flex-1 text-xs font-medium text-muted-foreground">
            {props.title ?? ""}
          </Text>
          {props.aside}
        </View>
      </If>
      <SettingPanel columns={props.columns} class="border border-input/80">
        {props.children}
      </SettingPanel>
    </View>
  );
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
