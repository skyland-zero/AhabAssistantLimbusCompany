// Task Notifications: the WxPusher SPT that receives task summaries.
//
// Saving and the test send are the sidecar's (`Settings.setWxpusherSpt`,
// `Notifications.test`), so both buttons report what they would do until P1
// wires them.
import { View } from "nsh";
import { Button } from "../../../components/Button";
import { Input } from "../../../components/Input";
import { SettingRow } from "../../../components/Field";
import {
  currentWxpusherSpt,
  setFeedback,
  setWxpusherSpt,
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
          <Button label="保存 SPT" onPress={() => setFeedback("SPT 已记录（保存随 P1 接入）")} />
          <Button
            label="发送测试通知"
            variant="outline"
            onPress={() => setFeedback("测试推送随 P1 的通知服务接入")}
          />
        </View>
      </SettingRow>
    </SettingsCard>
  );
}
