// The running task card's border breath: the card's own frame line lights up
// and dims, then rests. It is a sibling of the card, so its one-pixel border
// paints over the selected border instead of under the card's edge.
//
// The opacity is sampled at 15 fps in JS rather than run as a 60 fps native
// animation: on a 1px line this slow the step is invisible, and the renderer
// draws a quarter of the frames. The timer stops when the window is not in use
// or the system asks for less motion.
import {
  View,
  colorScheme,
  effect,
  lifecycle,
  reduceMotion,
  signal,
  token,
} from "nsh";
import { BREATH } from "../motion";

/** The ring's dim and bright ends. */
const REST = 0.15;
const PEAK = 1;
const CYCLE = BREATH.in + BREATH.out + BREATH.rest;

/** A soft S-curve: no velocity at either end, like a breath. */
function ease(t: number): number {
  return 0.5 - 0.5 * Math.cos(Math.PI * Math.min(Math.max(t, 0), 1));
}

/** The ring's opacity at `elapsed` in the cycle: in, out, then the pause. */
function level(elapsed: number): number {
  if (elapsed < BREATH.in) {
    return REST + (PEAK - REST) * ease(elapsed / BREATH.in);
  }
  if (elapsed < BREATH.in + BREATH.out) {
    return PEAK - (PEAK - REST) * ease((elapsed - BREATH.in) / BREATH.out);
  }
  return REST;
}

export function Breath() {
  const glow = signal(REST);
  const calm = signal(reduceMotion());

  effect(() => {
    const still = reduceMotion();
    calm.set(still);
    if (still || lifecycle() !== "active") {
      glow.set(REST);
      return;
    }
    const start = Date.now();
    let last = REST;
    glow.set(REST);
    const tick = () => {
      const next = level((Date.now() - start) % CYCLE);
      if (next !== last) {
        last = next;
        glow.set(next);
      }
    };
    const timer = setInterval(tick, BREATH.frame);
    return () => clearInterval(timer);
  });

  // The frame line's light end: the pale brand on light skins, near-white on
  // dark ones. The selected border stays underneath at full strength, so the
  // breath reads as the outline lighting up rather than as a second ring.
  const peak = () =>
    colorScheme() === "dark" ? token("foreground") : token("brand-light");

  return (
    <If when={!calm()}>
      <View
        testID="task-running-breath"
        class="pointer-events-none absolute inset-0 rounded-(--radius-xl) border"
        style={{
          borderColor: peak(),
          opacity: glow(),
        }}
        accessibilityElementsHidden
      />
    </If>
  );
}
