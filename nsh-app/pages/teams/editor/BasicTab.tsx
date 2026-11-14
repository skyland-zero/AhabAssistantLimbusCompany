// The editor's basic section: what the team is, where it lives in the game's
// forty loadouts, how its formation is sourced, and who is in it. Mirror teams
// also carry the gift system and the rotation switch; resource and general
// teams are manual-only.
import { Clipboard, Text, View } from "nsh";
import { Badge } from "../../../components/Badge";
import { Button } from "../../../components/Button";
import { Input } from "../../../components/Input";
import { Segmented } from "../../../components/Segmented";
import { Select } from "../../../components/Select";
import { Switch } from "../../../components/Switch";
import {
  PURPOSES,
  ROSTER,
  SCHEMES,
  SYSTEM_LABELS,
  clearDraftSinners,
  draftSource,
  draftTeamNumber,
  editTeam,
  editorIsNew,
  freeSlots,
  looksLikeTeamCode,
  setAccessoryScheme,
  setDraftSlot,
  setDraftSource,
  setDraftTeamCode,
  setMirrorBool,
  setMirrorNumber,
  setTeamsFeedback,
  teamCodeValue,
  toggleDraftSinner,
} from "../../../teams";
import type { Team, TeamPurpose } from "../../../teams";
import { ControlRow, EditorCard, Hint, SinnerCard, SystemChoice } from "./parts";

/** `#05` from a number. */
function slotLabel(number: number | null): string {
  return number === null ? "—" : `#${String(number).padStart(2, "0")}`;
}

export function BasicTab(props: { team: () => Team }) {
  const mirror = () => props.team().purpose === "mirror";
  const luxcavation = () => props.team().purpose === "luxcavation";
  const general = () => props.team().purpose === "general";
  const source = () => draftSource();
  const code = () => teamCodeValue(props.team());
  const codeEnabled = () =>
    props.team().purpose === "luxcavation"
      ? code().trim() !== ""
      : (props.team().mirrorConfig?.use_team_code ?? false);
  const slot = () => draftTeamNumber();

  return (
    <View class="flex-col gap-4">
      <View class="w-full flex-row flex-wrap items-start gap-3">
        <View class="min-w-50 flex-1 flex-col gap-1">
          <Text class="text-xs text-muted-foreground">队伍名称</Text>
          <Input
            value={props.team().name}
            accessibilityLabel="队伍名称"
            testID="team-name"
            onChange={({ value }) => editTeam({ name: value })}
            class="w-full"
          />
          <Hint>这里只改 AALC.nsh 备注名，不会修改游戏内队名；游戏内默认名是 编队#N。</Hint>
        </View>
        <View class="w-27 flex-none flex-col gap-1">
          <Text class="text-xs text-muted-foreground">编队位</Text>
          <If
            when={editorIsNew()}
            else={<Badge label={slotLabel(slot())} />}
          >
            <Select<string>
              accessibilityLabel="编队位"
              items={freeSlots()}
              value={String(slot() ?? "")}
              onChange={(next) => setDraftSlot(Number(next))}
              class="w-full"
            />
          </If>
        </View>
        <View class="w-35 flex-none flex-col gap-1">
          <Text class="text-xs text-muted-foreground">用途</Text>
          <Select<TeamPurpose>
            accessibilityLabel="用途"
            items={PURPOSES}
            value={props.team().purpose}
            onChange={(next) => editTeam({ purpose: next })}
            class="w-full"
          />
        </View>
      </View>

      <If when={mirror()}>
        <EditorCard title="饰品体系">
          <View class="grid w-full grid-cols-5 gap-2">
            <Each of={SCHEMES}>
              {(scheme) => (
                <SystemChoice
                  system={scheme()}
                  label={SYSTEM_LABELS[scheme()]}
                  selected={() => props.team().accessoryScheme === scheme()}
                  onPress={() => setAccessoryScheme(scheme())}
                />
              )}
            </Each>
          </View>
          <Hint>用来筛选商店、合成和开局饰品的体系，不是游戏内的队伍标签。</Hint>
        </EditorCard>
      </If>

      <EditorCard title="编队来源">
        <View class="flex-col gap-2">
          <If when={!luxcavation()}>
            <View class="w-full flex-row flex-wrap items-center gap-2">
              <Segmented<"code" | "order">
                options={[
                  { value: "code", label: "编队码" },
                  { value: "order", label: "手动顺序" },
                ]}
                value={source}
                onChange={(next) => setDraftSource(next)}
                accessibilityLabel="编队来源"
              />
              <Text class="min-w-0 flex-1 text-[10px] text-muted-foreground">
                编队码优先于人格顺序；两边的数据都会保存，切换不会丢失。
              </Text>
            </View>
          </If>

          <If when={luxcavation() || source() === "code"}>
            <View class="w-full flex-row items-start gap-2">
              <Input
                value={code()}
                accessibilityLabel="编队码"
                placeholder="从游戏复制的编队码"
                onChange={({ value }) => setDraftTeamCode(value)}
                class="flex-1"
              />
              <Button
                label="复制"
                variant="outline"
                onPress={() => {
                  Clipboard.writeText(code()).then(() => setTeamsFeedback("编队码已复制"));
                }}
              />
            </View>
            <If when={code().trim() === ""}>
              <Text class="text-xs text-warning">编队码为空：运行时将无法加载这支队伍的编队。</Text>
            </If>
            <If when={code().trim() !== "" && !looksLikeTeamCode(code())}>
              <Text class="text-xs text-warning">
                编队码格式看起来不完整；游戏内加载可能失败。
              </Text>
            </If>
            <Hint>
              {luxcavation()
                ? "资源本优先使用编队码；留空时按人格顺序编队，编队码需要在游戏内手动导入一次。"
                : "镜牢开局会把编队码加载到该编队；加载失败会中止本轮，避免用错队伍。"}
            </Hint>
          </If>

          <If when={!luxcavation() && source() === "order"}>
            <Hint>运行时按下面的人格顺序编队，不加载编队码。</Hint>
          </If>
          <If when={luxcavation() && source() === "order"}>
            <Hint>编队码为空，日常任务会按下面的人格顺序编队。</Hint>
          </If>
        </View>
      </EditorCard>

      <EditorCard
        titleBinding={() => `人格顺序（${props.team().sinners.length} / 12）`}
      >
        <View class="grid w-full grid-cols-6 gap-2">
          <Each of={ROSTER}>
            {(sinner) => (
              <SinnerCard
                sinner={sinner()}
                slot={() => {
                  const index = props.team().sinners.indexOf(sinner());
                  return index < 0 ? null : index + 1;
                }}
                onPress={() => toggleDraftSinner(sinner())}
              />
            )}
          </Each>
        </View>
        <View class="w-full flex-row items-end justify-between gap-2 pt-1">
          <Text class="min-w-0 flex-1 text-[10px] leading-4 text-muted-foreground">
            按点击顺序分配 #1~#12；再次点击可移除。镜牢当前前 7 位出战、后 5 位替补，
            顺序影响技能槽增长和替补顺序。
          </Text>
          <Button
            label="清空人格"
            variant="ghost"
            size="xs"
            onPress={() => clearDraftSinners()}
          />
        </View>
      </EditorCard>

      <If when={general()}>
        <EditorCard>
          <Hint>
            通用队伍仅手动使用，不参与镜牢轮换，也不会自动出现在日常任务里。
            切回镜牢时，之前保存的镜牢策略仍会保留。
          </Hint>
        </EditorCard>
      </If>

      <If when={mirror()}>
        <View class="flex-col gap-4">
          <EditorCard>
            <View class="flex-col gap-2">
              <ControlRow
                label="限定难度"
                control={
                  <Switch
                    checked={props.team().mirrorConfig?.fixed_team_use ?? false}
                    accessibilityLabel="限定难度"
                    onChange={({ checked }) => setMirrorBool("fixed_team_use", checked)}
                  />
                }
              />
              <Hint>开启后只在指定难度参与镜牢轮换，另一个难度会跳过它。</Hint>
              <If when={props.team().mirrorConfig?.fixed_team_use}>
                <Select<string>
                  accessibilityLabel="限定难度"
                  items={[
                    { value: "0", label: "困难专用" },
                    { value: "1", label: "普通专用" },
                    { value: "2", label: "全部通用" },
                  ]}
                  value={String(props.team().mirrorConfig?.fixed_team_use_select ?? 0)}
                  onChange={(next) => {
                    setMirrorNumber("fixed_team_use_select", Number(next));
                  }}
                  class="w-45"
                />
              </If>
            </View>
          </EditorCard>

          <EditorCard>
            <ControlRow
              label="加入镜牢轮换"
              control={
                <Switch
                  checked={props.team().enabled}
                  accessibilityLabel="加入镜牢轮换"
                  onChange={({ checked }) => editTeam({ enabled: checked })}
                />
              }
            />
            <Hint>
              启用的镜牢队伍会按启用先后进入轮换队列；每完成一次镜牢，队首转到队尾。
            </Hint>
          </EditorCard>
        </View>
      </If>
    </View>
  );
}
