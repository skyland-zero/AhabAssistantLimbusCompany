// The skin's background layer: the atmosphere a decorated skin puts behind the
// pages, between the window's `background` token and the content.
//
// Pages never draw it. Limbus paints its cold-black nebula plate (dark only:
// composited over parchment it grimes the page down to grey-taupe), mist a red
// vignette with a one-pixel diagonal grain, glass a faint accent wash along the
// top edge. Archive and the flat skin paint nothing.
import { Image, View, colorScheme, token } from "nsh";
import plate from "../assets/themes/limbus/bg.png";
import { currentDecor } from "../store";

export function SkinBackdrop() {
  const decor = () => currentDecor();
  const dark = () => colorScheme() === "dark";
  return (
    <View class="pointer-events-none absolute inset-0 overflow-hidden" accessibilityElementsHidden>
      <If when={decor() === "limbus" && dark()}>
        <Image
          src={plate}
          style={{ width: "100%", height: "100%", objectFit: "cover", opacity: 0.5 }}
        />
      </If>
      <If when={decor() === "mist"}>
        <Wash height="45%" />
        <View
          class="absolute inset-0"
          style={{
            backgroundImage: `repeating-linear-gradient(45deg, ${token("grain")} 0px, ${token("grain")} 1px, transparent 1px, transparent 6px)`,
          }}
        />
      </If>
      <If when={decor() === "glass"}>
        <Wash height="28%" />
      </If>
      <If when={decor() === "gradient"}>
        {/* The theme colour as a page wash: strongest at the leading edge and
            gone by 85% of the width, so the cards keep a white field. */}
        <View
          class="absolute inset-0"
          style={{ backgroundImage: `linear-gradient(90deg, ${token("wash")} 0%, transparent 85%)` }}
        />
      </If>
    </View>
  );
}

/** A top-edge gradient in the skin's wash token. A full-height tint would
 *  colour the whole page instead of only its atmosphere. */
function Wash(props: { height: `${number}%` }) {
  return (
    <View
      class="absolute inset-x-0 top-0"
      style={{
        height: props.height,
        backgroundImage: `linear-gradient(180deg, ${token("wash")} 0%, transparent 100%)`,
      }}
    />
  );
}
