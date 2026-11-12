// Routes the current page id to its page. Each page owns its own body, its
// dialogs and the work it starts; this file only picks one.
import { View } from "nsh";
import type { PageId } from "../store";
import { Gallery } from "./Gallery";
import { Placeholder } from "./Placeholder";

export function PageHost(props: { page: PageId }) {
  return (
    <View class="min-h-0 flex-1">
      <If when={props.page === "gallery"}>
        <Gallery />
      </If>
      <If when={props.page !== "gallery"}>
        <Placeholder page={props.page} />
      </If>
    </View>
  );
}
