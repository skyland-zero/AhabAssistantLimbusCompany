// The run bar's "when the queue finishes" editor: the exit actions (any of
// them, in any combination), the power action that follows, and whether the
// choice becomes the saved default or holds for this run only.
//
// The editor works on a draft, as the client's did: pressing the switches
// changes nothing until one of the two save buttons commits it.
import { Text, View, effect, signal } from "nsh";
import { Dialog } from "nsh/base";
import { Button } from "../../components/Button";
import { SettingLine, SettingList } from "../../components/Field";
import { Select } from "../../components/Select";
import { Switch } from "../../components/Switch";
import {
  AFTER_EXIT_ACTIONS,
  AFTER_POWER_ACTIONS,
  afterCompletionLabel,
  afterCompletionMode,
  afterCompletionSettings,
  closeAfterCompletion,
  isAfterCompletionOpen,
  saveAfterCompletion,
} from "../../home";
import type { AfterCompletion } from "../../home";

const DIALOG_CLASS =
  "m-auto w-120 max-w-full rounded-(--radius-lg) border border-border bg-popover p-0 shadow-lg";

export function AfterCompletionDialog() {
  const draft = signal<AfterCompletion>(afterCompletionSettings());
  // Each opening starts from what the task config holds; the presses below
  // change only the draft until a save.
  effect(() => {
    if (isAfterCompletionOpen()) draft.set(afterCompletionSettings());
  });
  const toggleAction = (value: string) => {
    const current = draft();
    const actions = current.actions.includes(value)
      ? current.actions.filter((action) => action !== value)
      : [...current.actions, value];
    draft.set({ ...current, actions });
  };
  return (
    <Dialog
      open={isAfterCompletionOpen()}
      onOpenChange={({ open }) => {
        if (!open) closeAfterCompletion();
      }}
      closeOnEscape
      closeOnBackdropPress
      class={DIALOG_CLASS}
      backdropClass="bg-scrim"
      accessibilityLabel="结束后操作"
    >
      <View class="h-full min-h-0 flex-col">
        <View class="w-full flex-none flex-row items-center justify-between gap-3 border-b border-border px-5 py-3">
          <Text class="text-base font-semibold text-popover-foreground">结束后操作</Text>
          <Button
            label="关闭"
            variant="ghost"
            size="xs"
            testID="after-completion-close"
            onPress={() => closeAfterCompletion()}
          />
        </View>
        <View class="w-full flex-col gap-4 p-5">
          <View class="w-full flex-col gap-2">
            <Text class="text-xs text-muted-foreground">退出动作（可多选）</Text>
            <SettingList>
              <Each of={AFTER_EXIT_ACTIONS} key={(action) => action.value}>
                {(action) => (
                  <SettingLine label={action().label}>
                    <Switch
                      checked={draft().actions.includes(action().value)}
                      accessibilityLabel={action().label}
                      onChange={() => toggleAction(action().value)}
                    />
                  </SettingLine>
                )}
              </Each>
            </SettingList>
          </View>
          <View class="w-full flex-col gap-2">
            <Text class="text-xs text-muted-foreground">最终电源动作</Text>
            <SettingLine label="电源动作">
              <Select<string>
                accessibilityLabel="电源动作"
                items={AFTER_POWER_ACTIONS}
                value={draft().power}
                onChange={(next) => draft.set({ ...draft(), power: next })}
                class="w-36"
              />
            </SettingLine>
          </View>
          <Text class="text-xs text-muted-foreground">{`${afterCompletionLabel(draft())}（${afterCompletionMode(draft())}）`}</Text>
        </View>
        <View class="w-full flex-row items-center justify-end gap-2 border-t border-border px-5 py-3">
          <Button
            label="仅本次生效"
            variant="outline"
            testID="after-completion-once"
            onPress={() => saveAfterCompletion({ ...draft(), keep: false })}
          />
          <Button
            label="保存为默认"
            testID="after-completion-save"
            onPress={() => saveAfterCompletion({ ...draft(), keep: true })}
          />
        </View>
      </View>
    </Dialog>
  );
}
