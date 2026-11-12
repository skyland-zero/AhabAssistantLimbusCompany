// The teams page: a filter strip and the formation slots as a two-column grid.
//
// A slot with a team shows that team (state, meta, members and its commands); a
// slot without one shows the two ways to fill it. Empty slots are part of the
// model, not gaps, so a team can always be created in its own formation number.
import { Scroll, Text, View, token } from "nsh";
import { Badge } from "../../components/Badge";
import { Button, IconButton } from "../../components/Button";
import { Card } from "../../components/Card";
import { PageRoot, PageToolbar } from "../../components/Page";
import { Switch } from "../../components/Switch";
import {
  PURPOSE_LABELS,
  SCHEME_LABELS,
  SINNER_NAMES,
  countFor,
  deleteTeam,
  discardCount,
  hasStarlight,
  openEditor,
  setTeamEnabled,
  setTeamFilter,
  slotsFor,
  teamFilter,
  teamsFeedback,
} from "../../teams";
import type { Team, TeamFilter } from "../../teams";
import { TeamEditor } from "./TeamEditor";

/** The filters, in the order the toolbar shows them. */
const FILTERS: { value: TeamFilter; label: string }[] = [
  { value: "all", label: "全部" },
  { value: "mirror", label: "镜牢" },
  { value: "luxcavation", label: "经验本" },
  { value: "general", label: "通用" },
];

export function Teams() {
  const slots = () => slotsFor(teamFilter());
  return (
    <PageRoot>
      <PageToolbar>
        <View class="w-full flex-row flex-wrap items-center justify-between gap-3">
          <View class="flex-row flex-wrap items-center gap-1 rounded-(--radius) bg-muted p-0.5">
            <Each of={FILTERS} key={(filter) => filter.value}>
              {(filter) => <FilterTab filter={filter()} />}
            </Each>
          </View>
          <Button
            label="新建队伍"
            icon="plus"
            onPress={() => openEditor(null)}
            testID="new-team"
          />
        </View>
      </PageToolbar>
      <If when={teamsFeedback()}>
        {(line) => (
          <Card class="w-full bg-success-light p-3">
            <Text class="text-xs text-success">{line()}</Text>
          </Card>
        )}
      </If>
      <Scroll class="min-h-0 flex-1">
        <View class="grid w-full grid-cols-2 items-stretch gap-3">
          <Each of={slots()} key={(slot) => slot.number}>
            {(slot) => (
              <If when={slot().team} else={<EmptySlot number={slot().number} />}>
                {(team) => <TeamCard team={team()} slot={slot().number} />}
              </If>
            )}
          </Each>
        </View>
      </Scroll>
      <TeamEditor />
    </PageRoot>
  );
}

/** One filter tab: its label and how many teams it holds. */
function FilterTab(props: { filter: { value: TeamFilter; label: string } }) {
  const active = () => teamFilter() === props.filter.value;
  const count = () => countFor(props.filter.value);
  return (
    <Button
      label={props.filter.label}
      variant="ghost"
      selected={active()}
      class={`min-h-7 px-3 ${active() ? "bg-card text-foreground hover:bg-card" : "text-muted-foreground"}`}
      onPress={() => setTeamFilter(props.filter.value)}
      testID={`team-filter-${props.filter.value}`}
    >
      <If when={props.filter.value === "all" || count() > 0}>
        <View class="rounded-sm bg-secondary px-1">
          <Text class="text-xs text-muted-foreground">{count()}</Text>
        </View>
      </If>
    </Button>
  );
}

/** A saved team: its number, state, commands, meta and party. */
function TeamCard(props: { team: Team; slot: number }) {
  const team = () => props.team;
  const luxcavation = () => team().purpose === "luxcavation";
  return (
    <Card class="w-full p-3">
      <View class="flex-col gap-2">
        <View class="w-full flex-row items-center justify-between gap-2">
          <Badge label={`#${String(props.slot).padStart(2, "0")}`} />
          <View class="flex-row items-center gap-2">
            <If when={!luxcavation()}>
              <View class="flex-row items-center gap-1">
                <Text class="text-xs text-muted-foreground">启用</Text>
                <Switch
                  checked={team().enabled}
                  accessibilityLabel={`启用 ${team().name}`}
                  onChange={({ checked }) => setTeamEnabled(team().id, checked)}
                />
              </View>
            </If>
            <Button
              label="从预设覆盖"
              variant="outline"
              size="xs"
              onPress={() => openEditor(team())}
            />
            <IconButton
              name="pencil"
              label={`编辑 ${team().name}`}
              size="xs"
              onPress={() => openEditor(team())}
            />
            <IconButton
              name="trash"
              label={`删除 ${team().name}`}
              size="xs"
              iconColor={token("danger")}
              onPress={() => deleteTeam(team().id)}
            />
          </View>
        </View>
        <Text class="text-sm font-semibold text-foreground">{team().name}</Text>
        <View class="w-full flex-row flex-wrap items-center gap-2">
          <Text class="text-xs text-muted-foreground">{PURPOSE_LABELS[team().purpose]}</Text>
          <Text class="text-xs text-foreground">
            {SCHEME_LABELS[team().accessoryScheme] ?? team().accessoryScheme}
          </Text>
          <Text class="text-xs text-muted-foreground">{team().sinners.length} 人格</Text>
          <If when={hasStarlight(team())}>
            <Badge label="已配星光" tone="success" />
          </If>
          <If when={discardCount(team()) > 0}>
            <Badge label={`舍弃 ${discardCount(team())} 项`} tone="danger" />
          </If>
          <If when={!team().enabled && !luxcavation()}>
            <Badge label="已停用" />
          </If>
        </View>
        <View class="w-full flex-row flex-wrap gap-1">
          <Each of={team().sinners}>
            {(sinner, index) => (
              <Badge label={`#${index() + 1} ${SINNER_NAMES[sinner()] ?? sinner()}`} />
            )}
          </Each>
        </View>
      </View>
    </Card>
  );
}

/** A formation slot with no team: the two ways to fill it. */
function EmptySlot(props: { number: number }) {
  return (
    <Card class="w-full bg-muted/40 p-3">
      <View class="flex-col gap-2">
        <View class="w-full flex-row items-center justify-between gap-2">
          <Badge label={`#${String(props.number).padStart(2, "0")}`} />
        </View>
        <View class="w-full flex-row items-center justify-between gap-2">
          <View class="flex-col gap-1">
            <Text class="text-sm text-muted-foreground">未配置</Text>
            <Text class="text-xs text-muted-foreground">镜牢</Text>
          </View>
          <View class="flex-row items-center gap-2">
            <Button
              label="新建"
              icon="plus"
              variant="ghost"
              size="xs"
              class="text-brand"
              iconColor={token("brand")}
              onPress={() => openEditor(null)}
            />
            <Button
              label="从预设编队选择"
              icon="package"
              variant="outline"
              size="xs"
              onPress={() => openEditor(null)}
            />
          </View>
        </View>
      </View>
    </Card>
  );
}
