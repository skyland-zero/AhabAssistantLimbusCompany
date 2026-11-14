// The console's detail dialog: the full run log. The statistics moved to their
// own page (the console's 明细 buttons open it), so the log is what is left
// here.
import { Scroll, Text, View } from "nsh";
import { Dialog } from "nsh/base";
import { Badge } from "../../components/Badge";
import { Button } from "../../components/Button";
import { closeLogDetails, isLogDetailsOpen, logEntries } from "../../home";

const DIALOG_CLASS =
  "m-auto h-130 max-h-full w-160 max-w-full rounded-(--radius-lg) border border-border bg-popover p-0 shadow-lg";

/** The full log dialog: every buffered line, with level and time. */
export function LogDetailsDialog() {
  return (
    <Dialog
      open={isLogDetailsOpen()}
      onOpenChange={({ open }) => {
        if (!open) closeLogDetails();
      }}
      closeOnEscape
      closeOnBackdropPress
      class={DIALOG_CLASS}
      backdropClass="bg-scrim"
      accessibilityLabel="运行日志明细"
    >
      <View class="h-full min-h-0 flex-col">
        <View class="w-full flex-none flex-row items-center justify-between gap-3 border-b border-border px-5 py-3">
          <View class="flex-row items-center gap-2">
            <Text class="text-base font-semibold text-popover-foreground">运行日志明细</Text>
            <Badge label={String(logEntries().length)} />
          </View>
          <Button
            label="关闭"
            variant="ghost"
            size="xs"
            testID="log-details-close"
            onPress={() => closeLogDetails()}
          />
        </View>
        <Scroll class="min-h-0 flex-1">
          <View class="w-full flex-col gap-1 p-4">
            <If
              when={logEntries().length > 0}
              else={
                <Text class="py-8 text-center text-xs text-muted-foreground">暂无日志</Text>
              }
            >
              <Each of={logEntries()} key={(entry) => `${entry.ts}-${entry.message}`}>
                {(entry) => (
                  <View class="w-full flex-row items-start gap-2 py-0.5 font-mono">
                    <Text class="w-16 flex-none text-[10px] text-muted-foreground">
                      {logTime(entry().ts)}
                    </Text>
                    <Badge label={entry().level} />
                    <Text class="min-w-0 flex-1 text-xs text-foreground">
                      {entry().message}
                    </Text>
                  </View>
                )}
              </Each>
            </If>
          </View>
        </Scroll>
      </View>
    </Dialog>
  );
}

/** `HH:mm:ss`, or `--:--:--` for a line without a timestamp. */
function logTime(ts: number): string {
  if (!Number.isFinite(ts) || ts <= 0) return "--:--:--";
  const date = new Date(ts);
  return date.toLocaleTimeString([], { hour12: false });
}
