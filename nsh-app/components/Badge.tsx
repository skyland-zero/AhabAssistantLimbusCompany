// The app's short-label chip: a state, a count or a classification.
//
// Neutral is the default; the semantic tones are for states that really carry
// success, warning, danger or informational meaning, and should stay scarce.
import { Text, View } from "nsh";

export type BadgeTone = "neutral" | "accent" | "success" | "warning" | "info" | "danger";

/** Each tone is a contrast-tested pair from the theme, as the GPUI badge was. */
const TONE: Record<BadgeTone, string> = {
  neutral: "bg-muted text-muted-foreground",
  accent: "bg-brand-light text-brand",
  success: "bg-success-light text-success",
  warning: "bg-warning-light text-warning",
  info: "bg-brand-light text-brand",
  danger: "bg-danger-light text-danger",
};

/** A compact label chip. `dot` adds the state's own mark, so a running state is
 *  not carried by colour alone. */
export function Badge(props: {
  label: string;
  tone?: BadgeTone;
  dot?: boolean;
  class?: string;
}) {
  const dotColor: Record<BadgeTone, string> = {
    neutral: "bg-muted-foreground",
    accent: "bg-brand",
    success: "bg-success",
    warning: "bg-warning",
    info: "bg-brand",
    danger: "bg-danger",
  };
  return (
    <View
      class={`flex-row items-center gap-1.5 rounded-full border border-border px-2 py-0.5 ${TONE[props.tone ?? "neutral"]} ${props.class ?? ""}`}
    >
      <If when={props.dot}>
        <View class={`size-1.5 flex-none rounded-full ${dotColor[props.tone ?? "neutral"]}`} />
      </If>
      <Text class="text-xs">{props.label}</Text>
    </View>
  );
}
