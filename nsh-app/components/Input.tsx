// The app's single-line text input: the field part of every form.
//
// `nsh/base`'s `Input` owns editing, selection, the composition of CJK input and
// the keyboard; this file gives it the app's frame, border and focus edge.
import { Input as BaseInput } from "nsh/base";
import type { InputRef } from "nsh";

const FRAME =
  "min-h-5 rounded-(--radius) border border-input bg-card px-2 py-1 text-xs text-foreground " +
  "placeholder:text-muted-foreground focus:outline-2 focus:outline-offset-0 focus:outline-ring " +
  "disabled:opacity-50";

export function Input(props: {
  /** The value (controlled): the owner sets it in `onChange`. */
  value: string;
  placeholder?: string;
  /** What screen readers call the field, when no visible label names it. */
  accessibilityLabel?: string;
  disabled?: boolean;
  /** Masks the text (a key or a token). */
  masked?: boolean;
  onChange?: (event: { value: string }) => void;
  onSubmit?: (event: { text: string }) => void;
  /** The id UI tests find the field by. */
  testID?: string;
  ref?: InputRef;
  class?: string;
}) {
  return (
    <BaseInput
      ref={props.ref}
      value={props.value}
      placeholder={props.placeholder}
      accessibilityLabel={props.accessibilityLabel}
      disabled={props.disabled}
      masked={props.masked}
      onChange={props.onChange}
      onSubmit={props.onSubmit}
      testID={props.testID}
      class={`${FRAME} ${props.class ?? ""}`}
    />
  );
}
