// A row that arrives with its siblings: it fades in and rises a few points, a
// step after the one before it, once, when the page is entered.
//
// For lists whose rows are not cards (a column of buttons); a card takes
// `arrive` itself, so its own frame moves and the layout keeps its stretches.
import type { JSX } from "nsh";
import { View, createMotion } from "nsh";
import { arriveSpec } from "../motion";

export function Reveal(props: {
  /** Its place in the list: the first one leads, the rest follow. */
  index?: number;
  class?: string;
  children?: JSX.Element;
}) {
  const motion = createMotion({ enter: arriveSpec(props.index ?? 0) });
  return (
    <View class={props.class ?? ""} style={motion.style}>
      {props.children}
    </View>
  );
}
