// A one-line state strip: what just happened, or what the page wants to warn
// about. It is not a card — a notice belongs to the page, so it sits on the
// page's own surface and never opens a box inside the content stack.
import { Text, View, token } from "nsh";
import { Icon } from "./Icon";
import type { IconName } from "./Icon";

export type NoticeTone = "info" | "success" | "warning" | "danger";

const SURFACE: Record<NoticeTone, string> = {
  info: "bg-brand-light",
  success: "bg-success-light",
  warning: "bg-warning-light",
  danger: "bg-danger-light",
};

const INK: Record<NoticeTone, string> = {
  info: "text-brand",
  success: "text-success",
  warning: "text-warning",
  danger: "text-danger",
};

const GLYPH: Record<NoticeTone, IconName> = {
  info: "help",
  success: "check",
  warning: "help",
  danger: "help",
};

const TINT: Record<NoticeTone, "brand" | "success" | "warning" | "danger"> = {
  info: "brand",
  success: "success",
  warning: "warning",
  danger: "danger",
};

export function Notice(props: {
  tone?: NoticeTone;
  icon?: IconName;
  label: string;
  class?: string;
}) {
  const tone = () => props.tone ?? "info";
  return (
    <View
      class={`w-full flex-none flex-row items-center gap-2 rounded-(--radius) px-3 py-2 ${SURFACE[tone()]} ${props.class ?? ""}`}
    >
      <Icon name={props.icon ?? GLYPH[tone()]} size={14} color={token(TINT[tone()])} />
      <Text class={`min-w-0 flex-1 truncate text-xs ${INK[tone()]}`}>{props.label}</Text>
    </View>
  );
}
