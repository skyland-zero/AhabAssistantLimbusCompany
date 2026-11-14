// A reading in a data card: what it counts, how it compares, and the figure.
//
// Label and note share one line so the meaning sits beside the name, and the
// figure under them is the loudest thing in the card. The unit (a denominator,
// `层`) stays one size down: it qualifies the number, it is not a second one.
import { Text, View } from "nsh";

export function Figure(props: {
  label: string;
  /** The comparison or the qualifier: `本周 11`, `今日 3 · 本周 11`, `普通`. */
  note?: string;
  value: string;
  unit?: string;
  /** The value is an absence (`—`) or a placeholder, not a reading. */
  quiet?: boolean;
}) {
  return (
    <View class="min-w-0 flex-1 flex-col">
      <View class="w-full flex-row items-baseline gap-1.5">
        <Text class="flex-none text-[10px] text-muted-foreground">{props.label}</Text>
        <If when={props.note}>
          {(note) => (
            <Text class="min-w-0 truncate text-[10px] text-muted-foreground opacity-80">
              {note()}
            </Text>
          )}
        </If>
      </View>
      <View class="mt-0.5 flex-row items-baseline gap-1">
        <Text
          class={`text-xl tracking-tight tabular-nums ${
            props.quiet ? "font-semibold text-muted-foreground" : "font-bold text-foreground"
          }`}
        >
          {props.value}
        </Text>
        <If when={props.unit}>
          {(unit) => (
            <Text class="flex-none text-xs font-semibold text-muted-foreground">{unit()}</Text>
          )}
        </If>
      </View>
    </View>
  );
}
