// The resources page: a toolbar with the update check and the sync, then one
// card per resource group with its versions and last sync time.
import { Scroll, Text, View } from "nsh";
import { Badge } from "../../components/Badge";
import { Button } from "../../components/Button";
import { Card } from "../../components/Card";
import { EmptyState } from "../../components/EmptyState";
import { PageRoot, PageToolbar } from "../../components/Page";
import {
  checkResources,
  currentSyncProgress,
  hasUpdate,
  resourceGroups,
  resourcesFeedback,
  syncResources,
} from "../../resources";
import type { ResourceGroup } from "../../resources";

export function Resources() {
  return (
    <PageRoot>
      <PageToolbar>
        <View class="w-full flex-row items-center justify-between gap-3">
          <SyncStatus />
          <View class="flex-row items-center gap-1.5">
            <Button
              label="检查更新"
              icon="searchCheck"
              variant="outline"
              onPress={() => checkResources()}
            />
            <Button label="立即同步" icon="refresh" onPress={() => syncResources()} />
          </View>
        </View>
      </PageToolbar>
      <Scroll class="min-h-0 flex-1">
        <View class="w-full flex-col gap-3">
          <If when={resourceGroups().length > 0}>
            <View class="w-full flex-row flex-wrap gap-2">
              <Each of={resourceGroups()} key={(group) => group.id}>
                {(group) => <ResourceCard group={group()} />}
              </Each>
            </View>
          </If>
          <If when={resourceGroups().length === 0 && currentSyncProgress() === null}>
            <EmptyState title="暂无资源" detail="资源状态将在后端接入后显示。" />
          </If>
          <If when={resourcesFeedback()}>
            {(line) => (
              <Card class="w-full p-3">
                <Text class="text-xs text-success">{line()}</Text>
              </Card>
            )}
          </If>
        </View>
      </Scroll>
    </PageRoot>
  );
}

/** The toolbar's leading slot: the sync's progress while it runs, nothing otherwise. */
function SyncStatus() {
  return (
    <View class="flex-row items-center">
      <If when={currentSyncProgress() !== null}>
        <Badge label={`同步中… ${currentSyncProgress()}%`} tone="accent" />
      </If>
    </View>
  );
}

/** One resource group: its name and state, then local, remote and last sync. */
function ResourceCard(props: { group: ResourceGroup }) {
  const progress = () => currentSyncProgress();
  const status = () => {
    if (progress() !== null) return { label: `同步中… ${progress()}%`, tone: "accent" as const };
    if (hasUpdate(props.group))
      return { label: `可更新到 ${props.group.remoteVersion}`, tone: "accent" as const };
    return { label: "已是最新", tone: "success" as const };
  };
  return (
    <Card class="h-full min-w-95 flex-1 p-0">
      <View class="flex-col gap-3 px-5 py-4">
        <View class="w-full flex-row items-center justify-between gap-2">
          <Text class="min-w-0 text-sm text-foreground">{props.group.name}</Text>
          <Badge label={status().label} tone={status().tone} />
        </View>
        <VersionRow label="本地版本" value={props.group.localVersion} />
        <VersionRow label="远端版本" value={props.group.remoteVersion ?? "—"} />
        <VersionRow label="上次同步" value={formatSyncTime(props.group.lastSyncAt)} />
        <If when={progress() !== null}>
          <View class="h-1 w-full overflow-hidden rounded-full bg-border">
            <View
              class="h-full rounded-full bg-brand"
              style={{ width: `${Math.min(100, progress() ?? 0)}%` }}
            />
          </View>
        </If>
      </View>
    </Card>
  );
}

/** A label and its value, on one baseline. */
function VersionRow(props: { label: string; value: string }) {
  return (
    <View class="w-full flex-row items-center justify-between gap-3">
      <Text class="text-xs text-muted-foreground">{props.label}</Text>
      <Text class="font-mono text-xs text-foreground">{props.value}</Text>
    </View>
  );
}

/** When it last synced: the client reported the raw stamp, and "never" for none. */
export function formatSyncTime(timestamp: number | null): string {
  if (timestamp === null || timestamp === 0) return "从未同步";
  return `时间戳 ${timestamp}`;
}
