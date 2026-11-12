// The shop section's second half: action restrictions, fusion, the tier-4
// follow-up, refresh limits and the floors whose shops are ignored.
import { View } from "nsh";
import { Button } from "../../../components/Button";
import { Input } from "../../../components/Input";
import { Select } from "../../../components/Select";
import { Switch } from "../../../components/Switch";
import {
  afterLevelLabel,
  setMirrorBool,
  setMirrorNumber,
  toggleIgnoreShop,
} from "../../../teams";
import type { TeamMirrorConfig } from "../../../teams";
import {
  ControlRow,
  EditorCard,
  Hint,
  OptionCell,
  OptionGrid,
  SectionTitle,
} from "./parts";

const AFTER_LEVEL = [0, 1, 2, 3].map((value) => ({
  value: String(value),
  label: afterLevelLabel(value),
}));

/** A switch row whose value is one mirror boolean. */
function RestrictionRow(props: {
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

/** The editor's shop options: everything after the discard grid. */
export function ShopOptions(props: { config: () => TeamMirrorConfig }) {
  return (
    <View class="flex-col gap-4">
      <EditorCard title="基础操作限制">
        <Hint>限制会在镜牢商店阶段按队伍配置执行。</Hint>
        <OptionGrid>
          <RestrictionRow
            label="不治疗"
            field="do_not_heal"
            checked={() => props.config().do_not_heal}
          />
          <RestrictionRow
            label="不购买"
            field="do_not_buy"
            checked={() => props.config().do_not_buy}
          />
          <RestrictionRow
            label="不合成"
            field="do_not_fuse"
            checked={() => props.config().do_not_fuse}
          />
          <RestrictionRow
            label="不出售"
            field="do_not_sell"
            checked={() => props.config().do_not_sell}
          />
          <RestrictionRow
            label="不升级"
            field="do_not_enhance"
            checked={() => props.config().do_not_enhance}
          />
        </OptionGrid>
      </EditorCard>

      <EditorCard title="合成策略">
        <OptionGrid>
          <RestrictionRow
            label="仅激进合成"
            field="only_aggressive_fuse"
            checked={() => props.config().only_aggressive_fuse}
          />
          <RestrictionRow
            label="不合成体系饰品"
            field="do_not_system_fuse"
            checked={() => props.config().do_not_system_fuse}
          />
          <RestrictionRow
            label="仅合成体系饰品"
            field="only_system_fuse"
            checked={() => props.config().only_system_fuse}
          />
          <RestrictionRow
            label="激进模式也升级"
            field="aggressive_also_enhance"
            checked={() => props.config().aggressive_also_enhance}
          />
          <RestrictionRow
            label="激进模式保留体系"
            field="aggressive_save_systems"
            checked={() => props.config().aggressive_save_systems}
          />
        </OptionGrid>
      </EditorCard>

      <EditorCard>
        <View class="flex-col gap-2">
          <ControlRow
            label="四级饰品后执行策略"
            control={
              <Switch
                checked={props.config().after_level_IV}
                accessibilityLabel="四级饰品后执行策略"
                onChange={({ checked }) => setMirrorBool("after_level_IV", checked)}
              />
            }
          />
          <If when={props.config().after_level_IV}>
            <Select<string>
              accessibilityLabel="四级饰品后执行策略"
              items={AFTER_LEVEL}
              value={String(props.config().after_level_IV_select)}
              onChange={(next) => setMirrorNumber("after_level_IV_select", Number(next))}
              class="w-45"
            />
          </If>
        </View>
      </EditorCard>

      <EditorCard>
        <SectionTitle>商店刷新上限</SectionTitle>
        <OptionGrid>
          <OptionCell>
            <ControlRow
              label="关键词刷新"
              control={
                <Input
                  value={String(props.config().max_keyword_refresh)}
                  accessibilityLabel="关键词刷新上限"
                  onChange={({ value }) =>
                    setMirrorNumber("max_keyword_refresh", Number(value))
                  }
                  class="w-20"
                />
              }
            />
          </OptionCell>
          <OptionCell>
            <ControlRow
              label="普通刷新"
              control={
                <Input
                  value={String(props.config().max_normal_refresh)}
                  accessibilityLabel="普通刷新上限"
                  onChange={({ value }) =>
                    setMirrorNumber("max_normal_refresh", Number(value))
                  }
                  class="w-20"
                />
              }
            />
          </OptionCell>
        </OptionGrid>
      </EditorCard>

      <EditorCard>
        <SectionTitle>忽略商店楼层</SectionTitle>
        <View class="flex-row flex-wrap gap-2">
          <Each of={[0, 1, 2, 3, 4]}>
            {(floor) => (
              <Button
                label={
                  props.config().ignore_shop[floor()] ? `已忽略 ${floor() + 1}F` : `${floor() + 1}F`
                }
                variant={props.config().ignore_shop[floor()] ? "danger" : "outline"}
                selected={props.config().ignore_shop[floor()]}
                size="xs"
                onPress={() => toggleIgnoreShop(floor())}
              />
            )}
          </Each>
        </View>
      </EditorCard>
    </View>
  );
}
