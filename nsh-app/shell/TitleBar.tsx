// The app-drawn window chrome: the brand, the six destinations, the execution
// status dot, the appearance and settings utilities, and (outside macOS) the
// window buttons.
//
// `nsh.json` asks for a hidden title bar, so the app paints it and keeps the
// platform's window buttons where the platform has them. The drag regions are
// the brand block and the flexible spacer only, so a press on a destination is
// a press, not a window drag.
import { Animated, Easing, Image, Text, View, createAnimatedValue, effect, platform, reduceMotion, signal, token } from "nsh";
import { isAvailable as hasDesktop, titleBar } from "nsh/desktop";
import banner from "../assets/limbus_title_banner.png";
import { Button, IconButton } from "../components/Button";
import { Icon } from "../components/Icon";
import type { IconName } from "../components/Icon";
import { PAGES, currentAppearance, currentDecor, currentHomeStatus, currentPage, cycleAppearance, navigate, pageLabel } from "../store";
import { t } from "../settings";
import type { Appearance, HomeStatus, PageId } from "../store";
import type { Decor } from "../themes";
import { StatusDot } from "./StatusDot";

/** The bar's area: the client's fixed 40pt bar. The platform still supplies
 *  the left/right insets for its own window buttons (traffic lights). */
const TITLEBAR_HEIGHT = 40;

/** The pill's slide: one duration and one curve for both channels, so the edge
 *  and the width settle together. The curve decelerates hard at the end
 *  (`cubic-bezier(0.32, 0.72, 0, 1)`), which is what makes the pill come to
 *  rest instead of stopping. */
const PILL_MS = 240;
const PILL_EASING = Easing.bezier(0.32, 0.72, 0, 1);

/** The pill's fade when a page outside the bar (Settings, Stats, Help) is up. */
const PILL_FADE_MS = 160;

/** macOS keeps the system's traffic lights on the left, so the app draws no
 *  window buttons there. */
const drawsWindowButtons = hasDesktop && platform.os !== "macos";

/** The rule under the bar, per decoration language: an opaque decoration tone
 *  for the framed skins, a softer hairline for archive, the top hilite for
 *  glass, and the reduced-contrast separator the web client used for the flat
 *  skin. */
const SEPARATOR: Record<Decor, string> = {
  plain: "border-input/60",
  glass: "border-hilite",
  archive: "border-decor-line/55",
  limbus: "border-decor-line",
  mist: "border-decor-line",
  hairline: "border-border",
  gradient: "border-transparent",
};

/** The bar's own surface: the gradient skin lets the page's wash run through
 *  it, the others draw the card surface the window chrome has always had. */
const BAR_BG: Record<Decor, string> = {
  plain: "bg-card",
  glass: "bg-card",
  archive: "bg-card",
  limbus: "bg-card",
  mist: "bg-card",
  hairline: "bg-card",
  gradient: "bg-transparent",
};

/** The appearance toggle's glyph and label, in the order it cycles. */
const APPEARANCE: Record<Appearance, { icon: IconName; key: string }> = {
  light: { icon: "sun", key: "themeLight" },
  dark: { icon: "moon", key: "themeDark" },
  system: { icon: "monitor", key: "themeSystem" },
};

export function TitleBar() {
  const bar = () =>
    hasDesktop ? titleBar() : { height: TITLEBAR_HEIGHT, left: 0, right: 0 };
  return (
    <View
      class={`flex-none flex-row items-center gap-1 border-b ${BAR_BG[currentDecor()]} ${SEPARATOR[currentDecor()]}`}
      style={{ height: TITLEBAR_HEIGHT, paddingLeft: bar().left }}
    >
      {/* Brand: a drag region, as the platform's own title bar was. */}
      <View class="h-full flex-row items-center gap-2 ps-3 pe-3" desktop={{ windowDrag: true }}>
        <Image src={banner} style={{ width: 38, height: 20 }} class="flex-none" accessibilityElementsHidden />
        <Text class="text-xs font-semibold text-foreground">AALC</Text>
      </View>

      <NavBar />

      {/* Drag spacer: the bar's empty middle belongs to the window. */}
      <View class="h-full flex-1" desktop={{ windowDrag: true }} />

      <View class="flex-row items-center gap-1 pe-2">
        <IconButton
          name="chart"
          label={pageLabel("stats")}
          size="md"
          variant={currentPage() === "stats" ? "outline" : "ghost"}
          iconColor={currentPage() === "stats" ? brandIcon() : undefined}
          onPress={() => navigate("stats")}
          class="size-7"
        />
        <IconButton
          name="settings"
          label={pageLabel("settings")}
          size="md"
          variant={currentPage() === "settings" ? "outline" : "ghost"}
          iconColor={currentPage() === "settings" ? brandIcon() : undefined}
          onPress={() => navigate("settings")}
          class="size-7"
        />
        <AppearanceButton />
      </View>

      <If when={drawsWindowButtons}>
        <View class="h-full flex-row">
          <WindowButton control="minimize" icon="minimize" label={t("titlebarMinimize")} />
          <WindowButton control="maximize" icon="maximize" label={t("titlebarMaximize")} />
          <WindowButton control="close" icon="close" label={t("titlebarClose")} danger />
        </View>
      </If>
    </View>
  );
}

/** The destinations: one pill slides under the current one, and one button
 *  follows each. The pill's width is the target's from the moment the page
 *  changes and its left edge springs over to it, so a switch reads as one pill
 *  moving rather than one closing and another opening. */
function NavBar() {
  const origin = createAnimatedValue(0);
  const stretch = createAnimatedValue(1);
  const fade = createAnimatedValue(1);
  const measured = signal<Record<string, { x: number; w: number }>>({});
  const pill = signal({ w: 0 });
  let previous: { x: number; w: number } | null = null;
  let width = 0;
  let visible = true;

  const place = (next: number) => {
    if (Math.abs(width - next) < 0.5) return;
    width = next;
    pill.set({ w: next });
  };

  const report = (id: string, bounds: { origin: { x: number }; size: { width: number } }) => {
    const next = { x: bounds.origin.x, w: bounds.size.width };
    const current = measured()[id];
    if (
      current &&
      Math.abs(current.x - next.x) < 0.5 &&
      Math.abs(current.w - next.w) < 0.5
    ) {
      return;
    }
    measured.set({ ...measured(), [id]: next });
  };

  effect(() => {
    const target = measured()[currentPage()];
    const onNav = target !== undefined;
    // `Animated` runs natively and has no reduced-motion switch of its own:
    // the charge is ours, so the user's setting lands the pill at once.
    const calm = reduceMotion();
    // Settings, Stats and Help have no tab: the pill fades out for them.
    if (onNav !== visible) {
      visible = onNav;
      if (calm) fade.set(onNav ? 1 : 0);
      else Animated.timing(fade, { toValue: onNav ? 1 : 0, duration: PILL_FADE_MS }).start();
    }
    if (!target || target.w <= 0) return;
    if (!previous) {
      // The first layout: place it where the current page's tab is.
      previous = target;
      origin.set(target.x);
      stretch.set(1);
      place(target.w);
      return;
    }
    const moved =
      Math.abs(previous.x - target.x) >= 0.5 || Math.abs(previous.w - target.w) >= 0.5;
    const from = previous;
    previous = target;
    if (!moved) {
      place(target.w);
      return;
    }
    if (calm) {
      origin.set(target.x);
      stretch.set(1);
      place(target.w);
      return;
    }
    // The native values land before the new width does. Taking the width first
    // would leave the pill one frame wide as the target tab while its edge and
    // scale were still the old ones: a pill stretched across both tabs.
    origin.set(from.x);
    stretch.set(from.w / target.w);
    place(target.w);
    Animated.parallel([
      Animated.timing(origin, { toValue: target.x, duration: PILL_MS, easing: PILL_EASING }),
      Animated.timing(stretch, { toValue: 1, duration: PILL_MS, easing: PILL_EASING }),
    ]).start();
  });

  return (
    <View class="relative flex-row items-center gap-1">
      <View
        class="absolute inset-y-0 rounded-full bg-brand"
        style={{
          width: pill().w,
          transformOrigin: "left center",
          transform: [{ translateX: origin }, { scaleX: stretch }],
          opacity: fade,
        }}
      />
      <Each of={PAGES}>
        {(entry) => (
          <View class="flex-none" onLayout={({ bounds }) => report(entry().id, bounds)}>
            <NavItem
              id={entry().id}
              label={pageLabel(entry().id)}
              icon={entry().icon}
              status={entry().id === "home" ? currentHomeStatus() : null}
            />
          </View>
        )}
      </Each>
    </View>
  );
}

/** One destination in the bar: icon, label, and the console's status dot. */
function NavItem(props: {
  id: PageId;
  label: string;
  icon: IconName;
  status: HomeStatus | null;
}) {
  const active = () => currentPage() === props.id;
  return (
    <Button
      label={props.label}
      icon={props.icon}
      size="md"
      variant={active() ? "nav" : "ghost"}
      selected={active()}
      onPress={() => navigate(props.id)}
      class="h-8 px-2"
    >
      <If when={props.status}>
        {(status) => <StatusDot status={status()} />}
      </If>
    </Button>
  );
}

/** The accent token, for the settings button's open state (the GPUI shell
 *  tinted its glyph with `brand` while Settings was showing). */
function brandIcon(): string {
  return token("brand");
}

/** Cycles light → dark → system, and says which one is showing. */
function AppearanceButton() {
  const appearance = () => APPEARANCE[currentAppearance()];
  return (
    <IconButton
      name={appearance().icon}
      label={`${t("appearanceLabel")}：${t(appearance().key)}`}
      size="md"
      onPress={() => {
        cycleAppearance();
      }}
      class="size-7"
    />
  );
}

/** One of the platform's window buttons, drawn by the app. The platform
 *  hit-tests it as the real button and carries out the press, so there is no
 *  `onPress` here: a second handler would fire twice on Windows. */
function WindowButton(props: {
  control: "minimize" | "maximize" | "close";
  icon: IconName;
  label: string;
  danger?: boolean;
}) {
  const hover = () => (props.danger ? "hover:bg-danger" : "hover:bg-secondary");
  return (
    <View
      role="button"
      accessibilityLabel={props.label}
      desktop={{ windowControl: props.control }}
      class={`h-full w-12 flex-row items-center justify-center active:bg-secondary focus-visible:outline-2 focus-visible:-outline-offset-2 focus-visible:outline-ring ${hover()}`}
    >
      <Icon name={props.icon} size={16} />
    </View>
  );
}
