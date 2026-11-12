// Updates & Sources: the prerelease channel, the download mirror, the mirror's
// CDK (only while that mirror is chosen) and the check for updates.
//
// `checkUpdate` needs the updater and the sidecar, so the button reports that
// the check is P1's work; the mirror choice and the CDK are editable now.
import { View } from "nsh";
import { Button } from "../../../components/Button";
import { Input } from "../../../components/Input";
import { SettingLine, SettingList } from "../../../components/Field";
import { Select } from "../../../components/Select";
import { Switch } from "../../../components/Switch";
import { UPDATE_SOURCE_LABELS } from "../../../model/types";
import type { UpdateSource } from "../../../model/types";
import {
  currentCdk,
  currentUpdateSource,
  setCdk,
  setFeedback,
  setUpdateSource,
  systemFlag,
  toggleSystemFlag,
} from "../../../settings";
import { SettingsCard } from "../SettingsCard";

/** The two mirrors, in the order the menu lists them. */
const SOURCES: { value: UpdateSource; label: string }[] = [
  { value: "github", label: UPDATE_SOURCE_LABELS.github },
  { value: "mirrorChyan", label: UPDATE_SOURCE_LABELS.mirrorChyan },
];

export function UpdatesCard() {
  return (
    <SettingsCard title="更新与源配置">
      <SettingList>
        <SettingLine label="参与预览版渠道">
          <Switch
            checked={systemFlag("prerelease")}
            accessibilityLabel="参与预览版渠道"
            onChange={() => toggleSystemFlag("prerelease")}
          />
        </SettingLine>
        <SettingLine label="更新源选择">
          <Select<UpdateSource>
            accessibilityLabel="更新源选择"
            items={SOURCES}
            value={currentUpdateSource()}
            onChange={(next) => {
              setUpdateSource(next);
              setFeedback(`更新源：${UPDATE_SOURCE_LABELS[next]}`);
            }}
            class="w-44"
          />
        </SettingLine>
        <If when={currentUpdateSource() === "mirrorChyan"}>
          <SettingLine label="Mirror 酱 CDK">
            <View class="flex-row items-center gap-2">
              <Input
                value={currentCdk()}
                placeholder="可选"
                accessibilityLabel="Mirror 酱 CDK"
                masked
                onChange={({ value }) => setCdk(value)}
                class="w-60"
              />
              <Button
                label="保存 CDK"
                onPress={() => setFeedback("CDK 已记录（保存随 P1 接入）")}
              />
            </View>
          </SettingLine>
        </If>
      </SettingList>
      <View class="flex-row pt-1">
        <Button
          label="检查更新"
          icon="check"
          variant="outline"
          onPress={() => setFeedback("更新检查随 P1 的 updater 接入")}
        />
      </View>
    </SettingsCard>
  );
}
