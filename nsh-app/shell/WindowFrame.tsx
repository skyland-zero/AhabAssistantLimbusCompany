// The window's frame: the app's, because `nsh.json` asks for a transparent
// window with no system frame (the only way to round all four corners on
// Windows 10, where the system does not round them, and where a per-pixel
// alpha window is what makes the corners see-through at all).
//
// Two things come with that: the system's shadow is off, so the app draws one,
// and the band around the card is the room it needs. That band is also the
// window's resize border, which is why it is this wide (`p-3`: 12 points).
import type { JSX } from "nsh";
import { View, token } from "nsh";
import { windowState } from "nsh/desktop";

/** True while the card fills the window: maximized or in full screen, where
 *  the screen's own edges are the corners. Off desktops there is no window
 *  state, and nothing to fill. */
function filled(): boolean {
  const state = windowState();
  return state !== null && state.mode !== "normal";
}

/** The card's shadow: short and soft, because the blur that reaches past the
 *  band is cut off by the window's edge. */
const SHADOW = [{ x: 0, y: 6, blur: 18, spread: -6 }];

export function WindowFrame(props: { children?: JSX.Element }) {
  return (
    <View class={filled() ? "flex-1" : "flex-1 p-3"}>
      <View
        class={`relative flex-1 overflow-hidden bg-background ${
          filled() ? "" : "rounded-(--radius-xl)"
        }`}
        style={{
          boxShadow: filled()
            ? undefined
            : SHADOW.map((layer) => ({ ...layer, color: token("window-shadow") })),
        }}
      >
        {props.children}
      </View>
    </View>
  );
}
