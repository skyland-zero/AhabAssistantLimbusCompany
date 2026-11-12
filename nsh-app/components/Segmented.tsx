// A segmented choice: two to four mutually exclusive options in one silhouette.
//
// The container is the fill; the selected option is the raised one, so the
// control reads as one object rather than as a row of unrelated buttons. Each
// option is a `nsh/base` Button, which keeps Tab order, keyboard activation and
// the selected state for screen readers.
import type { JSX } from "nsh";
import { View } from "nsh";
import { Button } from "./Button";

export function Segmented<T extends string>(props: {
  options: { value: T; label: string }[];
  value: T;
  onChange: (value: T) => void;
  /** What screen readers call the group: the setting it chooses. */
  accessibilityLabel?: string;
  class?: string;
}) {
  return (
    <View
      role="radiogroup"
      accessibilityLabel={props.accessibilityLabel}
      class={`flex-row items-center gap-1 ${props.class ?? ""}`}
    >
      <Each of={props.options} key={(option) => option.value}>
        {(option) => (
          <Button
            label={option().label}
            variant={props.value === option().value ? "secondary" : "ghost"}
            selected={props.value === option().value}
            onPress={() => props.onChange(option().value)}
            class="px-3"
          />
        )}
      </Each>
    </View>
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
