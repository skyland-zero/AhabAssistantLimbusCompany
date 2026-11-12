// The editor's basic section: what the team is, which gift system it wears,
// and who is in it. An experience-dungeon team has only this section.
import { Clipboard, Text, View } from "nsh";
import { Button } from "../../../components/Button";
import { Input } from "../../../components/Input";
import { Select } from "../../../components/Select";
import { Switch } from "../../../components/Switch";
import {
  PURPOSES,
  ROSTER,
  SCHEMES,
  SYSTEM_LABELS,
  clearDraftSinners,
  editTeam,
  setAccessoryScheme,
  setDraftTeamCode,
  setMirrorBool,
  setMirrorNumber,
  setTeamsFeedback,
  toggleDraftSinner,
} from "../../../teams";
import type { Team, TeamPurpose } from "../../../teams";
import { ControlRow, EditorCard, Hint, SinnerCard, SystemChoice } from "./parts";

export function BasicTab(props: { team: () => Team }) {
  const luxcavation = () => props.team().purpose === "luxcavation";
  const codeEnabled = () =>
    !luxcavation() && (props.team().mirrorConfig?.use_team_code ?? false);

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

      <If when={!luxcavation()}>
        <View class="flex-col gap-4">
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
                按点击顺序分配 #1~#12；再次点击可移除。
              </Text>
              <Button
                label="清空人格"
                variant="ghost"
                size="xs"
                onPress={() => clearDraftSinners()}
              />
            </View>
          </EditorCard>
        </View>
      </If>

      <If when={luxcavation()}>
        <EditorCard>
          <View class="flex-col gap-2">
            <Text class="text-xs text-muted-foreground">编队码</Text>
            <Input
              value={props.team().teamCode}
              accessibilityLabel="编队码"
              placeholder="从游戏复制的编队码"
              onChange={({ value }) => setDraftTeamCode(value)}
            />
            <Button
              label="复制编队码"
              variant="ghost"
              size="xs"
              class="self-start"
              onPress={() => {
                Clipboard.writeText(props.team().teamCode).then(() =>
                  setTeamsFeedback("编队码已复制"),
                );
              }}
            />
            <Hint>
              编队码会随队伍保存；在游戏内导入一次即可。日常任务仍按编号选队。
            </Hint>
          </View>
        </EditorCard>
      </If>

      <If when={!luxcavation()}>
        <View class="flex-col gap-4">
          <EditorCard>
            <View class="flex-col gap-2">
              <ControlRow
                label="启用编队码"
                control={
                  <Switch
                    checked={codeEnabled()}
                    accessibilityLabel="启用编队码"
                    onChange={({ checked }) => setMirrorBool("use_team_code", checked)}
                  />
                }
              />
              <Hint>启用后保存编队码，运行时由后端解析。</Hint>
              <If when={codeEnabled()}>
                <Input
                  value={props.team().teamCode}
                  accessibilityLabel="编队码"
                  placeholder="从游戏复制的编队码"
                  onChange={({ value }) => setDraftTeamCode(value)}
                />
              </If>
            </View>
          </EditorCard>

          <EditorCard>
            <View class="flex-col gap-2">
              <ControlRow
                label="固定队伍用途"
                control={
                  <Switch
                    checked={props.team().mirrorConfig?.fixed_team_use ?? false}
                    accessibilityLabel="固定队伍用途"
                    onChange={({ checked }) => setMirrorBool("fixed_team_use", checked)}
                  />
                }
              />
              <If when={props.team().mirrorConfig?.fixed_team_use}>
                <Select<string>
                  accessibilityLabel="固定队伍用途"
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
              label="队伍启用"
              control={
                <Switch
                  checked={props.team().enabled}
                  accessibilityLabel="启用队伍"
                  onChange={({ checked }) => editTeam({ enabled: checked })}
                />
              }
            />
          </EditorCard>
        </View>
      </If>
    </View>
  );
}
