// The team editor: the name, purpose, scheme, party and enabled state of one
// formation, in an overlay over the list.
//
// The client's editor had five sections (basic, combat, shop, strategy,
// advanced, starlight, stats). This overlay covers the basic section — what the
// team is, who is in it and whether it runs — and the mirror-config sections
// land with the mirror task work; the overlay is where they go.
import { Text, View } from "nsh";
import { Dialog } from "nsh/base";
import { Button } from "../../components/Button";
import { Input } from "../../components/Input";
import { Segmented } from "../../components/Segmented";
import { Select } from "../../components/Select";
import { Switch } from "../../components/Switch";
import {
  PURPOSES,
  ROSTER,
  SCHEME_LABELS,
  SCHEMES,
  SINNER_NAMES,
  closeEditor,
  editTeam,
  editingTeam,
  saveDraft,
  toggleDraftSinner,
} from "../../teams";
import type { TeamPurpose } from "../../teams";

export function TeamEditor() {
  const draft = () => editingTeam();
  return (
    <Dialog
      open={draft() !== null}
      onOpenChange={({ open }) => {
        if (!open) closeEditor();
      }}
      closeOnEscape
      closeOnBackdropPress
      class="m-auto w-160 rounded-(--radius-lg) border border-border bg-popover p-0 shadow-lg"
      backdropClass="bg-scrim"
    >
      <If when={draft()}>
        {(team) => (
          <View class="flex-col">
            <View class="flex-none flex-row items-center justify-between border-b border-border px-4 py-3">
              <Text class="text-sm font-semibold text-popover-foreground">队伍</Text>
              <Text class="font-mono text-xs text-muted-foreground">{team().id}</Text>
            </View>

            <View class="flex-col gap-3 px-4 py-3">
              <View class="flex-row items-center gap-3">
                <Text class="w-16 flex-none text-xs text-muted-foreground">名称</Text>
                <Input
                  value={team().name}
                  accessibilityLabel="队伍名称"
                  testID="team-name"
                  onChange={({ value }) => editTeam({ name: value })}
                  class="flex-1"
                />
              </View>
              <View class="flex-row items-center gap-3">
                <Text class="w-16 flex-none text-xs text-muted-foreground">用途</Text>
                <Segmented<TeamPurpose>
                  accessibilityLabel="用途"
                  value={team().purpose}
                  onChange={(next) => editTeam({ purpose: next })}
                  options={PURPOSES}
                />
              </View>
              <View class="flex-row items-center gap-3">
                <Text class="w-16 flex-none text-xs text-muted-foreground">饰品体系</Text>
                <Select<string>
                  accessibilityLabel="饰品体系"
                  items={SCHEMES.map((scheme) => ({ value: scheme, label: SCHEME_LABELS[scheme] }))}
                  value={team().accessoryScheme}
                  onChange={(next) => editTeam({ accessoryScheme: next })}
                  class="w-48"
                />
                <View class="flex-row items-center gap-2">
                  <Text class="text-xs text-muted-foreground">启用</Text>
                  <Switch
                    checked={team().enabled}
                    accessibilityLabel="启用队伍"
                    onChange={({ checked }) => editTeam({ enabled: checked })}
                  />
                </View>
              </View>
              <View class="flex-col gap-2">
                <Text class="text-xs text-muted-foreground">人格（{team().sinners.length}）</Text>
                <View class="flex-row flex-wrap gap-1">
                  <Each of={ROSTER}>
                    {(sinner) => (
                      <Button
                        label={SINNER_NAMES[sinner()] ?? sinner()}
                        variant={team().sinners.includes(sinner()) ? "primary" : "outline"}
                        selected={team().sinners.includes(sinner())}
                        size="xs"
                        onPress={() => toggleDraftSinner(sinner())}
                      />
                    )}
                  </Each>
                </View>
              </View>
            </View>

            <View class="flex-none flex-row justify-end gap-2 border-t border-border px-4 py-3">
              <Button label="取消" variant="outline" onPress={() => closeEditor()} />
              <Button
                label="保存"
                onPress={() => saveDraft()}
                disabled={team().name.trim() === ""}
                testID="save-team"
              />
            </View>
          </View>
        )}
      </If>
    </Dialog>
  );
}
