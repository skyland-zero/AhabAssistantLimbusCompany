// The app's icon set: Lucide SVGs in `assets/icons/`, drawn as an image and
// tinted with a theme token.
//
// `nsh` draws SVG through `Image`, so a glyph cannot inherit `text-*` the way
// GPUI's `currentColor` did: the colour is the `color` prop, a theme token by
// default. Callers therefore pass a token, never a literal; a state that has to
// recolour an icon (a selected nav item) reads the token it wants.
import { Image, token } from "nsh";
import check from "../assets/icons/check.svg";
import chevronDown from "../assets/icons/chevron-down.svg";
import circleQuestionMark from "../assets/icons/circle-question-mark.svg";
import house from "../assets/icons/house.svg";
import loaderCircle from "../assets/icons/loader-circle.svg";
import minus from "../assets/icons/minus.svg";
import monitor from "../assets/icons/monitor.svg";
import moon from "../assets/icons/moon.svg";
import packageIcon from "../assets/icons/package.svg";
import paletteIcon from "../assets/icons/palette.svg";
import settingsIcon from "../assets/icons/settings.svg";
import square from "../assets/icons/square.svg";
import squareStack from "../assets/icons/square-stack.svg";
import sun from "../assets/icons/sun.svg";
import users from "../assets/icons/users.svg";
import wrench from "../assets/icons/wrench.svg";
import x from "../assets/icons/x.svg";

/** Every glyph the UI may draw, named by what it means to the app. */
const ICONS = {
  check,
  chevronDown,
  close: x,
  help: circleQuestionMark,
  home: house,
  loader: loaderCircle,
  maximize: square,
  minimize: minus,
  monitor,
  moon,
  package: packageIcon,
  palette: paletteIcon,
  restore: squareStack,
  settings: settingsIcon,
  sun,
  users,
  wrench,
} as const;

export type IconName = keyof typeof ICONS;

/** A decorative glyph: the label beside it carries the meaning, so screen
 *  readers skip it. */
export function Icon(props: {
  name: IconName;
  /** The square size in points (14 when left out). */
  size?: number;
  /** A theme token's colour (`token("brand")`); `muted-foreground` when left out. */
  color?: string;
  class?: string;
}) {
  return (
    <Image
      src={ICONS[props.name]}
      accessibilityElementsHidden
      class={`flex-none ${props.class ?? ""}`}
      style={{
        width: props.size ?? 14,
        height: props.size ?? 14,
        tintColor: props.color ?? token("muted-foreground"),
      }}
    />
  );
}
