// The execution status dot in the title bar's Console destination.
//
// It is its own view so a change of state repaints six pixels, not the shell.
// Colour is not the only cue: the label beside it (in the nav item) says what
// the state is, and the dot is hidden from screen readers.
import { View } from "nsh";
import type { HomeStatus } from "../store";

/** What the dot draws for each state: idle has no dot at all. */
const TONE: Record<HomeStatus, string> = {
  idle: "bg-transparent",
  running: "bg-success",
  paused: "bg-warning",
};

export function StatusDot(props: { status: HomeStatus }) {
  return <View accessibilityElementsHidden class={`size-1.5 flex-none rounded-full ${TONE[props.status]}`} />;
}
