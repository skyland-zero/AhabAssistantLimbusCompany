// The app's switch: an on/off setting that takes effect at once.
//
// `nsh/base`'s `Switch` owns the presses, the keyboard activation, the switch
// role and its state for screen readers; this file styles the track and thumb
// and says how the thumb moves. Both parts must be styled: an unstyled switch
// draws nothing and its test still passes.
import { Switch as BaseSwitch } from "nsh/base";

export function Switch(props: {
  checked: boolean;
  disabled?: boolean;
  /** What screen readers say it turns on, when nothing beside it says so. */
  accessibilityLabel?: string;
  onChange?: (event: { checked: boolean }) => void;
  class?: string;
}) {
  return (
    <BaseSwitch
      checked={props.checked}
      disabled={props.disabled}
      accessibilityLabel={props.accessibilityLabel}
      onChange={props.onChange}
      class={`flex-row items-center ${props.class ?? ""}`}
      trackClass="h-5 w-9 flex-none items-center rounded-full border border-transparent bg-input p-0.5 transition-colors duration-150 checked:bg-primary disabled:opacity-50 motion-reduce:transition-none"
      thumbClass="size-4 flex-none rounded-full bg-card shadow-sm transition-transform duration-150 checked:translate-x-4 motion-reduce:transition-none"
    />
  );
}
