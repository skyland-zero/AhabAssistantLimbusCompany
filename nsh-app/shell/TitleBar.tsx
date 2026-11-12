// The app-drawn window chrome: the brand, the six destinations, the execution
// status dot, the appearance and settings utilities, and (outside macOS) the
// window buttons.
//
// `nsh.json` asks for a hidden title bar, so the app paints it and keeps the
// platform's window buttons where the platform has them. The drag regions are
// the brand block and the flexible spacer only, so a press on a destination is
// a press, not a window drag.
import { Image, Text, View, platform, token } from "nsh";
import { isAvailable as hasDesktop, titleBar } from "nsh/desktop";
import banner from "../assets/limbus_title_banner.png";
import { Button, IconButton } from "../components/Button";
import { Icon } from "../components/Icon";
import type { IconName } from "../components/Icon";
import { PAGES, currentAppearance, currentDecor, currentHomeStatus, currentPage, cycleAppearance, navigate, pageLabel } from "../store";
import type { Appearance, HomeStatus, PageId } from "../store";
import type { Decor } from "../themes";
import { StatusDot } from "./StatusDot";

/** The bar's area: the platform's own when it has a title bar (a hidden one on
 *  the desktops), a fixed 40pt otherwise (the headless runner, a browser). */
const FALLBACK_HEIGHT = 40;

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
};

/** The appearance toggle's glyph and label, in the order it cycles. */
const APPEARANCE: Record<Appearance, { icon: IconName; label: string }> = {
  light: { icon: "sun", label: "浅色" },
  dark: { icon: "moon", label: "深色" },
  system: { icon: "monitor", label: "跟随系统" },
};

export function TitleBar() {
  const bar = () =>
    hasDesktop ? titleBar() : { height: FALLBACK_HEIGHT, left: 0, right: 0 };
  return (
    <View
      class={`flex-none flex-row items-center gap-1 border-b bg-card ${SEPARATOR[currentDecor()]}`}
      style={{ height: bar().height || FALLBACK_HEIGHT, paddingLeft: bar().left }}
    >
      {/* Brand: a drag region, as the platform's own title bar was. */}
      <View class="h-full flex-row items-center gap-2 ps-3 pe-3" desktop={{ windowDrag: true }}>
        <Image src={banner} style={{ width: 38, height: 20 }} class="flex-none" accessibilityElementsHidden />
        <Text class="text-xs font-semibold text-foreground">AALC</Text>
      </View>

      <View class="flex-row items-center gap-1">
        <Each of={PAGES}>
          {(entry) => (
            <NavItem
              id={entry().id}
              label={entry().label}
              icon={entry().icon}
              status={entry().id === "home" ? currentHomeStatus() : null}
            />
          )}
        </Each>
      </View>

      {/* Drag spacer: the bar's empty middle belongs to the window. */}
      <View class="h-full flex-1" desktop={{ windowDrag: true }} />

      <View class="flex-row items-center gap-1 pe-2">
        <AppearanceButton />
        <IconButton
          name="settings"
          label={pageLabel("settings")}
          size="md"
          variant={currentPage() === "settings" ? "outline" : "ghost"}
          iconColor={currentPage() === "settings" ? brandIcon() : undefined}
          onPress={() => navigate("settings")}
          class="size-7"
        />
      </View>

      <If when={drawsWindowButtons}>
        <View class="h-full flex-row">
          <WindowButton control="minimize" icon="minimize" label="最小化" />
          <WindowButton control="maximize" icon="maximize" label="最大化" />
          <WindowButton control="close" icon="close" label="关闭" danger />
        </View>
      </If>
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
      variant={active() ? "primary" : "ghost"}
      selected={active()}
      onPress={() => navigate(props.id)}
      class="px-2"
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
      label={`外观：${appearance().label}`}
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
