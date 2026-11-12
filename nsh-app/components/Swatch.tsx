// A colour swatch: one choice of an accent, drawn as the accent's own colour.
//
// The colour is the *data* here (which accent this is), so it is passed in from
// the theme table rather than read from a class; the ring, the size and the
// pressed state still come from the design system.
import { Button } from "nsh/base";
import { View } from "nsh";

export function Swatch(props: {
  /** What screen readers call it: the accent's name. */
  label: string;
  /** The accent's brand colour in the current appearance. */
  color: string;
  selected: boolean;
  onPress: () => void;
}) {
  return (
    <Button
      accessibilityLabel={props.label}
      selected={props.selected}
      role="radio"
      onPress={props.onPress}
      class={`size-7 items-center justify-center rounded-full p-0 transition-transform duration-150 active:scale-[0.92] motion-reduce:transition-none focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-ring ${
        props.selected
          ? "outline-2 outline-offset-2 outline-foreground"
          : "opacity-80 hover:opacity-100"
      }`}
    >
      <View class="size-full rounded-full" style={{ backgroundColor: props.color }} />
    </Button>
  );
}
