// The app's select: one value out of a short list, on the shared popup surface.
//
// `nsh/base`'s `Select` owns the trigger's keyboard behavior, the list's
// placement, the navigation, the dismissal and the listbox roles; this file
// draws the trigger and gives the popup the app's surface. Every popup in the
// app shares one surface (`popover` tokens, `radius`, one shadow), so a select,
// a menu and a popover cannot drift apart.
import { Text, View, token } from "nsh";
import { Select as BaseSelect } from "nsh/base";
import { Icon } from "./Icon";

/** The app's popup surface, shared by selects, menus and popovers. The `popover`
 *  tone is the card's in the light theme, so the frame and the shadow are what
 *  make a list read as a panel laid over the page. */
export const POPUP_CLASS =
  "min-w-40 rounded-(--radius) border border-input bg-popover p-1 text-popover-foreground shadow-xl";

/** One row of a popup list: the height, the inset and the wash the pointer and
 *  the keys take, and the chosen row's tint (the card's own selected tint, so a
 *  list and a card cannot mark the current item differently). `group` lets the
 *  row's content follow its state (the check on the chosen one). */
const ITEM_CLASS =
  "group min-h-6 w-full flex-row items-center rounded-(--radius-sm) px-2 text-xs text-foreground " +
  "focus:bg-accent focus:text-accent-foreground selected:bg-brand/10";

/** The rows of a popup list: 2pt apart, so a list never reads as one block. */
const LIST_CLASS = "w-full flex-col gap-0.5";

/** A frameless trigger: the value and its chevron, inside the caller's chip. */
const COMPACT_CLASS =
  "min-h-5 flex-row items-center gap-1 rounded-(--radius-sm) px-1 " +
  "focus-visible:outline-2 focus-visible:outline-offset-0 focus-visible:outline-ring";

/** The trigger's frame: the same height and border as an input, so text in a
 *  field and in the list it opens start on one line. */
const TRIGGER_CLASS =
  "min-h-5 w-full flex-row items-center justify-between gap-2 rounded-(--radius) " +
  "border border-input bg-card px-2 py-1 transition-colors duration-150 " +
  "hover:bg-accent active:bg-secondary " +
  "focus-visible:outline-2 focus-visible:outline-offset-0 focus-visible:outline-ring";

export function Select<T extends string>(props: {
  items: { value: T; label: string }[];
  value: T;
  onChange: (value: T) => void;
  /** What screen readers call it: what the choice is about. */
  accessibilityLabel?: string;
  disabled?: boolean;
  /** A trigger without a frame, for a select that sits inside the app's own
   *  chip (a task card's parameter). */
  compact?: boolean;
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
      itemClass={ITEM_CLASS}
      listClass={LIST_CLASS}
      offset={4}
      renderItem={(entry) => (
        <View class="flex-row items-center gap-2">
          <Icon
            name="check"
            size={12}
            color={token("brand")}
            class="opacity-0 group-selected:opacity-100"
          />
          <Text class="text-xs text-foreground">{entry.title}</Text>
        </View>
      )}
      trigger={
        <View
          class={`${props.compact ? COMPACT_CLASS : TRIGGER_CLASS} ${props.class ?? ""}`}
        >
          <Text class="text-xs text-foreground">{label()}</Text>
          <Icon name="chevronDown" size={12} />
        </View>
      }
    />
  );
}
