// The mirror rotation as a first-class object: the runner consumes
// `teams_active_queue[0]`, then rotates it to the back after every run. The
// page must show that order instead of hiding it behind a switch. This is a
// single-line card, not a full section: it is a status strip above the slot map.
import { Text, View } from "nsh";
import { Badge } from "../../components/Badge";
import { Pressable } from "../../components/Button";
import { Card } from "../../components/Card";
import type { Team } from "../../teams";

/** The difficulty a fixed-use team is limited to, or `null` when unrestricted. */
function difficulty(team: Team): string | null {
  if (!team.mirrorConfig?.fixed_team_use) return null;
  if (team.mirrorConfig.fixed_team_use_select === 2) return null;
  return ["困难专用", "普通专用", "全部通用"][team.mirrorConfig.fixed_team_use_select] ?? null;
}

export function QueueStrip(props: { teams: Team[]; onOpen: (team: Team) => void }) {
  return (
    <Card class="w-full p-0" arrive={0}>
      <View class="w-full flex-row flex-wrap items-center gap-2 px-4 py-2.5">
        <Text class="flex-none text-sm font-semibold text-foreground">镜牢轮换</Text>
        <Text class="flex-none text-[10px] text-muted-foreground">{`${props.teams.length} 支`}</Text>
        <View class="min-w-0 flex-1 flex-row flex-wrap items-center gap-1.5">
          <If
            when={props.teams.length > 0}
            else={
              <Text class="text-xs text-muted-foreground">
                未启用任何镜牢队伍；镜牢任务不会启动。打开镜牢队伍卡片里的“加入镜牢轮换”。
              </Text>
            }
          >
            <Each of={props.teams} key={(team) => team.id}>
              {(team) => (
                <View class="flex-row items-center gap-1.5">
                  <Pressable
                    accessibilityLabel={`轮换第 ${team().queuePosition} 位：${team().name}`}
                    testID={`queue-${team().id}`}
                    onPress={() => props.onOpen(team())}
                    class="min-h-7 flex-row items-center gap-1.5 rounded-(--radius) border border-border bg-card px-2 hover:bg-accent active:bg-secondary"
                  >
                    <View class="rounded-full bg-brand px-1.5">
                      <Text class="text-[10px] font-medium text-brand-foreground">
                        {`#${team().queuePosition}`}
                      </Text>
                    </View>
                    <Text class="max-w-36 truncate text-xs text-foreground">{team().name}</Text>
                    <If when={difficulty(team()) !== null}>
                      <Badge label={difficulty(team()) ?? ""} tone="warning" />
                    </If>
                  </Pressable>
                  <Text class="text-xs text-muted-foreground">›</Text>
                </View>
              )}
            </Each>
          </If>
        </View>
        <Text class="flex-none text-[10px] text-muted-foreground">
          每完成一次镜牢，队首转到队尾
        </Text>
      </View>
    </Card>
  );
}
