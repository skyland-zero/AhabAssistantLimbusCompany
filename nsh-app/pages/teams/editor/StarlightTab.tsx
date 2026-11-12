// The editor's starlight section: whether opening bonuses apply, a one-click
// level for all ten, and each bonus's level and cost.
import { Text, View, token } from "nsh";
import { Button } from "../../../components/Button";
import { Icon } from "../../../components/Icon";
import { Switch } from "../../../components/Switch";
import {
  STARLIGHT_COSTS,
  STARLIGHT_DESCRIPTIONS,
  STARLIGHT_NAMES,
  draftMirror,
  draftOpeningBonus,
  draftStarlightCost,
  setAllStarlight,
  setMirrorBool,
  setStarlightLevel,
  starlightLevelLabel,
  starlightShortLevel,
} from "../../../teams";
import { ControlRow, EditorCard, Hint } from "./parts";

const LEVELS = [0, 1, 2, 3];
const INDICES = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9];

/** A level chip: selected draws the client's brand outline, not a filled one. */
function LevelChip(props: {
  label: string;
  selected: boolean;
  onPress: () => void;
}) {
  return (
    <Button
      label={props.label}
      variant="outline"
      size="xs"
      selected={props.selected}
      onPress={props.onPress}
      class={
        props.selected
          ? "border-brand bg-brand-light text-brand font-medium hover:bg-brand-light"
          : "min-w-8"
      }
    />
  );
}

export function StarlightTab() {
  return (
    <View class="flex-col gap-3">
      <EditorCard>
        <View class="flex-col gap-1">
          <ControlRow
            label="使用开局星光"
            control={
              <Switch
                checked={draftMirror().use_starlight}
                accessibilityLabel="使用开局星光"
                onChange={({ checked }) => setMirrorBool("use_starlight", checked)}
              />
            }
          />
          <Hint>开局时按下方等级消耗星光点数。</Hint>
        </View>
      </EditorCard>

      <EditorCard>
        <View class="w-full flex-row flex-wrap items-center justify-between gap-3">
          <View class="flex-row flex-wrap items-center gap-2">
            <Text class="text-xs text-muted-foreground">一键设置等级</Text>
            <View class="flex-row items-center gap-1">
              <Each of={LEVELS}>
                {(level) => (
                  <Button
                    label={starlightLevelLabel(level())}
                    variant="outline"
                    size="xs"
                    onPress={() => setAllStarlight(level())}
                  />
                )}
              </Each>
            </View>
          </View>
          <View class="flex-row items-center gap-1 rounded-(--radius) bg-brand-light px-2 py-1">
            <Icon name="sparkles" size={13} color={token("brand")} />
            <Text class="text-xs text-brand">总消耗 {draftStarlightCost()} 点</Text>
          </View>
        </View>
      </EditorCard>

      <Each of={INDICES}>
        {(index) => (
          <EditorCard>
            <View class="w-full flex-row items-center justify-between gap-2">
              <Text class="text-xs text-foreground">{STARLIGHT_NAMES[index()]}</Text>
              <View class="flex-row items-center gap-1">
                <Icon name="sparkles" size={11} color={token("brand")} />
                <Text class="text-[10px] text-muted-foreground">
                  {STARLIGHT_COSTS[index()]} 点
                </Text>
              </View>
            </View>
            <View class="flex-row gap-1">
              <Each of={LEVELS}>
                {(level) => (
                  <LevelChip
                    label={starlightShortLevel(level())}
                    selected={draftOpeningBonus()[index()] === level()}
                    onPress={() => setStarlightLevel(index(), level())}
                  />
                )}
              </Each>
            </View>
            <Hint>{STARLIGHT_DESCRIPTIONS[index()]}</Hint>
          </EditorCard>
        )}
      </Each>
    </View>
  );
}
