// Task Notifications: the WxPusher SPT that receives task summaries.
//
// Saving and the test send go through the Rust settings store
// (`systemSettings.set` and `notification.test`).
import { View } from "nsh";
import { Button } from "../../../components/Button";
import { Input } from "../../../components/Input";
import { SettingRow } from "../../../components/Field";
import {
  currentWxpusherSpt,
  saveWxpusherSpt,
  setWxpusherSpt,
  testNotification,
} from "../../../settings";
import { SettingsCard } from "../SettingsCard";

export function NotificationsCard() {
  return (
    <SettingsCard title="任务通知">
      <SettingRow
        label="WxPusher SPT"
        detail="配置个人 SPT 后接收任务完成与失败摘要，不发送原始日志"
      >
        <View class="flex-row items-center gap-2">
          <Input
            value={currentWxpusherSpt()}
            placeholder="SPT"
            accessibilityLabel="WxPusher SPT"
            masked
            onChange={({ value }) => setWxpusherSpt(value)}
            class="w-56"
          />
          <Button label="保存 SPT" onPress={() => saveWxpusherSpt()} />
          <Button
            label="发送测试通知"
            variant="outline"
            onPress={() => testNotification()}
          />
        </View>
      </SettingRow>
    </SettingsCard>
  );
}
