// The app's button: one press target, six variants, four sizes, and the
// disabled and selected states. It wraps `nsh/base`'s `Button`, which owns the
// presses, the keyboard activation, the disabled and selected semantics for
// screen readers; only the look is decided here.
//
// Geometry follows the GPUI client's control scale: the label is 12pt
// (`text-xs`) and the frame is 20pt tall (`min-h-5`), which is what
// `components/base.rs` computed from `FONT_SM + CONTROL_HEIGHT_PADDING`.
//
// An icon is an image tinted with a token, so it cannot inherit the label's
// colour. Each variant therefore names its icon token, and a caller that
// recolours the label passes `iconColor`.
import type { JSX } from "nsh";
import { Text, token } from "nsh";
import { Button as BaseButton } from "nsh/base";
import { Icon } from "./Icon";
import type { IconName } from "./Icon";

/** What the button means, not what it looks like. */
export type ButtonVariant =
  | "default" // the ordinary filled action, in the accent (the client's `Default`)
  | "primary" // the one default commit of a decision area: the same look as `default`
  | "secondary" // a quieter filled action, and the "current" state of a choice
  | "outline" // an action that needs a boundary with less weight
  | "ghost" // a familiar, low-emphasis action
  | "danger" // a destructive commitment
  | "link"; // an external resource only: never an in-app command

/** One step of the control scale: frame, padding, text and icon grow together. */
export type ButtonSize = "xs" | "sm" | "md" | "lg";

type VariantStyle = { frame: string; icon: () => string };

const VARIANT: Record<ButtonVariant, VariantStyle> = {
  // The old client's `Default` button is its accent-filled one (GPUI Kit's
  // `Primary` is projected onto the accent), so both names draw the same thing;
  // call sites pick the one that says what they mean.
  default: {
    frame:
      "bg-brand text-brand-foreground hover:bg-brand-hover active:bg-brand-hover selected:bg-brand-hover",
    icon: () => token("brand-foreground"),
  },
  primary: {
    frame:
      "bg-brand text-brand-foreground hover:bg-brand-hover active:bg-brand-hover selected:bg-brand-hover",
    icon: () => token("brand-foreground"),
  },
  secondary: {
    frame:
      "bg-secondary text-secondary-foreground hover:bg-secondary/80 active:bg-secondary/70 selected:bg-secondary/80",
    icon: () => token("secondary-foreground"),
  },
  outline: {
    frame:
      "border border-input bg-card text-foreground hover:bg-accent hover:text-accent-foreground active:bg-secondary selected:bg-accent",
    icon: () => token("foreground"),
  },
  ghost: {
    frame:
      "bg-transparent text-muted-foreground hover:bg-accent hover:text-accent-foreground active:bg-secondary selected:bg-accent selected:text-foreground",
    icon: () => token("muted-foreground"),
  },
  danger: {
    frame:
      "bg-danger text-danger-foreground hover:bg-danger/90 active:bg-danger/80 selected:bg-danger/90",
    icon: () => token("danger-foreground"),
  },
  link: {
    frame:
      "bg-transparent px-0 text-brand underline underline-offset-4 hover:text-brand-hover active:opacity-80",
    icon: () => token("brand"),
  },
};

const SIZE: Record<ButtonSize, { frame: string; icon: number }> = {
  xs: { frame: "min-h-5 gap-1 px-2", icon: 12 },
  sm: { frame: "min-h-5 gap-1.5 px-3", icon: 13 },
  md: { frame: "min-h-6 gap-1.5 px-3", icon: 14 },
  lg: { frame: "min-h-7 gap-2 px-4", icon: 16 },
};

/** Text size is part of the size step, so it is not per variant. */
const SIZE_TEXT: Record<ButtonSize, string> = {
  xs: "text-xs",
  sm: "text-xs",
  md: "text-xs",
  lg: "text-sm",
};

const FRAME =
  "flex-row items-center justify-center rounded-(--radius) font-medium transition-colors duration-150 " +
  "active:scale-[0.97] motion-reduce:transition-none motion-reduce:active:scale-100 " +
  "focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-ring " +
  "disabled:pointer-events-none disabled:opacity-50";

/** A button with a text label, an optional leading icon, and optional trailing
 *  content (a status dot, a count). */
export function Button(props: {
  label: string;
  variant?: ButtonVariant;
  size?: ButtonSize;
  icon?: IconName;
  /** The icon's token, when the variant's own does not match a recoloured label. */
  iconColor?: string;
  disabled?: boolean;
  selected?: boolean;
  /** What screen readers say, when the label alone is not enough. */
  accessibilityLabel?: string;
  onPress?: () => void;
  class?: string;
  testID?: string;
  children?: JSX.Element;
}) {
  const style = () => VARIANT[props.variant ?? "default"];
  return (
    <BaseButton
      onPress={props.onPress}
      disabled={props.disabled}
      selected={props.selected}
      accessibilityLabel={props.accessibilityLabel}
      testID={props.testID}
      class={`${FRAME} ${SIZE[props.size ?? "sm"].frame} ${SIZE_TEXT[props.size ?? "sm"]} ${style().frame} ${props.class ?? ""}`}
    >
      <If when={props.icon}>
        {(name) => (
          <Icon name={name()} size={SIZE[props.size ?? "sm"].icon} color={props.iconColor ?? style().icon()} />
        )}
      </If>
      <Text>{props.label}</Text>
      {props.children}
    </BaseButton>
  );
}

/** A square icon-only control: the same frames and states, no label. Its
 *  accessible name is required, since nothing on screen states it. */
export function IconButton(props: {
  name: IconName;
  /** What screen readers announce (a tooltip repeats it where one is drawn). */
  label: string;
  variant?: ButtonVariant;
  size?: ButtonSize;
  /** The icon's token, when the variant's own is not the wanted colour. */
  iconColor?: string;
  disabled?: boolean;
  selected?: boolean;
  onPress?: () => void;
  class?: string;
  testID?: string;
  children?: JSX.Element;
}) {
  const style = () => VARIANT[props.variant ?? "ghost"];
  return (
    <BaseButton
      onPress={props.onPress}
      disabled={props.disabled}
      selected={props.selected}
      accessibilityLabel={props.label}
      testID={props.testID}
      class={`${FRAME} ${SIZE[props.size ?? "sm"].frame} aspect-square ${style().frame} ${props.class ?? ""}`}
    >
      <Icon name={props.name} size={SIZE[props.size ?? "sm"].icon} color={props.iconColor ?? style().icon()} />
      {props.children}
    </BaseButton>
  );
}
