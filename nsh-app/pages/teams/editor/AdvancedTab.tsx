// The editor's advanced section: the starting-gift search, the team's theme
// pack weight, JSON import/export and the accumulated run statistics.
import { Dialog } from "nsh/base";
import { Text, View, signal } from "nsh";
import { Button, IconButton } from "../../../components/Button";
import { Input } from "../../../components/Input";
import { Switch } from "../../../components/Switch";
import {
  SPIDERWEB_GIFT_ID,
  addObserveGift,
  applyDraftJson,
  draftJson,
  draftMirror,
  editorTeamId,
  observeGiftLabel,
  removeObserveGift,
  setMirrorBool,
} from "../../../teams";
import {
  cancelClearTeamStats,
  clearTeamStats,
  durationLabel,
  refreshTeamStats,
  requestClearTeamStats,
  statsClearRequested,
  teamStats,
} from "../../../teams-stats";
import {
  ControlRow,
  EditorCard,
  Hint,
  OptionCell,
  OptionGrid,
  SectionTitle,
} from "./parts";

export function AdvancedTab() {
  const observeDraft = signal("");
  const jsonOpen = signal(false);
  const jsonDraft = signal("");

  return (
    <View class="flex-col gap-4">
      <EditorCard>
        <View class="flex-col gap-2">
          <ControlRow
            label="开局礼物搜索"
            control={
              <Switch
                checked={draftMirror().observe_ego_gift}
                accessibilityLabel="开局礼物搜索"
                onChange={({ checked }) => setMirrorBool("observe_ego_gift", checked)}
              />
            }
          />
          <Hint>
            推荐直接选择小指良专属礼物；高级配置可填写 system_level_row_col。
          </Hint>
          <View class="flex-row flex-wrap items-center gap-2">
            <Text class="text-xs text-muted-foreground">快速选择</Text>
            <Button
              label="赤红纠缠的蜘蛛巢"
              variant="outline"
              size="xs"
              selected={draftMirror().observe_ego_gift_selected.includes(SPIDERWEB_GIFT_ID)}
              onPress={() => addObserveGift(SPIDERWEB_GIFT_ID)}
            />
          </View>
          <SectionTitle>高级坐标配置</SectionTitle>
          <If when={draftMirror().observe_ego_gift}>
            <View class="flex-col gap-2 pt-1">
              <View class="flex-row items-center gap-2">
                <Input
                  value={observeDraft()}
                  accessibilityLabel="E.G.O 饰品坐标"
                  placeholder="system_level_row_col"
                  onSubmit={({ text }) => {
                    addObserveGift(text);
                    observeDraft.set("");
                  }}
                  onChange={({ value }) => observeDraft.set(value)}
                  class="flex-1"
                />
                <Button
                  label="添加"
                  disabled={observeDraft().trim() === ""}
                  onPress={() => {
                    addObserveGift(observeDraft());
                    observeDraft.set("");
                  }}
                />
              </View>
              <View class="flex-row flex-wrap gap-1">
                <Each of={draftMirror().observe_ego_gift_selected}>
                  {(gift) => (
                    <View class="flex-row items-center gap-1 rounded-(--radius) bg-secondary px-2 py-1">
                      <Text class="text-xs text-foreground">{observeGiftLabel(gift())}</Text>
                      <IconButton
                        name="close"
                        label={`移除 ${observeGiftLabel(gift())}`}
                        size="xs"
                        class="size-5"
                        onPress={() => removeObserveGift(gift())}
                      />
                    </View>
                  )}
                </Each>
              </View>
            </View>
          </If>
        </View>
      </EditorCard>

      <EditorCard>
        <View class="flex-col gap-2">
          <ControlRow
            label="使用队伍专属主题包权重"
            control={
              <Switch
                checked={draftMirror().use_custom_theme_pack_weight}
                accessibilityLabel="使用队伍专属主题包权重"
                onChange={({ checked }) =>
                  setMirrorBool("use_custom_theme_pack_weight", checked)
                }
              />
            }
          />
          <Hint>保存后由镜牢执行器读取该队伍的主题包权重。</Hint>
        </View>
      </EditorCard>

      <EditorCard>
        <View class="flex-col gap-2">
          <View class="w-full flex-row items-center justify-between gap-2">
            <SectionTitle>配置导入导出</SectionTitle>
            <Button
              label="粘贴 / 导入 JSON"
              icon="restore"
              variant="outline"
              size="xs"
              onPress={() => {
                if (!jsonOpen()) jsonDraft.set(draftJson());
                jsonOpen.set(!jsonOpen());
              }}
            />
          </View>
          <If when={jsonOpen()}>
            <View class="flex-col gap-2">
              <Input
                value={jsonDraft()}
                accessibilityLabel="队伍 JSON"
                placeholder="粘贴 TeamDetail JSON"
                onChange={({ value }) => jsonDraft.set(value)}
              />
              <View class="flex-row justify-end gap-2">
                <Button
                  label="关闭"
                  variant="ghost"
                  size="xs"
                  onPress={() => jsonOpen.set(false)}
                />
                <Button
                  label="校验并覆盖"
                  size="xs"
                  onPress={() => {
                    if (applyDraftJson(jsonDraft())) jsonOpen.set(false);
                  }}
                />
              </View>
            </View>
          </If>
        </View>
      </EditorCard>

      <EditorCard>
        <View class="flex-col gap-3">
          <View class="w-full flex-row items-center justify-between gap-2">
            <SectionTitle>编队统计数据</SectionTitle>
            <View class="flex-row gap-2">
              <Button
                label="刷新数据"
                variant="outline"
                size="xs"
                onPress={() => refreshTeamStats(editorTeamId())}
              />
              <Button
                label="清除历史统计数据"
                variant="danger"
                size="xs"
                onPress={() => requestClearTeamStats(editorTeamId())}
              />
            </View>
          </View>

          <ControlRow
            label="总镜牢次数"
            control={
              <Text class="text-xs text-foreground">
                {teamStats(editorTeamId()).totalCount}
              </Text>
            }
          />
          <OptionGrid>
            <StatsCell
              label="困难镜牢次数"
              value={() => String(teamStats(editorTeamId()).hard.count)}
            />
            <StatsCell
              label="困难镜牢平均用时"
              value={() => durationLabel(teamStats(editorTeamId()).hard.averageSeconds)}
            />
            <StatsCell
              label="困难镜牢最近 5 次平均用时"
              value={() =>
                durationLabel(teamStats(editorTeamId()).hard.last5AverageSeconds)
              }
            />
            <StatsCell
              label="困难镜牢最近 10 次平均用时"
              value={() =>
                durationLabel(teamStats(editorTeamId()).hard.last10AverageSeconds)
              }
            />
            <StatsCell
              label="普通镜牢次数"
              value={() => String(teamStats(editorTeamId()).normal.count)}
            />
            <StatsCell
              label="普通镜牢平均用时"
              value={() => durationLabel(teamStats(editorTeamId()).normal.averageSeconds)}
            />
            <StatsCell
              label="普通镜牢最近 5 次平均用时"
              value={() =>
                durationLabel(teamStats(editorTeamId()).normal.last5AverageSeconds)
              }
            />
            <StatsCell
              label="普通镜牢最近 10 次平均用时"
              value={() =>
                durationLabel(teamStats(editorTeamId()).normal.last10AverageSeconds)
              }
            />
          </OptionGrid>
        </View>
      </EditorCard>

      <Confirmation />
    </View>
  );
}

/** One statistic: the label and the value the stats signal currently holds. */
function StatsCell(props: { label: string; value: () => string }) {
  return (
    <OptionCell>
      <ControlRow
        label={props.label}
        control={<Text class="text-xs text-foreground">{props.value()}</Text>}
      />
    </OptionCell>
  );
}

/** The clear-history confirmation, drawn as a small modal over the editor. */
function Confirmation() {
  return (
    <Dialog
      open={statsClearRequested()}
      onOpenChange={({ open }) => {
        if (!open) cancelClearTeamStats();
      }}
      closeOnEscape
      closeOnBackdropPress
      class="m-auto w-100 rounded-(--radius-lg) border border-border bg-popover p-4 shadow-lg"
      backdropClass="bg-scrim"
      role="alertdialog"
      accessibilityLabel="清除历史统计数据"
    >
      <View class="flex-col gap-3">
        <Text class="text-sm font-semibold text-popover-foreground">
          清除历史统计数据
        </Text>
        <Text class="text-xs text-muted-foreground">
          将删除这支队伍的全部镜牢统计数据，且无法恢复。
        </Text>
        <View class="flex-row justify-end gap-2">
          <Button
            label="取消"
            variant="ghost"
            onPress={() => cancelClearTeamStats()}
          />
          <Button label="清除" variant="danger" onPress={() => clearTeamStats(editorTeamId())} />
        </View>
      </View>
    </Dialog>
  );
}
