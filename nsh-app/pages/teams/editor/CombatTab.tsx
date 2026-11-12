// The editor's combat section: the second gift system, the combat/skill
// preferences, the defense strategy and skill replacement.
import { View } from "nsh";
import { Select } from "../../../components/Select";
import { Switch } from "../../../components/Switch";
import {
  SYSTEM_LABELS,
  SYSTEM_NAMES,
  setMirrorBool,
  setMirrorNumber,
  skillReplacementModeLabel,
  skillReplacementSelectLabel,
  turnsLabel,
} from "../../../teams";
import type { TeamMirrorConfig } from "../../../teams";
import {
  ControlRow,
  EditorCard,
  Hint,
  OptionCell,
  OptionGrid,
} from "./parts";

const SYSTEMS = SYSTEM_NAMES.map((name, value) => ({
  value: String(value),
  label: SYSTEM_LABELS[name],
}));

const FLOORS = [0, 1].map((value) => ({
  value: String(value),
  label: value === 0 ? "主体系四级前" : "主体系四级后",
}));

const TURNS = [1, 2, 3, 4, 5].map((value) => ({
  value: String(value),
  label: turnsLabel(value),
}));

const REPLACEMENT_TARGETS = [0, 1, 2, 3].map((value) => ({
  value: String(value),
  label: skillReplacementSelectLabel(value),
}));

const REPLACEMENT_MODES = [0, 1, 2, 3, 4].map((value) => ({
  value: String(value),
  label: skillReplacementModeLabel(value),
}));

/** A switch row whose value is one mirror boolean. */
function BoolRow(props: {
  label: string;
  field: keyof TeamMirrorConfig;
  checked: () => boolean;
}) {
  return (
    <OptionCell>
      <ControlRow
        label={props.label}
        control={
          <Switch
            checked={props.checked()}
            accessibilityLabel={props.label}
            onChange={({ checked }) => setMirrorBool(props.field, checked)}
          />
        }
      />
    </OptionCell>
  );
}

export function CombatTab(props: { config: () => TeamMirrorConfig }) {
  return (
    <View class="flex-col gap-4">
      <EditorCard>
        <View class="flex-col gap-2">
          <ControlRow
            label="启用第二体系"
            control={
              <Switch
                checked={props.config().second_system}
                accessibilityLabel="启用第二体系"
                onChange={({ checked }) => setMirrorBool("second_system", checked)}
              />
            }
          />
          <If when={props.config().second_system}>
            <OptionGrid>
              <OptionCell>
                <ControlRow
                  label="第二体系"
                  control={
                    <Select<string>
                      accessibilityLabel="第二体系"
                      items={SYSTEMS}
                      value={String(props.config().second_system_select)}
                      onChange={(next) =>
                        setMirrorNumber("second_system_select", Number(next))
                      }
                      class="w-40"
                    />
                  }
                />
              </OptionCell>
              <OptionCell>
                <ControlRow
                  label="起始楼层"
                  control={
                    <Select<string>
                      accessibilityLabel="第二体系起始楼层"
                      items={FLOORS}
                      value={String(props.config().second_system_setting)}
                      onChange={(next) =>
                        setMirrorNumber("second_system_setting", Number(next))
                      }
                      class="w-40"
                    />
                  }
                />
              </OptionCell>
              <BoolRow
                label="四级合成"
                field="second_system_fuse_IV"
                checked={() => props.config().second_system_fuse_IV}
              />
              <BoolRow
                label="购买饰品"
                field="second_system_buy"
                checked={() => props.config().second_system_buy}
              />
              <BoolRow
                label="选择奖励"
                field="second_system_select_reward"
                checked={() => props.config().second_system_select_reward}
              />
              <BoolRow
                label="升级饰品"
                field="second_system_power_up"
                checked={() => props.config().second_system_power_up}
              />
            </OptionGrid>
          </If>
          <If when={!props.config().second_system}>
            <Hint>开启后可配置第二体系和起始楼层。</Hint>
          </If>
        </View>
      </EditorCard>

      <EditorCard title="战斗与技能偏好">
        <OptionGrid>
          <BoolRow
            label="避免三技能"
            field="avoid_skill_3"
            checked={() => props.config().avoid_skill_3}
          />
          <BoolRow
            label="优先三技能"
            field="prioritize_skill_3"
            checked={() => props.config().prioritize_skill_3}
          />
          <BoolRow
            label="每层重新编队"
            field="re_formation_each_floor"
            checked={() => props.config().re_formation_each_floor}
          />
          <BoolRow
            label="启用伤害P"
            field="use_damage_p"
            checked={() => props.config().use_damage_p}
          />
        </OptionGrid>
      </EditorCard>

      <EditorCard title="防御策略（互斥）">
        <OptionGrid>
          <BoolRow
            label="首回合防御"
            field="defense_first_round"
            checked={() => props.config().defense_first_round}
          />
          <BoolRow
            label="良秀单通防御"
            field="defense_for_solo"
            checked={() => props.config().defense_for_solo}
          />
          <If when={props.config().defense_for_solo}>
            <OptionCell>
              <ControlRow
                label="防御回合"
                control={
                  <Select<string>
                    accessibilityLabel="防御回合"
                    items={TURNS}
                    value={String(props.config().defense_for_solo_turns)}
                    onChange={(next) =>
                      setMirrorNumber("defense_for_solo_turns", Number(next))
                    }
                    class="w-40"
                  />
                }
              />
            </OptionCell>
          </If>
        </OptionGrid>
      </EditorCard>

      <EditorCard>
        <View class="flex-col gap-2">
          <ControlRow
            label="启用技能替换"
            control={
              <Switch
                checked={props.config().skill_replacement}
                accessibilityLabel="启用技能替换"
                onChange={({ checked }) => setMirrorBool("skill_replacement", checked)}
              />
            }
          />
          <If when={props.config().skill_replacement}>
            <OptionGrid>
              <OptionCell>
                <ControlRow
                  label="替换目标"
                  control={
                    <Select<string>
                      accessibilityLabel="技能替换目标"
                      items={REPLACEMENT_TARGETS}
                      value={String(props.config().skill_replacement_select)}
                      onChange={(next) =>
                        setMirrorNumber("skill_replacement_select", Number(next))
                      }
                      class="w-40"
                    />
                  }
                />
              </OptionCell>
              <OptionCell>
                <ControlRow
                  label="替换模式"
                  control={
                    <Select<string>
                      accessibilityLabel="技能替换模式"
                      items={REPLACEMENT_MODES}
                      value={String(props.config().skill_replacement_mode)}
                      onChange={(next) =>
                        setMirrorNumber("skill_replacement_mode", Number(next))
                      }
                      class="w-40"
                    />
                  }
                />
              </OptionCell>
            </OptionGrid>
          </If>
        </View>
      </EditorCard>
    </View>
  );
}
