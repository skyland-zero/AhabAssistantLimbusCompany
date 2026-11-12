// The app's select: one value out of a short list, on the shared popup surface.
//
// `nsh/base`'s `Select` owns the trigger's keyboard behavior, the list's
// placement, the navigation, the dismissal and the listbox roles; this file
// draws the trigger and gives the popup the app's surface. Every popup in the
// app shares one surface (`popover` tokens, `radius`, one shadow), so a select,
// a menu and a popover cannot drift apart.
import { Text, View } from "nsh";
import { Select as BaseSelect } from "nsh/base";
import { Icon } from "./Icon";

/** The app's popup surface, shared by selects, menus and popovers. */
export const POPUP_CLASS =
  "min-w-40 rounded-(--radius) border border-border bg-popover p-1 text-popover-foreground shadow-lg";

/** The trigger's frame: the same height and border as an input, so text in a
 *  field and in the list it opens start on one line. */
const TRIGGER_CLASS =
  "min-h-5 w-full flex-row items-center justify-between gap-2 rounded-(--radius) " +
  "border border-input bg-card px-2 py-1 " +
  "focus-visible:outline-2 focus-visible:outline-offset-0 focus-visible:outline-ring";

export function Select<T extends string>(props: {
  items: { value: T; label: string }[];
  value: T;
  onChange: (value: T) => void;
  /** What screen readers call it: what the choice is about. */
  accessibilityLabel?: string;
  disabled?: boolean;
  class?: string;
}) {
  const label = () =>
    props.items.find((item) => item.value === props.value)?.label ?? "";
  return (
    <BaseSelect
      items={props.items.map((item) => ({ value: item.value, title: item.label }))}
      value={props.value}
      onChange={(event) => {
        const chosen = props.items.find((item) => item.value === event.value);
        if (chosen) props.onChange(chosen.value);
      }}
      accessibilityLabel={props.accessibilityLabel}
      disabled={props.disabled}
      popupClass={POPUP_CLASS}
      trigger={
        <View class={`${TRIGGER_CLASS} ${props.class ?? ""}`}>
          <Text class="text-xs text-foreground">{label()}</Text>
          <Icon name="chevronDown" size={12} />
        </View>
      }
    />
  );
}
