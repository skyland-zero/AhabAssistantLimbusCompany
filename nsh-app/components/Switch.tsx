// The app's switch: an on/off setting that takes effect at once.
//
// `nsh/base`'s `Switch` owns the presses, the keyboard activation, the switch
// role and its state for screen readers; this file styles the track and thumb
// and says how the thumb moves. Both parts must be styled: an unstyled switch
// draws nothing and its test still passes.
//
// The track is a row on purpose: a view lays its children out in a column
// unless a class says otherwise, and a column's `items-center` would centre the
// thumb that `checked:translate-x-4` then pushes out of the track. The padding
// and the 1pt border make the 20pt track hold a 16pt thumb with 2pt on every
// side, so the same 16pt travel lands it at the other inset.
//
// The track carries the brand: `brand-light` off, `brand` on. The thumb is the
// light surface in the light appearance (white off, brand-foreground on) and
// flips its contrast in the dark: a light knob over the deep wine off track, a
// dark knob over the light brand on track. This is the reference switch's
// pale-pink / solid-crimson pair, kept at the app's 36×20 control scale.
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
      trackClass="h-5 w-9 flex-none flex-row items-center rounded-full border border-transparent bg-brand-light p-px transition-colors duration-150 checked:bg-brand disabled:opacity-50 motion-reduce:transition-none"
      thumbClass="size-4 flex-none rounded-full bg-card dark:bg-foreground checked:bg-brand-foreground dark:checked:bg-brand-foreground shadow-sm transition-transform duration-150 checked:translate-x-4 motion-reduce:transition-none"
    />
  );
}
