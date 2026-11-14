// The theme-pack page: one total, four commands and one row per pack.
//
// A pack's row is its switch, its name and tier, and a weight slider from 0 to
// 10; a disabled pack dims and its slider stops taking input, because its weight
// no longer affects a run.
import { Scroll, Text, View, token } from "nsh";
import { Slider } from "nsh/base";
import { Badge } from "../../components/Badge";
import { Button } from "../../components/Button";
import { Card } from "../../components/Card";
import { Notice } from "../../components/Notice";
import { PageRoot, PageToolbar } from "../../components/Page";
import { Switch } from "../../components/Switch";
import {
  MAX_WEIGHT,
  hardMirror,
  isSortedByWeight,
  resetWeights,
  setAllEnabled,
  setPackWeight,
  themePacks,
  themePacksFeedback,
  togglePack,
  toggleSortByWeight,
  totalWeight,
} from "../../themepacks";
import type { ThemePack } from "../../themepacks";

export function ThemePacks() {
  return (
    <PageRoot>
      <PageToolbar>
        <View class="w-full flex-row flex-wrap items-center justify-between gap-3">
          <View class="flex-row items-center gap-2">
            <Text class="text-xs text-muted-foreground">总权重：</Text>
            <Text class="font-mono text-xs text-foreground">{totalWeight()}</Text>
          </View>
          <View class="flex-row flex-wrap items-center justify-end gap-1.5">
            <Button
              label="按权重排序"
              icon="refresh"
              variant={isSortedByWeight() ? "secondary" : "outline"}
              selected={isSortedByWeight()}
              onPress={() => toggleSortByWeight()}
            />
            <Button label="全部启用" variant="outline" onPress={() => setAllEnabled(true)} />
            <Button label="全部停用" variant="outline" onPress={() => setAllEnabled(false)} />
            <Button
              label="恢复默认权重"
              icon="rotateCcw"
              variant="ghost"
              onPress={() => resetWeights()}
            />
          </View>
        </View>
      </PageToolbar>
      <If when={hardMirror()}>
        <Notice
          tone="warning"
          label="困难镜牢周期进行中：建议优先筛选高星主题包并提高其权重"
        />
      </If>
      <If when={themePacksFeedback()}>
        {(line) => <Notice tone="success" label={line()} />}
      </If>
      <Scroll class="min-h-0 flex-1">
        <View class="w-full flex-col gap-2">
          <Each of={themePacks()} key={(pack) => pack.id}>
            {(pack, index) => <PackRow pack={pack()} index={index()} />}
          </Each>
        </View>
      </Scroll>
    </PageRoot>
  );
}

/** One pack: its switch, name and tier, then its weight. */
function PackRow(props: { pack: ThemePack; index: number }) {
  const pack = () => props.pack;
  const enabled = () => pack().enabled;
  return (
    <Card class="w-full p-0" arrive={props.index}>
      <View class={`flex-row items-center gap-4 px-4 py-3 ${enabled() ? "" : "opacity-60"}`}>
        <Switch
          checked={enabled()}
          accessibilityLabel={`启用 ${pack().name}`}
          onChange={() => togglePack(pack().id)}
        />
        <View class="w-44 flex-none flex-row items-center gap-2">
          <Text class="min-w-0 text-sm text-foreground">{pack().name}</Text>
          <Badge label={pack().tier} />
        </View>
        <View class="min-w-0 flex-1 flex-row items-center gap-3">
          <Text class="flex-none text-xs text-muted-foreground">权重</Text>
          <View class="min-w-20 flex-1">
            <Slider
              value={pack().weight}
              min={0}
              max={MAX_WEIGHT}
              step={1}
              disabled={!enabled()}
              accessibilityLabel={`${pack().name} 权重`}
              onChange={({ value }) =>
                setPackWeight(pack().id, typeof value === "number" ? value : pack().weight)
              }
              trackClass="h-4 justify-center"
              indicatorClass="h-1.5 rounded-full bg-muted"
              rangeClass="h-full rounded-full bg-primary"
              thumbClass="size-4 rounded-full border border-primary bg-card shadow-sm focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-ring"
            />
          </View>
          <Text class="w-6 flex-none font-mono text-xs text-foreground">{pack().weight}</Text>
        </View>
      </View>
    </Card>
  );
}
