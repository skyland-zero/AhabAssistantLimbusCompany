// The interface-skin grid: a miniature of the app under each skin.
//
// The miniature is drawn from that skin's own token table (not from the running
// theme and not from a screenshot), so it always reflects the current accent and
// appearance. Its sizes are literal points on purpose: it is a scaled-down
// diagram, like a raster preview, not page layout.
import { Text, View, colorScheme } from "nsh";
import { Button } from "nsh/base";
import { SKINS, SKIN_BLURBS, SKIN_LABELS, tokensFor } from "../../../themes";
import type { SkinName } from "../../../themes";
import { setFeedback } from "../../../settings";
import { currentAccent, currentSkin, setSkin } from "../../../store";

export function SkinPicker() {
  return (
    <View class="w-full flex-row flex-wrap gap-3">
      <Each of={SKINS}>
        {(skin) => (
          <SkinOption
            skin={skin()}
            active={currentSkin() === skin()}
            onPress={() => {
              setSkin(skin());
              setFeedback(`界面皮肤：${SKIN_LABELS[skin()]}`);
            }}
          />
        )}
      </Each>
    </View>
  );
}

function SkinOption(props: { skin: SkinName; active: boolean; onPress: () => void }) {
  const tokens = () => tokensFor(props.skin, currentAccent(), colorScheme());
  return (
    <Button
      onPress={props.onPress}
      selected={props.active}
      accessibilityLabel={`${SKIN_LABELS[props.skin]} / ${props.skin}`}
      class={`min-w-44 flex-1 flex-col items-stretch gap-2 border p-2 text-left transition-colors duration-150 active:scale-[0.99] motion-reduce:transition-none focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-ring ${
        props.active ? "border-2 border-ring" : "border border-border hover:border-ring/60"
      }`}
      style={{ backgroundColor: tokens()["background"], borderRadius: 8 }}
    >
      <Frame skin={props.skin} />

      <View class="flex-col gap-0.5" style={{ width: "100%" }}>
        <View class="flex-row items-center justify-between" style={{ width: "100%" }}>
          <Text class="text-xs font-medium" style={{ color: tokens()["foreground"] }}>
            {SKIN_LABELS[props.skin]}
          </Text>
          <If when={props.active}>
            <Text class="text-xs" style={{ color: tokens()["brand"] }}>
              已启用
            </Text>
          </If>
        </View>
        <Text class="text-xs" style={{ color: tokens()["muted-foreground"] }}>
          {SKIN_BLURBS[props.skin]}
        </Text>
      </View>
    </Button>
  );
}

/** The preview's own frame: a card-coloured plate holding the wireframe. */
function Frame(props: { skin: SkinName }) {
  const t = () => tokensFor(props.skin, currentAccent(), colorScheme());
  return (
    <View style={{ width: "100%", padding: 3, backgroundColor: t()["card"], borderRadius: 6 }}>
      <Miniature skin={props.skin} />
    </View>
  );
}

/** A 78pt-tall wireframe of the app: title bar, two panels, one control. */
function Miniature(props: { skin: SkinName }) {
  const t = () => tokensFor(props.skin, currentAccent(), colorScheme());
  return (
    <View
      style={{
        width: "100%",
        height: 78,
        gap: 4,
        padding: 5,
        backgroundColor: t()["background"],
        borderRadius: 6,
      }}
    >
      <TitleStrip skin={props.skin} />
      <View style={{ flexDirection: "row", flex: 1, gap: 4 }}>
        <View
          style={{
            flex: 1,
            gap: 3,
            padding: 4,
            backgroundColor: t()["card"],
            borderRadius: 3,
          }}
        >
          <PanelHeader skin={props.skin} />
          <Bar color={t()["muted-foreground"]} width="100%" opacity={0.55} />
          <Bar color={t()["muted-foreground"]} width="100%" opacity={0.35} />
        </View>
        <View
          style={{
            flex: 1,
            gap: 3,
            padding: 4,
            backgroundColor: t()["secondary"],
            borderRadius: 3,
          }}
        >
          <Bar color={t()["secondary-foreground"]} width="100%" opacity={0.5} />
          <View style={{ flexDirection: "row", alignItems: "center", gap: 3 }}>
            <View
              style={{
                width: 18,
                height: 10,
                borderRadius: 5,
                backgroundColor: t()["primary"],
              }}
            />
            <Bar color={t()["muted-foreground"]} width={24} opacity={0.4} />
          </View>
        </View>
      </View>
      <View
        style={{
          flexDirection: "row",
          justifyContent: "center",
          alignItems: "center",
          gap: 3,
          paddingTop: 3,
        }}
      >
        <View style={{ width: 14, height: 4, borderRadius: 2, backgroundColor: t()["brand"] }} />
        <View
          style={{ width: 26, height: 4, borderRadius: 2, backgroundColor: t()["brand-hover"] }}
        />
      </View>
    </View>
  );
}

/** The miniature title bar: the skin's rule, a brand chip and two nav ticks. */
function TitleStrip(props: { skin: SkinName }) {
  const t = () => tokensFor(props.skin, currentAccent(), colorScheme());
  return (
    <View
      style={{
        flexDirection: "row",
        alignItems: "center",
        gap: 3,
        height: 12,
        paddingHorizontal: 4,
        backgroundColor: t()["card"],
        borderBottomWidth: 1,
        borderBottomColor: t()["decor-line"],
      }}
    >
      <View
        style={{ width: 10, height: 6, borderRadius: 2, backgroundColor: t()["brand"] }}
      />
      <Bar color={t()["muted-foreground"]} width={14} opacity={0.5} />
      <Bar color={t()["muted-foreground"]} width={10} opacity={0.5} />
    </View>
  );
}

/** The miniature card header, in that skin's decoration language. */
function PanelHeader(props: { skin: SkinName }) {
  const t = () => tokensFor(props.skin, currentAccent(), colorScheme());
  return (
    <View
      style={{
        flexDirection: "row",
        height: 6,
        borderBottomWidth: 1,
        borderBottomColor: t()["decor-line"],
        backgroundColor: props.skin === "limbus" ? t()["band-top"] : t()["card"],
      }}
    >
      <If when={props.skin === "mist" || props.skin === "limbus"}>
        <View style={{ width: 3, height: 6, backgroundColor: t()["decor-line"] }} />
      </If>
    </View>
  );
}

/** One line of the wireframe. `width` of 0 stretches. */
function Bar(props: { color: string; width: number | `${number}%`; opacity: number }) {
  return (
    <View
      style={{
        height: 3,
        borderRadius: 2,
        backgroundColor: props.color,
        opacity: props.opacity,
        width: props.width === 0 ? "100%" : props.width,
      }}
    />
  );
}
