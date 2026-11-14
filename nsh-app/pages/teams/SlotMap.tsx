// The game's forty loadout numbers as one compact map. The map is the page's
// index: a used number opens its team, an empty number creates there, and the
// selection-mode switch makes the name/order correspondence explicit.
import { Text, View } from "nsh";
import { Card, CardBody } from "../../components/Card";
import { Pressable } from "../../components/Button";
import { Select } from "../../components/Select";
import type { Team } from "../../teams";

/** One slot as the page passes it: a game number and its AALC team. */
export type Slot = { number: number; team: Team | null };

/** A purpose dot: colour is a secondary cue, the title carries the purpose. */
function purposeDot(team: Team): string {
  if (team.purpose === "mirror") return "bg-brand";
  if (team.purpose === "luxcavation") return "bg-success";
  return "bg-muted-foreground";
}

export function SlotMap(props: {
  slots: Slot[];
  selectedId: string | null;
  selectionMode: () => boolean;
  onSelectionMode: (enabled: boolean) => void;
  onOpen: (team: Team) => void;
  onCreate: (number: number) => void;
}) {
  const mode = () => (props.selectionMode() ? "order" : "name");
  return (
    <Card
      title="编队位（游戏内 1–40）"
      note="点击空位直接在该编号新建"
      class="w-full"
      actions={
        <Select<string>
          accessibilityLabel="选队方式"
          items={[
            { value: "name", label: "按队名" },
            { value: "order", label: "按序号" },
          ]}
          value={mode()}
          onChange={(next) => props.onSelectionMode(next === "order")}
          class="w-28"
        />
      }
    >
      <CardBody class="gap-2">
        <View class="grid w-full grid-cols-8 gap-1.5">
          <Each of={props.slots} key={(slot) => slot.number}>
            {(slot) => {
              const team = () => slot().team;
              const selected = () => team() !== null && team()?.id === props.selectedId;
              return (
                <Pressable
                  accessibilityLabel={
                    team()
                      ? `编队 #${slot().number}，${team()?.name}`
                      : `编队 #${slot().number}，AALC.nsh 未配置，点击新建`
                  }
                  testID={`slot-${slot().number}`}
                  selected={selected()}
                  onPress={() =>
                    team() ? props.onOpen(team() as Team) : props.onCreate(slot().number)
                  }
                  class={`h-8 min-w-0 flex-col justify-center gap-0.5 rounded-(--radius-sm) border px-1.5 transition-colors duration-150 focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-ring ${
                    selected()
                      ? "border-brand bg-brand-light"
                      : team()
                        ? "border-border bg-card hover:bg-accent active:bg-secondary"
                        : "border-transparent bg-muted/30 hover:bg-accent active:bg-secondary"
                  }`}
                >
                  <View class="w-full flex-row items-center gap-1">
                    <If when={team()}>
                      {() => <View class={`size-1.5 flex-none rounded-full ${purposeDot(team() as Team)}`} />}
                    </If>
                    <Text class="flex-none text-[10px] text-muted-foreground">
                      {`#${String(slot().number).padStart(2, "0")}`}
                    </Text>
                  </View>
                  <Text
                    class={`w-full truncate text-xs ${
                      team() ? "text-foreground" : "text-muted-foreground"
                    }`}
                  >
                    {team()?.name ?? "AALC.nsh 未配置"}
                  </Text>
                </Pressable>
              );
            }}
          </Each>
        </View>
        <Text class="text-[10px] leading-4 text-muted-foreground">
          按队名：游戏内队名保持 编队#N；按序号：游戏内列表 1–40 顺序不能乱，拖动排序后要重新核对。
          “AALC.nsh 未配置”只表示这边没有策略，不代表游戏里那个编队位是空的。
        </Text>
      </CardBody>
    </Card>
  );
}
