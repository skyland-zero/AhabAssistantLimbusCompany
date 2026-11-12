// About: the version and the repository.
//
// The repository opens outside the app (`openUrl`, granted for `https` in
// `nsh.json`), which is what the link role is for: it leaves the app for a
// resource, so it is not a command.
import { Text, openUrl } from "nsh";
import { Button } from "../../../components/Button";
import { SettingLine, SettingList } from "../../../components/Field";
import { Runtime } from "aalc/native";
import { setFeedback } from "../../../settings";
import { SettingsCard } from "../SettingsCard";

const REPO_URL = "https://github.com/KIYI671/AhabAssistantLimbusCompany";

export function AboutCard() {
  return (
    <SettingsCard title="关于">
      <SettingList>
        <SettingLine label="版本">
          <Text class="font-mono text-xs text-muted-foreground">v{Runtime.appVersion()}</Text>
        </SettingLine>
        <SettingLine label="开源地址">
          <Button
            label="GitHub"
            icon="package"
            variant="ghost"
            onPress={() => {
              openUrl(REPO_URL).catch(() => setFeedback("无法打开浏览器"));
            }}
          />
        </SettingLine>
      </SettingList>
    </SettingsCard>
  );
}
