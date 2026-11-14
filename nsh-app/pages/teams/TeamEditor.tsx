// The team editor: one overlay with the client's five sections. Basic covers
// what the team is and who is in it; shop, combat, starlight and advanced are
// the mirror-run configuration; an experience-dungeon team only has basic.
//
// Everything is edited on a draft; Save writes it back to the list (P1 sends
// it to `Teams.*` instead). The section is screen-level state, so it lives in
// `teams.ts` and resets whenever the overlay opens.
import { Clipboard, Scroll, Text, View } from "nsh";
import { Dialog } from "nsh/base";
import { Button } from "../../components/Button";
import {
  EDITOR_TAB_LABELS,
  activeEditorTab,
  closeEditor,
  draftJson,
  draftStarlightCost,
  draftMirror,
  editorIsNew,
  editorTabs,
  editingTeam,
  saveDraft,
  setEditorTab,
  setTeamsFeedback,
  teamsFeedback,
  teamsFeedbackTone,
} from "../../teams";
import type { Team, TeamEditorTab } from "../../teams";
import { AdvancedTab } from "./editor/AdvancedTab";
import { BasicTab } from "./editor/BasicTab";
import { CombatTab } from "./editor/CombatTab";
import { ShopTab } from "./editor/ShopTab";
import { StarlightTab } from "./editor/StarlightTab";

export function TeamEditor() {
  return (
    <Dialog
      open={editingTeam() !== null}
      onOpenChange={({ open }) => {
        if (!open) closeEditor();
      }}
      closeOnEscape
      closeOnBackdropPress
      class="m-auto h-150 max-h-full w-170 max-w-full rounded-(--radius-lg) border border-border bg-popover p-0 shadow-lg"
      backdropClass="bg-scrim"
      accessibilityLabel="队伍编辑器"
    >
      <If when={editingTeam()}>
        {(team) => (
          <View class="h-full min-h-0 flex-col">
            <View class="w-full flex-none flex-row items-center justify-between gap-3 border-b border-border px-6 py-3">
              <View class="min-w-0 flex-col gap-1">
                <Text class="text-base font-semibold text-popover-foreground">
                  {editorIsNew()
                    ? `新建编队 #${String(team().id.replace("team-", "")).padStart(2, "0")}`
                    : `编辑编队 #${String(team().id.replace("team-", "")).padStart(2, "0")}`}
                </Text>
                <Text class="text-[10px] text-muted-foreground">
                  保存前所有修改只存在于当前编辑器
                </Text>
              </View>
              <Button
                label="复制 JSON"
                icon="copy"
                variant="outline"
                onPress={() => {
                  Clipboard.writeText(draftJson()).then(() =>
                    setTeamsFeedback("队伍 JSON 已复制"),
                  );
                }}
              />
            </View>

            <View class="w-full flex-none border-b border-border bg-muted px-6 py-2">
              <View class="flex-row flex-wrap items-center gap-1 rounded-(--radius) bg-muted p-0.5">
                <Each of={editorTabs(team().purpose)}>
                  {(tab) => (
                    <TabButton tab={tab()} current={() => activeEditorTab()} />
                  )}
                </Each>
              </View>
            </View>

            <Scroll class="min-h-0 flex-1 bg-background">
              <View class="w-full flex-col px-6 py-4">
                <TabBody team={team} />
              </View>
            </Scroll>

            <View class="w-full flex-none flex-row items-center justify-between gap-3 border-t border-border bg-card px-6 py-3">
              <View class="min-w-0 flex-1">
                <If when={teamsFeedback()}>
                  {(message) => (
                    <Text class={`text-xs ${feedbackClass(teamsFeedbackTone())}`}>{message()}</Text>
                  )}
                </If>
                <If when={!teamsFeedback()}>
                  <Text class="text-xs text-muted-foreground">
                    支持中文输入、剪贴板和 JSON 导入
                  </Text>
                </If>
              </View>
              <View class="flex-row flex-none gap-2">
                <Button label="取消" variant="ghost" onPress={() => closeEditor()} />
                <Button
                  label="保存队伍"
                  testID="save-team"
                  disabled={team().name.trim() === ""}
                  onPress={() => saveDraft()}
                />
              </View>
            </View>
          </View>
        )}
      </If>
    </Dialog>
  );
}

/** The footer's feedback colour: errors are not success messages. */
function feedbackClass(tone: "info" | "success" | "warning" | "danger"): string {
  if (tone === "danger") return "text-danger";
  if (tone === "warning") return "text-warning";
  if (tone === "success") return "text-success";
  return "text-muted-foreground";
}

/** One section tab; the starlight tab carries the current cost as a suffix. */
function TabButton(props: {
  tab: TeamEditorTab;
  current: () => TeamEditorTab;
}) {
  const selected = () => props.current() === props.tab;
  return (
    <Button
      label={EDITOR_TAB_LABELS[props.tab]}
      variant={selected() ? "outline" : "ghost"}
      selected={selected()}
      testID={`team-tab-${props.tab}`}
      onPress={() => setEditorTab(props.tab)}
      class={
        selected()
          ? "border-transparent bg-card text-foreground shadow-sm hover:bg-card"
          : ""
      }
    >
      <If when={props.tab === "starlight" && draftStarlightCost() > 0}>
        <View class="rounded-(--radius-sm) bg-brand-light px-1">
          <Text class="text-[10px] text-brand">{draftStarlightCost()}</Text>
        </View>
      </If>
    </Button>
  );
}

/** The active section's body. The fallback keeps a stale tab harmless when a
 *  team changes purpose while the editor is open. */
function TabBody(props: { team: () => Team }) {
  const current = () => {
    const available = editorTabs(props.team().purpose);
    const tab = activeEditorTab();
    return available.includes(tab) ? tab : "basic";
  };
  return (
    <View class="flex-col">
      <If when={current() === "basic"}>
        <BasicTab team={props.team} />
      </If>
      <If when={current() === "shop"}>
        <ShopTab config={() => draftMirror()} />
      </If>
      <If when={current() === "combat"}>
        <CombatTab config={() => draftMirror()} />
      </If>
      <If when={current() === "starlight"}>
        <StarlightTab />
      </If>
      <If when={current() === "advanced"}>
        <AdvancedTab />
      </If>
    </View>
  );
}
