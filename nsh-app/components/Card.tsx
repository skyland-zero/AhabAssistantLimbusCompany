// The app's card surface and its header, in all five decoration languages.
//
// Every skin shares one card structure (same padding, same 44pt header slot)
// and changes only its decoration: the limbus frame draws corner brackets and a
// stencilled band, glass a top hilite, archive a trailing hairline, mist a
// leading accent bar. A skin switch therefore never moves the body below the
// header.
import type { JSX } from "nsh";
import { Image, Text, View, createMotion, token } from "nsh";
import { arriveSpec } from "../motion";
import dividerArt from "../assets/themes/limbus/divider.png";
import { currentDecor } from "../store";
import type { Decor } from "../themes";

/** Shared card-header height (44pt in the GPUI client, `CARD_HEADER_HEIGHT`). */
const HEADER_HEIGHT = "h-11";

/** Per-skin elevation: flat skins separate with hairlines and whitespace, the
 *  translucent ones float. */
const CARD_SHADOW: Record<Decor, string> = {
  plain: "shadow-sm",
  glass: "shadow-lg",
  archive: "shadow-none",
  limbus: "shadow-none",
  mist: "shadow-md",
  // Structure only: the hairline border already explains the surface.
  hairline: "shadow-none",
  // A phone app's card: white over the page's wash, lifted by one soft shadow.
  gradient: "shadow-sm",
};

/** A surface container: the app's card geometry, padding and decoration. */
export function Card(props: {
  /** The header's label. A card without one is a bare surface. */
  title?: string;
  /** The card is the current one — the running task, the open item. It keeps
   *  the skin's surface and takes the accent as a border and a faint fill. */
  selected?: boolean;
  /** Its place in a list: the card arrives as one of a wave when the page is
   *  entered (fades in and rises, a step after the card before it). The motion
   *  rides the card's own frame, so a grid keeps its `items-stretch` and no
   *  wrapper is added to the layout. */
  arrive?: number;
  children?: JSX.Element;
  class?: string;
}) {
  // Created whether or not it is used, so the component always makes the same
  // allocations; `enter` stays unset (and the style unbound) for a still card.
  const arrival = createMotion(
    props.arrive === undefined ? undefined : { enter: arriveSpec(props.arrive) },
  );
  const surface = () =>
    props.selected ? "border-brand bg-brand/10" : "border-border bg-card";
  return (
    <View
      class={`relative min-w-0 rounded-(--radius-lg) border text-card-foreground ${surface()} ${CARD_SHADOW[currentDecor()]} ${props.class ?? "p-4"}`}
      style={props.arrive === undefined ? undefined : arrival.style}
    >
      <If when={props.title}>
        {(title) => <CardHeader label={title()} />}
      </If>
      {props.children}
      <If when={currentDecor() === "glass"}>
        {/* A one-pixel highlight along the top edge: what makes a translucent
            panel read as glass rather than as a flat tint. */}
        <View class="pointer-events-none absolute inset-x-0 top-0 h-px rounded-t-(--radius-lg) bg-hilite" />
      </If>
      <If when={currentDecor() === "limbus"}>
        <Bracket top left />
        <Bracket top />
        <Bracket left />
        <Bracket />
      </If>
    </View>
  );
}

/** One L-bracket of the framed skin's corner decoration. */
function Bracket(props: { top?: boolean; left?: boolean }) {
  return (
    <View
      class={`pointer-events-none absolute size-3.5 border-decor-line ${
        props.top ? "top-1 border-t-2" : "bottom-1 border-b-2"
      } ${props.left ? "left-1 border-l-2" : "right-1 border-r-2"}`}
    />
  );
}

/** The card's title band, in the active skin's decoration language. */
export function CardHeader(props: { label: string; children?: JSX.Element }) {
  const decor = () => currentDecor();
  return (
    <View
      role="header"
      class={`${HEADER_HEIGHT} relative flex-none flex-row items-center px-4 ${HEADER_CLASS[decor()]}`}
    >
      <If when={decor() === "limbus"}>
        {/* The stencilled band: a dark-red (or parchment) gradient behind the
            label, with grime marks only where they read as texture. */}
        <View
          class="pointer-events-none absolute inset-0"
          style={{
            backgroundImage: `linear-gradient(180deg, ${token("band-top")} 0%, ${token("band-bottom")} 100%)`,
          }}
        />
        <View class="pointer-events-none absolute inset-y-0 left-3.5 w-[3px] bg-decor-line" />
        <View class="pointer-events-none absolute inset-x-0 bottom-0 h-px bg-decor-line" />
        <Smear class="left-14 top-2 h-2 w-8" />
        <Smear class="left-33 top-7 h-1.5 w-4" />
        <Smear class="right-33 top-3 h-1.5 w-7" />
        <Smear class="right-13 top-7 h-1.5 w-11" />
      </If>
      <If when={decor() === "mist"}>
        <View class="pointer-events-none absolute inset-y-0 left-0 w-[3px] bg-decor-line" />
      </If>
      <Text class={`text-sm font-semibold ${HEADER_LABEL[decor()]}`}>{props.label}</Text>
      <If when={decor() === "archive"}>
        <View class="ms-3 h-px flex-1 bg-decor-line opacity-35" />
      </If>
      {props.children}
    </View>
  );
}

/** One grime mark on the limbus band. The `scrim` token is a dark translucent
 *  ink in both appearances, so the marks stay texture and never become opaque
 *  redaction bars on parchment. */
function Smear(props: { class: string }) {
  return <View class={`pointer-events-none absolute rounded-full bg-scrim/40 ${props.class}`} />;
}

/** Borders and dividers per decoration language. All of them are drawn once,
 *  by the region that owns the boundary. */
const HEADER_CLASS: Record<Decor, string> = {
  plain: "border-b border-input",
  glass: "border-t border-hilite",
  archive: "border-b border-border",
  limbus: "overflow-hidden",
  mist: "border-b border-border",
  hairline: "border-b border-border",
  gradient: "border-b-0",
};

const HEADER_LABEL: Record<Decor, string> = {
  plain: "text-foreground",
  glass: "text-foreground",
  archive: "text-foreground",
  limbus: "text-accent-foreground",
  mist: "text-foreground",
  hairline: "text-foreground",
  gradient: "text-foreground",
};

/** A hairline rule: a separator inside a card or between regions. The ruled
 *  skins draw it in their decoration tone, the others in the border tone. */
export function Rule(props: { class?: string }) {
  return (
    <View
      class={`h-px w-full flex-none ${RULE_CLASS[currentDecor()]} ${props.class ?? ""}`}
    />
  );
}

const RULE_CLASS: Record<Decor, string> = {
  plain: "bg-border",
  glass: "bg-border",
  archive: "bg-decor-line opacity-60",
  limbus: "bg-decor-line opacity-60",
  mist: "bg-border",
  hairline: "bg-border",
  gradient: "bg-border",
};

/** A decorative section divider: the shipped gold flourish for the ruled
 *  skins, a plain hairline for the rest. Kept at the source art's aspect ratio
 *  and a fixed width, so it can never stretch. */
export function Divider() {
  return (
    <View class="w-full flex-row justify-center py-1">
      <If when={usesFlourish(currentDecor())}>
        <Image
          src={dividerArt}
          accessibilityElementsHidden
          style={{ width: 120, height: 120 * (dividerArt.size.height / dividerArt.size.width), opacity: 0.85 }}
        />
      </If>
      <If when={!usesFlourish(currentDecor())}>
        <Rule />
      </If>
    </View>
  );
}

function usesFlourish(decor: Decor): boolean {
  return decor === "archive" || decor === "limbus";
}
