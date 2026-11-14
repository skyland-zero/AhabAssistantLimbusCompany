// A segmented choice: two to four mutually exclusive options in one silhouette.
//
// The container is the fill; the selected option is the raised one, so the
// control reads as one object rather than as a row of unrelated buttons. The
// raised one is a single marker that slides between the options (`Tabs`), not
// a background per option switching on and off.
import type { JSX } from "nsh";
import { View } from "nsh";
import { Tabs } from "./Tabs";

export function Segmented<T extends string>(props: {
  options: { value: T; label: string }[];
  /** The chosen value, read as a signal so the marker follows it. */
  value: () => T;
  onChange: (value: T) => void;
  /** What screen readers call the group: the setting it chooses. */
  accessibilityLabel?: string;
  class?: string;
}) {
  return (
    <Tabs<T>
      items={props.options}
      value={props.value}
      onChange={props.onChange}
      accessibilityLabel={props.accessibilityLabel}
      trackClass={`gap-1 ${props.class ?? ""}`}
      itemClass="min-h-5 px-3"
      pillClass="bg-secondary"
    />
  );
}

/** A row of values that is not a radio group: the accent swatches, where each
 *  option is a colour and the radio semantics come from the wrapper. */
export function SwatchRow(props: { children?: JSX.Element; label?: string }) {
  return (
    <View role="radiogroup" accessibilityLabel={props.label} class="flex-row items-center gap-2">
      {props.children}
    </View>
  );
}
