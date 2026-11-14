// The built-in preset catalog. A preset is a template, not a saved team: the
// picker chooses a target loadout number, then hands the draft to the editor.
import { Dialog } from "nsh/base";
import { Scroll, Text, View, effect, signal } from "nsh";
import { Badge } from "../../components/Badge";
import { Button, Pressable } from "../../components/Button";
import { Select } from "../../components/Select";
import {
  PURPOSE_LABELS,
  applyPreset,
  freeSlots,
  localizedText,
  presets,
  teamNumber,
  type Team,
  type TeamPreset,
} from "../../teams";

/** The preset card's purpose tone. */
function purposeTone(preset: TeamPreset): "accent" | "success" | "neutral" {
  if (preset.team.purpose === "mirror") return "accent";
  if (preset.team.purpose === "luxcavation") return "success";
  return "neutral";
}

/** Whether the template carries a team code. */
function hasCode(preset: TeamPreset): boolean {
  return (
    preset.team.teamCode.trim() !== "" ||
    (preset.team.mirrorConfig?.use_team_code === true &&
      (preset.team.mirrorConfig?.team_code ?? "").trim() !== "")
  );
}

export function PresetPicker(props: {
  open: boolean;
  overwrite: Team | null;
  onClose: () => void;
}) {
  const selected = signal("");
  const slot = signal("");

  const available = (): TeamPreset[] => {
    if (props.overwrite === null) return presets();
    return presets().filter((preset) => preset.team.purpose === props.overwrite?.purpose);
  };

  // Reset the draft whenever the dialog opens for a new target.
  effect(() => {
    if (!props.open) return;
    const candidates = available();
    selected.set(candidates[0]?.presetId ?? "");
    slot.set(freeSlots()[0]?.value ?? "");
  });

  const current = (): TeamPreset | null =>
    available().find((preset) => preset.presetId === selected()) ?? null;

  const canApply = () =>
    current() !== null &&
    (props.overwrite !== null || (slot() !== "" && freeSlots().some((item) => item.value === slot())));

  const apply = () => {
    const preset = current();
    if (preset === null) return;
    if (props.overwrite !== null) {
      applyPreset(preset, teamNumber(props.overwrite.id) ?? 0, props.overwrite);
    } else {
      applyPreset(preset, Number(slot()));
    }
    props.onClose();
  };

  return (
    <Dialog
      open={props.open}
      onOpenChange={({ open }) => {
        if (!open) props.onClose();
      }}
      closeOnEscape
      closeOnBackdropPress
      class="m-auto h-140 max-h-full w-180 max-w-full rounded-(--radius-lg) border border-border bg-popover p-0 shadow-lg"
      backdropClass="bg-scrim"
      accessibilityLabel="从预设编队选择"
    >
      <View class="h-full min-h-0 flex-col">
        <View class="w-full flex-none border-b border-border px-5 py-3">
          <Text class="text-base font-semibold text-popover-foreground">
            {props.overwrite === null
              ? "从预设新建"
              : `从预设覆盖 #${String(teamNumber(props.overwrite.id) ?? 0).padStart(2, "0")}`}
          </Text>
          <Text class="text-[10px] text-muted-foreground">
            {props.overwrite === null
              ? "预设只是模板；选好目标编队后仍会进入编辑器，保存前不会写入。"
              : "覆盖会替换当前队伍配置，编队统计会保留。"}
          </Text>
        </View>

        <Scroll class="min-h-0 flex-1">
          <View class="grid w-full grid-cols-2 gap-3 p-5">
            <Each of={available()} key={(preset) => preset.presetId}>
              {(preset) => {
                const active = () => selected() === preset().presetId;
                return (
                  <Pressable
                    accessibilityLabel={localizedText(preset().name)}
                    selected={active()}
                    testID={`preset-${preset().presetId}`}
                    onPress={() => selected.set(preset().presetId)}
                    class={`min-w-0 flex-col gap-2 rounded-(--radius-lg) border p-3 text-left transition-colors duration-150 ${
                      active()
                        ? "border-brand bg-brand-light"
                        : "border-border bg-card hover:bg-accent active:bg-secondary"
                    }`}
                  >
                    <View class="w-full flex-row items-center gap-2">
                      <Text class="min-w-0 flex-1 truncate text-sm font-semibold text-foreground">
                        {localizedText(preset().name)}
                      </Text>
                      <Badge
                        label={PURPOSE_LABELS[preset().team.purpose]}
                        tone={purposeTone(preset())}
                      />
                    </View>
                    <View class="w-full flex-row flex-wrap items-center gap-1.5">
                      <Badge label={`${preset().team.sinners.length} 人格`} />
                      <If when={hasCode(preset())}>
                        <Badge label="编队码" tone="warning" />
                      </If>
                      <Text class="text-[10px] text-muted-foreground">
                        {localizedText(preset().floorHint)}
                      </Text>
                    </View>
                    <Text class="text-[10px] text-muted-foreground">
                      {localizedText(preset().routeName)}
                    </Text>
                    <Text class="text-xs leading-5 text-foreground">
                      {localizedText(preset().description)}
                    </Text>
                  </Pressable>
                );
              }}
            </Each>
          </View>
        </Scroll>

        <View class="w-full flex-none border-t border-border px-5 py-3">
          <View class="w-full flex-row flex-wrap items-center justify-between gap-3">
            <View class="min-w-0 flex-row items-center gap-2">
              <Text class="flex-none text-xs text-muted-foreground">目标编队</Text>
              <If
                when={props.overwrite === null}
                else={
                  <Badge
                    label={`#${String(teamNumber(props.overwrite?.id ?? "") ?? 0).padStart(2, "0")}`}
                  />
                }
              >
                <Select<string>
                  accessibilityLabel="目标编队"
                  items={freeSlots()}
                  value={slot() || null}
                  onChange={(next) => slot.set(next)}
                  class="w-40"
                />
              </If>
              <If when={props.overwrite === null && freeSlots().length === 0}>
                <Text class="text-xs text-danger">没有空余编队位，请先删除一支队伍。</Text>
              </If>
            </View>
            <View class="flex-row gap-2">
              <Button label="取消" variant="ghost" onPress={() => props.onClose()} />
              <Button label="使用并编辑" disabled={!canApply()} onPress={() => apply()} />
            </View>
          </View>
        </View>
      </View>
    </Dialog>
  );
}
