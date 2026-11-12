// The card shell every settings section uses: the active skin's header
// decoration, then a body with one shared inset and one shared row rhythm.
import type { JSX } from "nsh";
import { View } from "nsh";
import { Card } from "../../components/Card";

export function SettingsCard(props: { title: string; children?: JSX.Element }) {
  return (
    <Card title={props.title} class="p-0">
      <View class="flex-col gap-3 px-4 pt-4 pb-4">{props.children}</View>
    </Card>
  );
}
