// The app's page frame: the outer surface every page sits in, and the toolbar
// card that page-level action bars use.
//
// `page_root` in the GPUI client paints no background on purpose: the window
// root owns `background` and the skin's artwork layer sits between the two. The
// 10pt inset is kept so every page shares the console's content rhythm.
import type { JSX } from "nsh";
import { View } from "nsh";
import { Card } from "./Card";

/** The outer surface of a top-level page: a 10pt inset and one gap between
 *  the page's regions. */
export function PageRoot(props: { children?: JSX.Element; class?: string }) {
  return (
    <View class={`size-full min-h-0 min-w-0 flex-col gap-2 p-2.5 ${props.class ?? ""}`}>
      {props.children}
    </View>
  );
}

/** The card around a page's action bar. */
export function PageToolbar(props: { children?: JSX.Element; class?: string }) {
  return (
    <Card class={`w-full p-3 ${props.class ?? ""}`}>
      {props.children}
    </Card>
  );
}
