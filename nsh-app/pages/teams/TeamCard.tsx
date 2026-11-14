// One saved team: its game loadout number in the title, its AALC remark name
// beside it, and the information needed to decide whether the run can use it.
import { Text, View, token } from "nsh";
import { Badge } from "../../components/Badge";
import { IconButton } from "../../components/Button";
import { Card, CardBody } from "../../components/Card";
import { Switch } from "../../components/Switch";
import {
  PURPOSE_LABELS,
  SCHEME_LABELS,
  SINNER_NAMES,
  discardCount,
  hasStarlight,
  openEditor,
  requestDeleteTeam,
  routeLabel,
  setTeamEnabled,
  teamCodeValue,
  teamNumber,
  teamSource,
  type Team,
} from "../../teams";

/** `#02` from a team id. */
function slotLabel(team: Team): string {
  const number = teamNumber(team.id);
  return number === null ? "#—" : `#${String(number).padStart(2, "0")}`;
}

/** One row of ordered sinner chips, after the optional first-N divider. */
function SinnerRow(props: { from: number; sinners: string[]; class?: string }) {
  return (
    <View class={`flex-row flex-wrap gap-1 ${props.class ?? "w-full"}`}>
      <Each of={props.sinners}>
        {(sinner, index) => (
          <Badge label={`#${props.from + index() + 1} ${SINNER_NAMES[sinner()] ?? sinner()}`} />
        )}
      </Each>
    </View>
  );
}

export function TeamCard(props: {
  team: Team;
  index: number;
  onPreset: (team: Team) => void;
}) {
  const team = () => props.team;
  const mirror = () => team().purpose === "mirror";
  const luxcavation = () => team().purpose === "luxcavation";
  const general = () => team().purpose === "general";
  const source = () => teamSource(team());
  const code = () => teamCodeValue(team());
  const note = () => {
    if (team().queuePosition > 0) return `轮换 #${team().queuePosition}`;
    if (mirror()) return "未启用";
    if (luxcavation()) return "日常";
    return "手动";
  };
  return (
    <Card
      title={`${slotLabel(team())} ${team().name}`}
      note={note()}
      class="w-full"
      arrive={props.index}
      actions={
        <View class="flex-row items-center gap-1.5">
          <If when={mirror()}>
            <Switch
              checked={team().enabled}
              accessibilityLabel={`加入镜牢轮换：${team().name}`}
              onChange={({ checked }) => setTeamEnabled(team().id, checked)}
            />
          </If>
          <If when={!general()}>
            <IconButton
              name="package"
              label={`从预设覆盖 ${team().name}`}
              size="xs"
              onPress={() => props.onPreset(team())}
            />
          </If>
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
            onPress={() => requestDeleteTeam(team().id)}
          />
        </View>
      }
    >
      <CardBody class="gap-2">
        <View class="w-full flex-row flex-wrap items-center gap-2">
          <Text class="text-xs text-muted-foreground">{PURPOSE_LABELS[team().purpose]}</Text>
          <If when={mirror()}>
            <Text class="text-xs text-foreground">
              {SCHEME_LABELS[team().accessoryScheme] ?? team().accessoryScheme}
            </Text>
            <Text class="text-xs text-muted-foreground">{routeLabel(team().mirrorConfig?.mirror_route_profile)}</Text>
          </If>
          <Text class="text-xs text-muted-foreground">
            {source() === "code" ? "编队码" : `${team().sinners.length} 人格`}
          </Text>
          <If when={source() === "empty"}>
            <Badge label="未配置编队" tone="warning" />
          </If>
          <If when={source() === "code" && code().trim() === ""}>
            <Badge label="编队码缺失" tone="danger" />
          </If>
          <If when={hasStarlight(team().mirrorConfig) && mirror()}>
            <Badge label="已配星光" tone="success" />
          </If>
          <If when={discardCount(team().mirrorConfig) > 0 && mirror()}>
            <Badge label={`舍弃 ${discardCount(team().mirrorConfig)} 项`} tone="danger" />
          </If>
          <If when={!mirror()}>
            <Badge label={luxcavation() ? "仅日常" : "仅手动"} />
          </If>
        </View>
        <If when={team().sinners.length > 0}>
          <View class="w-full flex-col gap-1">
            <SinnerRow from={0} sinners={team().sinners.slice(0, 7)} />
            <If when={team().sinners.length > 7}>
              <View class="w-full flex-row items-start gap-2 pt-0.5">
                <Text class="flex-none pt-1 text-[10px] text-muted-foreground">
                  {mirror() ? "替补" : "后续"}
                </Text>
                <SinnerRow
                  from={7}
                  sinners={team().sinners.slice(7)}
                  class="min-w-0 flex-1"
                />
              </View>
            </If>
          </View>
        </If>
      </CardBody>
    </Card>
  );
}
