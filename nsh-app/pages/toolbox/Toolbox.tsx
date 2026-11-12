// The toolbox: four cards in a two-column grid, each with its state, copy and
// one action.
//
// The actions are requests to the sidecar; until P1 wires the RPC they run the
// store's stand-ins, which report what a real run would say.
import type { JSX } from "nsh";
import { Scroll, Text, View, token } from "nsh";
import { Badge } from "../../components/Badge";
import { Button } from "../../components/Button";
import { Card } from "../../components/Card";
import { Icon } from "../../components/Icon";
import { PageRoot } from "../../components/Page";
import {
  TOOLS,
  applyResolution,
  isToolRunning,
  resetResolution,
  screenshot,
  toggleTool,
  toolboxFeedback,
} from "../../toolbox";
import type { ToolId } from "../../toolbox";

/** What one tool is, as the page draws it. */
type Tool = (typeof TOOLS)[number];

export function Toolbox() {
  return (
    <PageRoot>
      <Scroll class="min-h-0 flex-1">
        <View class="w-full flex-col gap-3">
          <View class="grid w-full grid-cols-2 gap-4">
            <Each of={TOOLS} key={(tool) => tool.id}>
              {(tool) => <ToolCard tool={tool()} />}
            </Each>
          </View>
          <Note>
            <Text class="text-xs text-muted-foreground">工具请求通过 Python sidecar 执行</Text>
          </Note>
          <If when={toolboxFeedback()}>
            {(line) => (
              <Note>
                <Text class="text-xs text-success">{line()}</Text>
              </Note>
            )}
          </If>
        </View>
      </Scroll>
    </PageRoot>
  );
}

function Note(props: { children?: JSX.Element }) {
  return (
    <Card class="w-full p-3">
      <View>{props.children}</View>
    </Card>
  );
}

/** One tool: its icon and state, what it does, and the action that runs it. */
function ToolCard(props: { tool: Tool }) {
  const running = () => isToolRunning(props.tool.id);
  return (
    <Card class="w-full p-0">
      <View class="flex-col items-start gap-3 px-4 py-4">
        <View class="w-full flex-row items-center justify-between">
          <Icon name={props.tool.icon} size={20} color={token("brand")} />
          <ToolStatus id={props.tool.id} running={running()} />
        </View>
        <View class="flex-col gap-1">
          <Text class="text-sm text-foreground">{props.tool.title}</Text>
          <Text class="text-xs text-muted-foreground">{props.tool.description}</Text>
        </View>
        <ToolAction id={props.tool.id} running={running()} />
      </View>
    </Card>
  );
}

/** Idle / running / the tool's own label (`ADB`, `—`). */
function ToolStatus(props: { id: ToolId; running: boolean }) {
  return (
    <Badge
      label={toolBadgeLabel(props.id, props.running)}
      tone={props.running ? "success" : "neutral"}
      dot={props.running}
    />
  );
}

/** The card's action: one full-width Run/Stop, or the two resolution buttons. */
function ToolAction(props: { id: ToolId; running: boolean }) {
  // The resolution tool commits a device change, so it offers two explicit
  // commands instead of a toggle.
  if (props.id === "resolution") {
    return (
      <View class="w-full flex-row gap-2">
        <Button
          label="修改 1080P"
          testID="tool-action-resolution-set"
          icon="monitor"
          iconColor={token("brand-foreground")}
          class="min-h-8 flex-1"
          onPress={() => applyResolution()}
        />
        <Button
          label="还原默认"
          testID="tool-action-resolution-reset"
          icon="rotateCcw"
          variant="outline"
          class="min-h-8 flex-1"
          onPress={() => resetResolution()}
        />
      </View>
    );
  }
  // A screenshot is one-shot; the other two tools run until stopped.
  const oneShot = props.id === "screenshot";
  return (
    <Button
      label={props.running ? "停止" : "运行"}
      testID={`tool-action-${props.id}`}
      icon={oneShot ? "camera" : props.running ? "stop" : "play"}
      variant={oneShot || props.running ? "outline" : "default"}
      iconColor={
        oneShot
          ? token("foreground")
          : props.running
            ? token("danger")
            : token("primary-foreground")
      }
      class={`w-full min-h-8 ${props.running ? "text-danger" : ""}`}
      onPress={() => (oneShot ? screenshot() : toggleTool(props.id))}
    />
  );
}

/** The status chip's text: the running state, or what the card is. */
export function toolBadgeLabel(id: ToolId, running: boolean): string {
  if (running) return "运行中";
  if (id === "resolution") return "ADB";
  if (id === "screenshot") return "—";
  return "待机";
}
