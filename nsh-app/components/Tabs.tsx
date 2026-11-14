// A segmented choice: the macOS control.
//
// One selection pill is drawn behind the options and slides under the chosen
// one — its left edge and its width both animate, so a switch reads as the
// marker moving rather than as two backgrounds swapping on and off. Each
// option is a button (presses, keys and the selected state for screen readers);
// the pill is drawn once, by the row, and never per option.
//
// The chosen value is a signal getter (`value: () => range()`), not a plain
// value: the pill follows it from an `effect`, which only re-runs for what it
// reads.
import type { JSX } from "nsh";
import {
  Animated,
  Text,
  View,
  createAnimatedValue,
  effect,
  reduceMotion,
  signal,
} from "nsh";
import { ENTER_EASING, MS } from "../motion";
import { Pressable } from "./Button";

type Item<T extends string> = { value: T; label: string };

export function Tabs<T extends string>(props: {
  items: Item<T>[];
  /** The chosen value, read as a signal so the pill follows it. */
  value: () => T;
  onChange: (value: T) => void;
  /** What screen readers call the group: the setting it chooses. */
  accessibilityLabel?: string;
  /** The row's frame: the wash behind the options, the padding, the radius. */
  trackClass?: string;
  /** One option's frame: its height and inset. */
  itemClass?: string;
  /** The moving marker's look. */
  pillClass?: string;
  /** Content after an option's label: its count. */
  suffix?: (item: Item<T>) => JSX.Element;
  /** A stable test id per option (`team-filter-mirror`). */
  itemTestID?: (item: Item<T>) => string;
}) {
  const frames = signal<Record<string, { x: number; y: number; w: number; h: number }>>({});
  const marker = signal({ y: 0, w: 0, h: 0 });
  const origin = createAnimatedValue(0);
  const stretch = createAnimatedValue(1);
  let previous: { x: number; w: number } | null = null;
  let width = 0;

  /** The marker's width, set before the native values move onto it (setting it
   *  after would paint one frame of the new width with the old scale). */
  const place = (next: { y: number; w: number; h: number }) => {
    const same = Math.abs(width - next.w) < 0.5;
    if (same && marker().y === next.y && marker().h === next.h) return;
    width = next.w;
    marker.set(next);
  };

  const report = (
    id: string,
    bounds: { origin: { x: number; y: number }; size: { width: number; height: number } },
  ) => {
    const next = {
      x: bounds.origin.x,
      y: bounds.origin.y,
      w: bounds.size.width,
      h: bounds.size.height,
    };
    const current = frames()[id];
    if (
      current &&
      Math.abs(current.x - next.x) < 0.5 &&
      Math.abs(current.y - next.y) < 0.5 &&
      Math.abs(current.w - next.w) < 0.5 &&
      Math.abs(current.h - next.h) < 0.5
    ) {
      return;
    }
    frames.set({ ...frames(), [id]: next });
  };

  effect(() => {
    const target = frames()[props.value()];
    if (!target || target.w <= 0) return;
    const next = { y: target.y, w: target.w, h: target.h };
    if (!previous) {
      // The first layout: stand the marker where the chosen option is.
      previous = target;
      origin.set(target.x);
      stretch.set(1);
      place(next);
      return;
    }
    const moved =
      Math.abs(previous.x - target.x) >= 0.5 || Math.abs(previous.w - target.w) >= 0.5;
    const from = previous;
    previous = target;
    if (!moved) {
      place(next);
      return;
    }
    origin.set(from.x);
    stretch.set(from.w / target.w);
    place(next);
    if (reduceMotion()) {
      origin.set(target.x);
      stretch.set(1);
      return;
    }
    Animated.parallel([
      Animated.timing(origin, {
        toValue: target.x,
        duration: MS.slide,
        easing: ENTER_EASING,
      }),
      Animated.timing(stretch, {
        toValue: 1,
        duration: MS.slide,
        easing: ENTER_EASING,
      }),
    ]).start();
  });

  return (
    <View
      role="radiogroup"
      accessibilityLabel={props.accessibilityLabel}
      class={`relative flex-row items-center ${props.trackClass ?? "gap-1"}`}
    >
      {/* Behind the options, which paint over it: a moving background, not a
          wrapper that clips them. */}
      <View
        class={`pointer-events-none absolute rounded-(--radius) ${props.pillClass ?? "bg-secondary"}`}
        style={{
          top: marker().y,
          height: marker().h,
          width: marker().w,
          transformOrigin: "left center",
          transform: [{ translateX: origin }, { scaleX: stretch }],
        }}
      />
      <Each of={props.items} key={(item) => item.value}>
        {(item) => (
          <View class="flex-none" onLayout={({ bounds }) => report(item().value, bounds)}>
            <Pressable
              accessibilityLabel={item().label}
              selected={props.value() === item().value}
              testID={props.itemTestID?.(item())}
              onPress={() => props.onChange(item().value)}
              class={
                "flex-row items-center gap-1.5 rounded-(--radius) transition-colors duration-150 " +
                "hover:bg-accent active:scale-[0.97] " +
                "motion-reduce:transition-none motion-reduce:active:scale-100 " +
                (props.value() === item().value ? "text-foreground" : "text-muted-foreground") +
                " " +
                (props.itemClass ?? "min-h-5 px-3")
              }
            >
              <Text class="text-xs font-medium">{item().label}</Text>
              <If when={props.suffix?.(item())}>{(content) => content()}</If>
            </Pressable>
          </View>
        )}
      </Each>
    </View>
  );
}
