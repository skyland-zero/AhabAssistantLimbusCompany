// The editor's shop section: the route and theme-pack weight, the
// reward/opening strategy, the shop strategy and the discarded gift systems;
// `ShopOptions` draws the dense settings after them.
import type { JSX } from "nsh";
import { View } from "nsh";
import { Select } from "../../../components/Select";
import { Switch } from "../../../components/Switch";
import {
  ROUTE_OPTIONS,
  SYSTEM_LABELS,
  SYSTEM_NAMES,
  openingItemsOrderLabel,
  rewardCardsLabel,
  routeLabel,
  setDraftRoute,
  shoppingStrategyLabel,
  shopStrategyLabel,
  setMirrorBool,
  setMirrorNumber,
  toggleDiscardSystem,
} from "../../../teams";
import type { TeamMirrorConfig } from "../../../teams";
import {
  ControlRow,
  EditorCard,
  Hint,
  OptionCell,
  OptionGrid,
  SystemChoice,
} from "./parts";
import { ShopOptions } from "./ShopOptions";

const SHOP_STRATEGIES = [0, 1, 2].map((value) => ({
  value: String(value),
  label: shopStrategyLabel(value),
}));

const REWARD_CARDS = [0, 1, 2, 3].map((value) => ({
  value: String(value),
  label: rewardCardsLabel(value),
}));

const SHOPPING_STRATEGIES = [0, 1, 2, 3, 4, 5].map((value) => ({
  value: String(value),
  label: shoppingStrategyLabel(value),
}));

const OPENING_ORDERS = [0, 1, 2, 3, 4, 5].map((value) => ({
  value: String(value),
  label: openingItemsOrderLabel(value),
}));

const OPENING_SYSTEMS = SYSTEM_NAMES.map((name, value) => ({
  value: String(value),
  label: SYSTEM_LABELS[name],
}));

/** A switch row for one boolean field. */
function SwitchCard(props: {
  label: string;
  field: keyof TeamMirrorConfig;
  checked: () => boolean;
  children?: JSX.Element;
}) {
  return (
    <EditorCard>
      <View class="flex-col gap-2">
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
        {props.children}
      </View>
    </EditorCard>
  );
}

export function ShopTab(props: { config: () => TeamMirrorConfig }) {
  return (
    <View class="flex-col gap-4">
      <EditorCard title="镜牢路线与主题包">
        <View class="flex-col gap-2">
          <ControlRow
            label="镜牢路线"
            control={
              <Select<string>
                accessibilityLabel="镜牢路线"
                items={ROUTE_OPTIONS}
                value={props.config().mirror_route_profile}
                onChange={(next) => setDraftRoute(next)}
                class="w-60"
              />
            }
          />
          <Hint>
            {`${routeLabel(props.config().mirror_route_profile)}：按路线目录优先选择主题包与目标饰品；留空使用默认饰品逻辑。`}
          </Hint>
          <ControlRow
            label="队伍专属主题包权重"
            control={
              <Switch
                checked={props.config().use_custom_theme_pack_weight}
                accessibilityLabel="队伍专属主题包权重"
                onChange={({ checked }) =>
                  setMirrorBool("use_custom_theme_pack_weight", checked)
                }
              />
            }
          />
          <Hint>开启后由镜牢执行器读取该队伍的主题包权重文件。</Hint>
        </View>
      </EditorCard>

      <EditorCard title="奖励与开局策略">
        <View class="flex-col gap-2">
          <SwitchCard
            label="奖励卡优先度"
            field="reward_cards"
            checked={() => props.config().reward_cards}
          >
            <If when={props.config().reward_cards}>
              <Select<string>
                accessibilityLabel="奖励卡优先度"
                items={REWARD_CARDS}
                value={String(props.config().reward_cards_select)}
                onChange={(next) => setMirrorNumber("reward_cards_select", Number(next))}
                class="w-60"
              />
            </If>
          </SwitchCard>
          <SwitchCard
            label="自定义购物策略"
            field="shopping_strategy"
            checked={() => props.config().shopping_strategy}
          >
            <If when={props.config().shopping_strategy}>
              <Select<string>
                accessibilityLabel="自定义购物策略"
                items={SHOPPING_STRATEGIES}
                value={String(props.config().shopping_strategy_select)}
                onChange={(next) =>
                  setMirrorNumber("shopping_strategy_select", Number(next))
                }
                class="w-60"
              />
            </If>
          </SwitchCard>
          <SwitchCard
            label="自选开局饰品"
            field="opening_items"
            checked={() => props.config().opening_items}
          >
            <If when={props.config().opening_items}>
              <OptionGrid>
                <OptionCell>
                  <ControlRow
                    label="饰品体系"
                    control={
                      <Select<string>
                        accessibilityLabel="开局饰品体系"
                        items={OPENING_SYSTEMS}
                        value={String(props.config().opening_items_system)}
                        onChange={(next) =>
                          setMirrorNumber("opening_items_system", Number(next))
                        }
                        class="w-40"
                      />
                    }
                  />
                </OptionCell>
                <OptionCell>
                  <ControlRow
                    label="选择顺序"
                    control={
                      <Select<string>
                        accessibilityLabel="开局饰品选择顺序"
                        items={OPENING_ORDERS}
                        value={String(props.config().opening_items_select)}
                        onChange={(next) =>
                          setMirrorNumber("opening_items_select", Number(next))
                        }
                        class="w-40"
                      />
                    }
                  />
                </OptionCell>
              </OptionGrid>
            </If>
          </SwitchCard>
        </View>
      </EditorCard>

      <EditorCard title="商店策略">
        <Select<string>
          accessibilityLabel="商店策略"
          items={SHOP_STRATEGIES}
          value={String(props.config().shop_strategy)}
          onChange={(next) => setMirrorNumber("shop_strategy", Number(next))}
          class="w-45"
        />
      </EditorCard>

      <EditorCard title="舍弃饰品体系">
        <Hint>选择不参与商店购买、合成与保留的体系。</Hint>
        <View class="grid w-full grid-cols-5 gap-2">
          <Each of={SYSTEM_NAMES}>
            {(name, index) => (
              <SystemChoice
                system={name()}
                label={SYSTEM_LABELS[name()]}
                destructive
                selected={() => props.config().discard_systems[name()]}
                onPress={() => toggleDiscardSystem(index())}
              />
            )}
          </Each>
        </View>
      </EditorCard>

      <ShopOptions config={props.config} />
    </View>
  );
}
