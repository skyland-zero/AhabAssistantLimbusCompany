// Resource sync, folded into Settings: the two resource groups with their
// local/remote versions and last sync, the update check and the sync itself.
//
// It used to be its own destination in the title bar; it is maintenance work
// that runs beside "更新与源配置", not part of the console's flow.
import { Text, View } from "nsh";
import { Badge } from "../../../components/Badge";
import { Button } from "../../../components/Button";
import { Rule } from "../../../components/Card";
import { SettingLine, SettingList } from "../../../components/Field";
import {
  checkResources,
  currentSyncProgress,
  hasUpdate,
  resourceGroups,
  resourcesFeedback,
  syncResources,
} from "../../../resources";
import type { ResourceGroup } from "../../../resources";
import { SettingsCard } from "../SettingsCard";

export function ResourcesCard() {
  const progress = () => currentSyncProgress();
  return (
    <SettingsCard title="资源同步">
      <If when={resourceGroups().length > 0}>
        <View class="w-full flex-col gap-3">
          <Each of={resourceGroups()} key={(group) => group.id}>
            {(group, index) => <ResourceGroupRows group={group()} first={index() === 0} />}
          </Each>
        </View>
      </If>
      <If when={resourceGroups().length === 0 && progress() === null}>
        <Text class="text-xs text-muted-foreground">资源状态将在后端接入后显示。</Text>
      </If>
      <If when={progress() !== null}>
        <View class="h-1 w-full overflow-hidden rounded-full bg-border">
          <View
            class="h-full rounded-full bg-brand"
            style={{ width: `${Math.min(100, progress() ?? 0)}%` }}
          />
        </View>
      </If>
      <View class="w-full flex-row items-center gap-2 pt-1">
        <Button
          label="检查资源更新"
          icon="searchCheck"
          variant="outline"
          onPress={() => checkResources()}
        />
        <Button label="立即同步" icon="refresh" onPress={() => syncResources()} />
        <If when={resourcesFeedback()}>
          {(line) => (
            <Text class="min-w-0 flex-1 truncate text-xs text-success">{line()}</Text>
          )}
        </If>
      </View>
    </SettingsCard>
  );
}

/** One resource group: its name and state, then local, remote and last sync. */
function ResourceGroupRows(props: { group: ResourceGroup; first: boolean }) {
  const status = () => {
    const syncing = currentSyncProgress();
    if (syncing !== null) return { label: `同步中… ${syncing}%`, tone: "accent" as const };
    if (hasUpdate(props.group))
      return { label: `可更新到 ${props.group.remoteVersion}`, tone: "accent" as const };
    return { label: "已是最新", tone: "success" as const };
  };
  return (
    <View class="w-full flex-col gap-2">
      <If when={!props.first}>
        <Rule />
      </If>
      <View class="w-full flex-row items-center justify-between gap-2">
        <Text class="min-w-0 text-xs font-medium text-foreground">{props.group.name}</Text>
        <Badge label={status().label} tone={status().tone} />
      </View>
      <SettingList>
        <SettingLine label="本地版本">
          <Text class="font-mono text-xs text-muted-foreground">{props.group.localVersion}</Text>
        </SettingLine>
        <SettingLine label="远端版本">
          <Text class="font-mono text-xs text-muted-foreground">
            {props.group.remoteVersion ?? "—"}
          </Text>
        </SettingLine>
        <SettingLine label="上次同步">
          <Text class="font-mono text-xs text-muted-foreground">
            {formatSyncTime(props.group.lastSyncAt)}
          </Text>
        </SettingLine>
      </SettingList>
    </View>
  );
}

/** When it last synced: the client reported the raw stamp, and "never" for none. */
export function formatSyncTime(timestamp: number | null): string {
  if (timestamp === null || timestamp === 0) return "从未同步";
  return `时间戳 ${timestamp}`;
}
