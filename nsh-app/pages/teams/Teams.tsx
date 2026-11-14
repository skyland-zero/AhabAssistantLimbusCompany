// The teams page: the mirror rotation, the game's forty loadout numbers and
// the configured teams. A slot is the game address; purpose is a tag used by
// the filters, not a range of numbers.
import { Scroll, Text, View, signal } from "nsh";
import { Dialog } from "nsh/base";
import { Button } from "../../components/Button";
import { EmptyState } from "../../components/EmptyState";
import { Notice } from "../../components/Notice";
import { PageRoot, PageToolbar } from "../../components/Page";
import { Tabs } from "../../components/Tabs";
import {
  allSlots,
  cancelDeleteTeam,
  confirmDeleteTeam,
  countFor,
  deleteTarget,
  editingTeam,
  extraTeams,
  freeSlots,
  openEditor,
  queueTeams,
  selectionMode,
  setSelectionMode,
  setTeamFilter,
  teamFilter,
  teamsFeedback,
  teamsFeedbackTone,
  teamsFor,
  teamsLoading,
  type Team,
  type TeamFilter,
} from "../../teams";
import { PresetPicker } from "./PresetPicker";
import { QueueStrip } from "./QueueStrip";
import { SlotMap } from "./SlotMap";
import { TeamCard } from "./TeamCard";
import { TeamEditor } from "./TeamEditor";

/** The filters, in the order the toolbar shows them. */
const FILTERS: { value: TeamFilter; label: string }[] = [
  { value: "all", label: "全部" },
  { value: "mirror", label: "镜牢" },
  { value: "luxcavation", label: "资源本" },
  { value: "general", label: "通用" },
];

export function Teams() {
  const pickerOpen = signal(false);
  const pickerOverwrite = signal<Team | null>(null);
  const slots = () => allSlots();
  const extras = () =>
    extraTeams().filter((team) => teamFilter() === "all" || team.purpose === teamFilter());
  const cards = () => teamsFor(teamFilter());

  const openPreset = (team: Team | null) => {
    pickerOverwrite.set(team);
    pickerOpen.set(true);
  };

  return (
    <PageRoot>
      <PageToolbar>
        <View class="w-full flex-row flex-wrap items-center justify-between gap-3">
          <Tabs<TeamFilter>
            items={FILTERS}
            value={teamFilter}
            onChange={(next) => setTeamFilter(next)}
            accessibilityLabel="队伍分类"
            trackClass="flex-wrap items-center gap-1 rounded-(--radius) bg-muted p-0.5"
            itemClass="min-h-7 px-3"
            pillClass="bg-card shadow-sm"
            itemTestID={(filter) => `team-filter-${filter.value}`}
            suffix={(filter) => (
              <If when={filter.value === "all" || countFor(filter.value) > 0}>
                <View class="rounded-sm bg-secondary px-1">
                  <Text class="text-xs text-muted-foreground">{countFor(filter.value)}</Text>
                </View>
              </If>
            )}
          />
          <View class="flex-row flex-wrap items-center gap-2">
            <Button
              label="从预设新建"
              icon="package"
              variant="outline"
              disabled={freeSlots().length === 0}
              onPress={() => openPreset(null)}
              testID="preset-new"
            />
            <Button
              label="新建队伍"
              icon="plus"
              disabled={freeSlots().length === 0}
              onPress={() => openEditor(null)}
              testID="new-team"
            />
          </View>
        </View>
      </PageToolbar>
      <If when={teamsFeedback()}>
        {(line) => <Notice tone={teamsFeedbackTone()} label={line()} />}
      </If>
      <Scroll class="min-h-0 flex-1">
        <View class="w-full flex-col gap-3 pb-2">
          <If
            when={slots().length > 0}
            else={
              <EmptyState
                title={teamsLoading() ? "正在加载队伍…" : "队伍数据暂不可用"}
                detail={
                  teamsLoading()
                    ? "正在连接 Python 后端。"
                    : "请检查后端连接后重试；已保存的队伍不会被清空。"
                }
                class="min-h-60"
              />
            }
          >
            <QueueStrip teams={queueTeams()} onOpen={(team) => openEditor(team)} />
            <SlotMap
              slots={slots()}
              selectedId={editingTeam()?.id ?? null}
              selectionMode={selectionMode}
              onSelectionMode={(enabled) => setSelectionMode(enabled)}
              onOpen={(team) => openEditor(team)}
              onCreate={(number) => openEditor(null, number)}
            />
            <View class="w-full flex-col gap-2">
              <View class="w-full flex-row items-center justify-between gap-2">
                <Text class="text-xs font-medium text-muted-foreground">
                  {`已配置队伍 · ${cards().length}`}
                </Text>
                <Text class="text-[10px] text-muted-foreground">
                  用途只是分类；资源本和通用队伍不参与镜牢轮换
                </Text>
              </View>
              <If
                when={cards().length > 0}
                else={
                  <EmptyState
                    title="该分类没有队伍"
                    detail="切换分类，或从预设新建一支队伍。"
                    class="min-h-36"
                  >
                    <Button
                      label="从预设新建"
                      icon="package"
                      variant="outline"
                      onPress={() => openPreset(null)}
                    />
                  </EmptyState>
                }
              >
                <View class="grid w-full grid-cols-2 items-stretch gap-3">
                  <Each of={cards()} key={(team) => team.id}>
                    {(team, index) => (
                      <TeamCard
                        team={team()}
                        index={index()}
                        onPreset={(selected) => openPreset(selected)}
                      />
                    )}
                  </Each>
                </View>
              </If>
            </View>
            <If when={extras().length > 0}>
              <Notice
                tone="warning"
                label="有队伍编号超出游戏编队位（1–40）；这些队伍可能无法被自动选队。"
              />
              <View class="grid w-full grid-cols-2 items-stretch gap-3">
                <Each of={extras()} key={(team) => team.id}>
                  {(team, index) => (
                    <TeamCard
                      team={team()}
                      index={index()}
                      onPreset={(selected) => openPreset(selected)}
                    />
                  )}
                </Each>
              </View>
            </If>
          </If>
        </View>
      </Scroll>
      <TeamEditor />
      <PresetPicker
        open={pickerOpen()}
        overwrite={pickerOverwrite()}
        onClose={() => {
          pickerOpen.set(false);
          pickerOverwrite.set(null);
        }}
      />
      <DeleteDialog />
    </PageRoot>
  );
}

/** The destructive confirmation for a saved team. */
function DeleteDialog() {
  return (
    <Dialog
      open={deleteTarget() !== null}
      onOpenChange={({ open }) => {
        if (!open) cancelDeleteTeam();
      }}
      closeOnEscape
      closeOnBackdropPress
      class="m-auto w-105 rounded-(--radius-lg) border border-border bg-popover p-4 shadow-lg"
      backdropClass="bg-scrim"
      role="alertdialog"
      accessibilityLabel="删除编队"
    >
      <If when={deleteTarget()}>
        {(team) => (
          <View class="flex-col gap-3">
            <Text class="text-sm font-semibold text-popover-foreground">
              {`删除编队 #${(team().id.replace("team-", "")).padStart(2, "0")}？`}
            </Text>
            <Text class="text-xs text-muted-foreground">
              {`将删除「${team().name}」及其编队统计和主题包权重；日常任务与镜牢轮换里的编号不会自动替换。`}
            </Text>
            <View class="flex-row justify-end gap-2">
              <Button label="取消" variant="ghost" onPress={() => cancelDeleteTeam()} />
              <Button label="删除" variant="danger" onPress={() => confirmDeleteTeam()} />
            </View>
          </View>
        )}
      </If>
    </Dialog>
  );
}
