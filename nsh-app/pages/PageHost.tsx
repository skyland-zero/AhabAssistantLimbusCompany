// Routes the current page id to its page. Each page owns its own body, its
// dialogs and the work it starts; this file only picks one.
import { View } from "nsh";
import type { PageId } from "../store";
import { Gallery } from "./Gallery";
import { Placeholder } from "./Placeholder";
import { Help } from "./help/Help";
import { Teams } from "./teams/Teams";
import { Resources } from "./resources/Resources";
import { Settings } from "./settings/Settings";
import { Toolbox } from "./toolbox/Toolbox";

/** The pages that are still placeholders. */
function placed(page: PageId): boolean {
  return (
    page !== "settings" &&
    page !== "toolbox" &&
    page !== "resources" &&
    page !== "teams" &&
    page !== "help" &&
    page !== "gallery"
  );
}

export function PageHost(props: { page: PageId }) {
  return (
    <View class="min-h-0 flex-1">
      <If when={props.page === "settings"}>
        <Settings />
      </If>
      <If when={props.page === "toolbox"}>
        <Toolbox />
      </If>
      <If when={props.page === "resources"}>
        <Resources />
      </If>
      <If when={props.page === "teams"}>
        <Teams />
      </If>
      <If when={props.page === "help"}>
        <Help />
      </If>
      <If when={props.page === "gallery"}>
        <Gallery />
      </If>
      <If when={placed(props.page)}>
        <Placeholder page={props.page} />
      </If>
    </View>
  );
}
