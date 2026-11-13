// The window's frame: wraps the application card. When floating, it rounds all
// four corners with `rounded-(--radius-xl)`; when maximized or in full screen,
// the corners flatten to fill the display.
import type { JSX } from "nsh";
import { View, effect } from "nsh";
import { windowState } from "nsh/desktop";
import { Runtime } from "aalc/native";

/** True while the card fills the window: maximized or in full screen, where
 *  the screen's own edges are the corners. Off desktops there is no window
 *  state, and nothing to fill. */
function filled(): boolean {
  const state = windowState();
  return state !== null && state.mode !== "normal";
}

export function WindowFrame(props: { children?: JSX.Element }) {
  const isFilled = filled();

  effect(() => {
    if (Runtime.platformOs() === "windows") {
      Runtime.applyRoundedCorners(isFilled ? 0 : 20);
    }
  });

  return (
    <View
      class={`relative flex-1 overflow-hidden bg-background ${
        isFilled ? "" : "rounded-(--radius-xl)"
      }`}
    >
      {props.children}
    </View>
  );
}
