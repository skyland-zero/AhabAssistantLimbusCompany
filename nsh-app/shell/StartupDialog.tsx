// The automation-core startup failure dialog. The core is required for every
// screen, so the only actions are restarting the app or quitting it; neither
// Escape nor a backdrop press can leave the app in a half-working state.
import { Text, View } from "nsh";
import { Application } from "nsh/desktop";
import { Dialog } from "nsh/base";
import { Button } from "../components/Button";
import { startupFailure } from "../home";

export function StartupDialog() {
  return (
    <Dialog
      open={startupFailure()}
      testID="startup-dialog"
      closeOnEscape={false}
      closeOnBackdropPress={false}
      class="m-auto w-105 rounded-(--radius-lg) border border-border bg-popover p-4 shadow-lg"
      backdropClass="bg-scrim"
      role="alertdialog"
      accessibilityLabel="自动化核心初始化失败"
    >
      <View class="flex-col gap-3">
        <Text class="text-sm font-semibold text-popover-foreground">自动化核心初始化失败</Text>
        <Text class="text-xs text-muted-foreground">
          自动化核心未能完成初始化。已自动重试 2 次。请重新启动应用；若仍然失败，请查看运行日志。
        </Text>
        <View class="w-full flex-row items-center justify-end gap-2 pt-1">
          <Button
            label="关闭应用"
            variant="outline"
            testID="startup-close"
            onPress={() => Application.quit()}
          />
          <Button
            label="重新启动"
            testID="startup-restart"
            onPress={() => Application.restart()}
          />
        </View>
      </View>
    </Dialog>
  );
}
