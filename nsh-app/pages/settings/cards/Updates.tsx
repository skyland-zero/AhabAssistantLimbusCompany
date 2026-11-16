// Updates & Sources: the prerelease channel, the download mirror, the mirror's
// CDK (only while that mirror is chosen) and the check for updates.
//
// `checkUpdate` reports available versions; `installUpdate` downloads the
// signed GitHub assets and hands them to the native updater.  The mirror
// choice and its CDK remain for checks and manual downloads.
import { View } from "nsh";
import { Button } from "../../../components/Button";
import { Rule } from "../../../components/Card";
import { Input } from "../../../components/Input";
import { SettingLine, SettingsGroup } from "../../../components/Field";
import { Select } from "../../../components/Select";
import { Switch } from "../../../components/Switch";
import { UPDATE_SOURCE_LABELS } from "../../../model/types";
import type { UpdateSource } from "../../../model/types";
import {
  checkForUpdates,
  currentCdk,
  currentUpdateSource,
  installUpdate,
  saveCdk,
  setCdk,
  setUpdateSource,
  systemFlag,
  toggleSystemFlag,
} from "../../../settings";
import { SettingsSection } from "../SettingsSection";

/** The two mirrors, in the order the menu lists them. */
const SOURCES: { value: UpdateSource; label: string }[] = [
  { value: "github", label: UPDATE_SOURCE_LABELS.github },
  { value: "mirrorChyan", label: UPDATE_SOURCE_LABELS.mirrorChyan },
];

export function UpdatesCard() {
  return (
    <SettingsSection title="更新与源配置">
      <SettingsGroup title="版本渠道">
        <SettingLine label="参与预览版渠道">
          <Switch
            checked={systemFlag("prerelease")}
            accessibilityLabel="参与预览版渠道"
            onChange={() => toggleSystemFlag("prerelease")}
          />
        </SettingLine>
      </SettingsGroup>
      <SettingsGroup title="更新源">
        <SettingLine label="更新源选择">
          <Select<UpdateSource>
            accessibilityLabel="更新源选择"
            items={SOURCES}
            value={currentUpdateSource()}
            onChange={(next) => setUpdateSource(next)}
            class="w-44"
          />
        </SettingLine>
        <If when={currentUpdateSource() === "mirrorChyan"}>
          <Rule />
        </If>
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
              <Button label="保存 CDK" onPress={() => saveCdk()} />
            </View>
          </SettingLine>
        </If>
      </SettingsGroup>
      <View class="flex-row gap-2 pt-1">
        <Button
          label="检查更新"
          icon="check"
          variant="outline"
          onPress={() => checkForUpdates()}
        />
        <Button
          label="安装更新"
          icon="package"
          variant="default"
          onPress={() => installUpdate()}
        />
      </View>
    </SettingsSection>
  );
}
