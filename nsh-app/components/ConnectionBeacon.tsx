// The automation-core initialization beacon: a rotating arc, two expanding
// radar rings and a pulsing core. It says "initializing" by motion instead of a
// status word, so the core state cannot be confused with a task that is running.
import { Animated, View, createAnimatedValue, effect, reduceMotion, signal, token } from "nsh";
import { BEACON } from "../motion";
import { Icon } from "./Icon";

/** The beacon's footprint in points. */
const SIZE = 16;

export function ConnectionBeacon(props: { size?: number }) {
  const size = props.size ?? SIZE;
  const spin = createAnimatedValue(0);
  const phase = createAnimatedValue(0);
  // Two rings share one phase, half a cycle apart. The second one wraps with
  // `mod(1)` so both keep looping from the same value.
  const ringA = phase.interpolate({ inputRange: [0, 1], outputRange: [0.45, 1.4] });
  const ringB = phase
    .add(0.5)
    .mod(1)
    .interpolate({ inputRange: [0, 1], outputRange: [0.45, 1.4] });
  const fadeA = phase.interpolate({
    inputRange: [0, 0.15, 1],
    outputRange: [0, 0.55, 0],
  });
  const fadeB = phase
    .add(0.5)
    .mod(1)
    .interpolate({
      inputRange: [0, 0.15, 1],
      outputRange: [0, 0.55, 0],
    });
  const core = phase.interpolate({
    inputRange: [0, 0.5, 1],
    outputRange: [0.9, 1.18, 0.9],
  });
  // A signal, rather than a body snapshot, so a runtime "reduce motion" switch
  // reaches the JSX bindings below.
  const calm = signal(reduceMotion());

  effect(() => {
    const still = reduceMotion();
    calm.set(still);
    if (still) {
      spin.set(0);
      phase.set(0);
      return;
    }
    const orbit = Animated.loop(
      Animated.timing(spin, {
        toValue: 360,
        duration: BEACON.spin,
        easing: "linear",
      }),
    );
    const ping = Animated.loop(
      Animated.timing(phase, {
        toValue: 1,
        duration: BEACON.ping,
        easing: "linear",
      }),
    );
    orbit.start();
    ping.start();
    return () => {
      orbit.stop();
      ping.stop();
    };
  });

  return (
    <View
      class="relative flex-none items-center justify-center"
      style={{ width: size, height: size }}
      accessibilityLabel="自动化核心初始化中"
    >
      <If when={!calm()}>
        <View class="absolute inset-0">
          <View
            class="absolute rounded-full border border-brand"
            style={{
              width: size,
              height: size,
              opacity: fadeA,
              transform: [{ scale: ringA }],
            }}
          />
          <View
            class="absolute rounded-full border border-brand"
            style={{
              width: size,
              height: size,
              opacity: fadeB,
              transform: [{ scale: ringB }],
            }}
          />
        </View>
      </If>
      <View class="absolute" style={{ transform: [{ rotate: spin }] }}>
        <Icon name="loader" size={size} color={token("brand")} />
      </View>
      <View
        class="absolute rounded-full bg-brand"
        style={{
          width: Math.max(2, Math.round(size * 0.2)),
          height: Math.max(2, Math.round(size * 0.2)),
          opacity: calm() ? 0.8 : 1,
          transform: [{ scale: calm() ? 1 : core }],
        }}
      />
    </View>
  );
}
