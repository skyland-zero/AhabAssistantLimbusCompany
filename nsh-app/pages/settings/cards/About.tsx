// About: the version and the repository.
//
// The repository opens outside the app (`openUrl`, granted for `https` in
// `nsh.json`), which is what the link role is for: it leaves the app for a
// resource, so it is not a command.
import { Text, openUrl } from "nsh";
import { Button } from "../../../components/Button";
import { Rule } from "../../../components/Card";
import { SettingLine, SettingsGroup } from "../../../components/Field";
import { Runtime } from "aalc/native";
import { setFeedback } from "../../../settings";
import { navigate } from "../../../store";
import { SettingsSection } from "../SettingsSection";

const REPO_URL = "https://github.com/KIYI671/AhabAssistantLimbusCompany";

export function AboutCard() {
  return (
    <SettingsSection title="关于">
      <SettingsGroup>
        <SettingLine label="版本">
          <Text class="font-mono text-xs text-muted-foreground">v{Runtime.appVersion()}</Text>
        </SettingLine>
        <Rule />
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
        <Rule />
        <SettingLine label="帮助文档">
          <Button
            label="查看帮助"
            icon="help"
            variant="ghost"
            onPress={() => navigate("help")}
          />
        </SettingLine>
      </SettingsGroup>
    </SettingsSection>
  );
}
