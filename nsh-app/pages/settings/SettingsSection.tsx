// One settings section: a page-level heading, a visible rule, then the groups
// of settings that belong to it. B deliberately paints no card here — the
// sections sit on the page background and the tonal `SettingsGroup` surfaces
// carry the grouping, as on the console's expanded task settings.
import type { JSX } from "nsh";
import { Text, View } from "nsh";
import { Rule } from "../../components/Card";

export function SettingsSection(props: { title: string; children?: JSX.Element }) {
  return (
    <View class="w-full flex-col">
      <View role="header" class="w-full flex-row items-center gap-2">
        <Text class="text-sm font-semibold text-foreground">{props.title}</Text>
      </View>
      <Rule class="mt-2" />
      <View class="w-full flex-col gap-4 pt-3">{props.children}</View>
    </View>
  );
}
