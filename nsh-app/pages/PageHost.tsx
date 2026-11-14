// Routes the current page id to its page. Each page owns its own body, its
// dialogs and the work it starts; this file only picks one — and dissolves the
// new one in over the last one.
//
// The change is a cross-dissolve: the page that leaves fades out while the one
// that arrives fades in, both on the same curve and duration, so the frame the
// switch happens on is never a half-transparent page over another one's
// contents. A layer that only faded in on top of an opaque layer would show the
// old page's text through the new page's empty areas for the length of the
// fade. The arriving page also travels a few points toward the side its
// destination sits on in the bar; the leaving one stays put.
import { Animated, Easing, View, cleanup, createMotion, effect, signal } from "nsh";
import type { MotionSpec } from "nsh";
import { PAGES, currentPage } from "../store";
import type { PageId } from "../store";
import { Gallery } from "./Gallery";
import { Help } from "./help/Help";
import { Home } from "./home/Home";
import { Teams } from "./teams/Teams";
import { Settings } from "./settings/Settings";
import { Stats } from "./stats/Stats";
import { ThemePacks } from "./theme_packs/ThemePacks";
import { Toolbox } from "./toolbox/Toolbox";

/** The fade, in milliseconds: the arriving page fades in over the leaving one
 *  for exactly this long, both on the same curve. */
const ENTER_MS = 200;

/** A margin past the fade before the layer under it is dropped, so the layer is
 *  never removed on the frame it reaches zero. */
const HOLD_MS = 60;

/** How far the arriving page travels, in points. A hint of direction, not a
 *  slide: at this distance the change reads as a dissolve, and a 24-point
 *  travel reads as the layout shifting. */
const NUDGE = 6;

/** Strong deceleration (`cubic-bezier(0.16, 1, 0.3, 1)`): the curve NSH's own
 *  entrances use, and the one that makes an arrival settle instead of stop. */
const ENTER_EASING = Easing.bezier(0.16, 1, 0.3, 1);

/** The pages this file routes; `placed` guards against a page falling through
 *  unnoticed. */
function placed(page: PageId): boolean {
  return (
    page !== "home" &&
    page !== "settings" &&
    page !== "stats" &&
    page !== "toolbox" &&
    page !== "teams" &&
    page !== "help" &&
    page !== "theme-packs" &&
    page !== "gallery"
  );
}

/** Every page in the bar's order, the utility pages last: the direction of a
 *  page change is the direction it sits in this list. */
const ORDER: PageId[] = [
  ...PAGES.map((page) => page.id),
  "settings",
  "stats",
  "help",
  "gallery",
];

/** One page drawn by the host: its id, the side it came from, and whether it
 *  is the one on top (the only one that is read and pressed). */
type Layer = {
  /** Stable for the life of the layer: the switch never rebuilds the page
   *  that is leaving, so it keeps its state and its place in its own motion. */
  key: number;
  page: PageId;
  /** `-1` from the left, `1` from the right, `0` for the first layout. */
  dir: -1 | 0 | 1;
  top: boolean;
};

export function PageHost(props: { page: PageId }) {
  const layers = signal<Layer[]>([{ key: 0, page: props.page, dir: 0, top: true }]);
  let shown = props.page;
  let top: Layer = layers()[0];
  let seq = 1;

  effect(() => {
    const page = currentPage();
    if (page === shown) return;
    const from = ORDER.indexOf(shown);
    const to = ORDER.indexOf(page);
    const dir: -1 | 0 | 1 = to === from ? 0 : to > from ? 1 : -1;
    shown = page;
    const next: Layer = { key: seq++, page, dir, top: true };
    // The page that leaves is demoted, not unmounted: it fades out under the
    // arriving one, on the same curve, so the two are complements and the pair
    // always paints the full page — neither the backdrop nor the old text shows
    // through. At most two layers are ever up.
    layers.set([{ ...top, top: false }, next]);
    top = next;
    // A newer switch replaces the wait of the older one.
    const wait = setTimeout(() => layers.set([next]), ENTER_MS + HOLD_MS);
    cleanup(() => clearTimeout(wait));
  });

  return (
    <View class="relative min-h-0 flex-1">
      <Each of={layers()} key={(layer) => layer.key}>
        {(layer) => <PageLayer page={layer().page} dir={layer().dir} top={layer().top} />}
      </Each>
    </View>
  );
}

/** One layer of the host. It is created with its entrance already running, so
 *  its first frame is where the entrance starts: there is no still frame of the
 *  page at rest for the motion to pull back to its opening values. */
function PageLayer(props: { page: PageId; dir: -1 | 0 | 1; top: boolean }) {
  // `enter` starts the channels where the motion starts and plays it at once.
  // With reduced motion it lands where it ends without moving. The first layer
  // (dir 0, the app's own page on launch) has no entrance: a window that fades
  // its first page in on launch is a slower start, not a transition.
  const motion = createMotion(
    props.dir === 0 ? {} : { enter: enterMotion(props.dir) },
  );

  // A layer a newer page pushed down fades out on the same curve its successor
  // fades in on, so the two always add up to a full page: the old text can never
  // read through the new page's empty areas, and the backdrop never shows
  // either. `top` is stable for the life of a layer (a demoted one is dropped
  // right after), so this runs once.
  effect(() => {
    if (props.top) return;
    motion.opacity.set(1);
    Animated.timing(motion.opacity, {
      toValue: 0,
      duration: ENTER_MS,
      easing: ENTER_EASING,
    }).start();
  });

  return (
    <View
      class="absolute inset-0"
      style={{ ...motion.style, pointerEvents: props.top ? "auto" : "none" }}
      accessibilityElementsHidden={!props.top}
    >
      <If when={props.page === "settings"}>
        <Settings />
      </If>
      <If when={props.page === "stats"}>
        <Stats />
      </If>
      <If when={props.page === "toolbox"}>
        <Toolbox />
      </If>
      <If when={props.page === "home"}>
        <Home />
      </If>
      <If when={props.page === "teams"}>
        <Teams />
      </If>
      <If when={props.page === "help"}>
        <Help />
      </If>
      <If when={props.page === "theme-packs"}>
        <ThemePacks />
      </If>
      <If when={props.page === "gallery"}>
        <Gallery />
      </If>
    </View>
  );
}

/** The entrance: a fade plus the small travel toward its own side. */
function enterMotion(dir: -1 | 0 | 1): MotionSpec {
  return {
    duration: ENTER_MS,
    easing: ENTER_EASING,
    keyframes: { opacity: [0, 1], translateX: [dir * NUDGE, 0] },
  };
}
