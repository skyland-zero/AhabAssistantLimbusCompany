// The toolbox: four cards in a two-column grid, each with its state, copy and
// one action.
//
// The actions are requests to the sidecar; until P1 wires the RPC they run the
// store's stand-ins, which report what a real run would say.
import { Scroll, Text, View, token } from "nsh";
import { Badge } from "../../components/Badge";
import { Button } from "../../components/Button";
import { Card, CardBody } from "../../components/Card";
import { Notice } from "../../components/Notice";
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
        <View class="w-full flex-col gap-3 pb-2">
          <View class="grid w-full grid-cols-2 gap-4">
            <Each of={TOOLS} key={(tool) => tool.id}>
              {(tool, index) => <ToolCard tool={tool()} index={index()} />}
            </Each>
          </View>
          <Text class="px-1 text-xs text-muted-foreground">工具请求通过 Python sidecar 执行</Text>
          <If when={toolboxFeedback()}>
            {(line) => <Notice tone="success" label={line()} />}
          </If>
        </View>
      </Scroll>
    </PageRoot>
  );
}

/** One tool: its icon and state in the header band, what it does, and the
 *  action that runs it at the foot of the card. */
function ToolCard(props: { tool: Tool; index: number }) {
  const running = () => isToolRunning(props.tool.id);
  return (
    <Card
      title={props.tool.title}
      icon={props.tool.icon}
      class="w-full flex-col"
      arrive={props.index}
      actions={<ToolStatus id={props.tool.id} running={running()} />}
    >
      <CardBody class="flex-1 justify-between">
        <Text class="text-xs text-muted-foreground">{props.tool.description}</Text>
        <ToolAction id={props.tool.id} running={running()} />
      </CardBody>
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
