// The app's empty state: what a region shows before it has content, or while
// it has nothing to show. It names the state and, when there is one, the next
// action — never a bare blank region.
//
// Each decoration language gets its own marker in the same slot: limbus stamps
// its wax seal, archive leaves a ruled blank, glass a raised chip, mist a thin
// bar, and the flat skin nothing at all. None of them changes the layout.
import type { JSX } from "nsh";
import { Image, Text, View } from "nsh";
import seal from "../assets/themes/limbus/seal-red.png";
import { currentDecor } from "../store";

export function EmptyState(props: {
  /** The state, e.g. "还没有队伍". */
  title: string;
  /** What the state means, or what to do next. */
  detail?: string;
  /** The action that leaves the state, when there is one. */
  children?: JSX.Element;
  class?: string;
}) {
  return (
    <View
      class={`flex-1 items-center justify-center gap-2 rounded-(--radius-xl) border border-border p-6 ${props.class ?? ""}`}
    >
      <Marker />
      <Text class="text-sm font-semibold text-foreground">{props.title}</Text>
      <If when={props.detail}>
        {(detail) => (
          <Text class="max-w-96 text-center text-xs text-muted-foreground">{detail()}</Text>
        )}
      </If>
      {props.children}
    </View>
  );
}

/** The per-skin stand-in for an illustration. */
function Marker() {
  const decor = () => currentDecor();
  return (
    <View class="flex-none items-center">
      <If when={decor() === "limbus"}>
        <Image
          src={seal}
          accessibilityElementsHidden
          style={{ width: 72, height: 72, opacity: 0.85 }}
        />
      </If>
      <If when={decor() === "archive"}>
        <View class="h-0.5 w-16 flex-none bg-decor-line opacity-50" />
      </If>
      <If when={decor() === "glass"}>
        <View class="size-11 flex-none rounded-(--radius-lg) border border-hilite bg-secondary" />
      </If>
      <If when={decor() === "mist"}>
        <View class="h-0.5 w-7 flex-none bg-decor-line opacity-70" />
      </If>
    </View>
  );
}
