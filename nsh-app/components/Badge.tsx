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

/** A compact label chip. */
export function Badge(props: { label: string; tone?: BadgeTone; class?: string }) {
  return (
    <View
      class={`flex-row items-center rounded-(--radius-sm) border border-border px-2 py-0.5 ${TONE[props.tone ?? "neutral"]} ${props.class ?? ""}`}
    >
      <Text class="text-xs">{props.label}</Text>
    </View>
  );
}
