// The app's motion tokens for the native animator, in milliseconds, and the
// curves and distances that go with them.
//
// DESIGN.md names the same scale for classes (`duration-100` … `duration-300`);
// a motion or an animated value names these, never a number of its own, so the
// shell and the pages settle on one rhythm. The title bar's pill is the one
// place that travels far enough to take longer (`slide`).
import { Easing } from "nsh";
import type { MotionSpec } from "nsh";

/** Milliseconds, the classes' steps: `instant` 100, `fast` 150, `base` 200,
 *  `slow` 300, and the title bar's pill at 240. */
export const MS = {
  instant: 100,
  fast: 150,
  base: 200,
  slow: 300,
  slide: 240,
} as const;

/** Entering decelerates hard at the end, so a view settles instead of stopping.
 *  The same curve the shell's pill comes to rest on. */
export const ENTER_EASING = Easing.bezier(0.32, 0.72, 0, 1);

/** Between two places: nothing to settle into. */
export const MOVE_EASING = Easing.bezier(0.4, 0, 0.2, 1);

/** Leaving starts at once, so the next thing is not kept waiting. */
export const LEAVE_EASING = Easing.bezier(0.4, 0, 1, 1);

/** How far an arriving item rises, in points: a hint of direction, not a slide. */
export const RISE = 6;

/** A wave: the delay between two items, and how many take one. A long list
 *  starts its tail together rather than making the reader wait for it, and the
 *  whole wave (last delay + one entrance) stays inside the page fade — content
 *  that outlives the transition it belongs to reads as lag. */
export const WAVE = { step: 20, cap: 5 } as const;

/** The entrance of a listed item: it fades in and rises, a step after the one
 *  before it. `index` is its place in that list, capped so a wave never runs
 *  longer than `WAVE`. */
export function arriveSpec(index: number): MotionSpec {
  return {
    duration: MS.fast,
    easing: ENTER_EASING,
    delay: Math.min(Math.max(index, 0), WAVE.cap) * WAVE.step,
    keyframes: { opacity: [0, 1], translateY: [RISE, 0] },
  };
}
