// The appearance card: colour scheme, accent, language and interface skin.
//
// Every control here changes the running theme, so each one writes the store and
// the theme follows on the next frame. The skin grid is drawn from the theme
// table rather than from screenshots, so a miniature always matches the accent
// and appearance in effect.
import { Text, View, colorScheme, signal } from "nsh";
import { Button } from "../../../components/Button";
import { SettingLine, SettingList } from "../../../components/Field";
import { Input } from "../../../components/Input";
import { Segmented, SwatchRow } from "../../../components/Segmented";
import { Swatch } from "../../../components/Swatch";
import { currentLanguage, setFeedback, setLanguage } from "../../../settings";
import type { Language } from "../../../settings";
import {
  ACCENTS,
  ACCENT_LABELS,
  SEED_PRESETS,
  SKIN_DEFAULT_ACCENT,
  normaliseSeed,
  tokensFor,
} from "../../../themes";
import {
  applyAppearance,
  currentAccent,
  currentAppearance,
  currentSeed,
  currentSkin,
  setAccent,
  setThemeSeed,
} from "../../../store";
import type { Appearance } from "../../../store";
import { SkinPicker } from "./SkinPicker";
import { SettingsCard } from "../SettingsCard";

export function AppearanceCard() {
  return (
    <SettingsCard title="外观">
      <SettingList>
        <SettingLine label="主题模式">
          <Segmented<Appearance>
            accessibilityLabel="主题模式"
            value={currentAppearance}
            onChange={(next) => {
              applyAppearance(next);
              setFeedback(`主题模式：${APPEARANCE_LABELS[next]}`);
            }}
            options={[
              { value: "light", label: "浅色" },
              { value: "dark", label: "深色" },
              { value: "system", label: "跟随系统" },
            ]}
          />
        </SettingLine>
        <SettingLine label="强调色">
          <SwatchRow label="强调色">
            <Each of={ACCENTS}>
              {(accent) => (
                <Swatch
                  label={ACCENT_LABELS[accent()]}
                  color={tokensFor(currentSkin(), accent(), colorScheme())["brand"]}
                  selected={currentAccent() === accent()}
                  onPress={() => {
                    setAccent(accent());
                    setFeedback(`强调色：${ACCENT_LABELS[accent()]}`);
                  }}
                />
              )}
            </Each>
            <If when={signatureHint()}>
              <View class="ms-1 rounded-(--radius-sm) bg-warning-light px-2 py-1">
                <Text class="text-xs text-warning">边狱签名配色为黄铜</Text>
              </View>
            </If>
          </SwatchRow>
        </SettingLine>
        <SettingLine label="颜色主题">
          <View class="flex-col gap-1.5">
            <View class="flex-row flex-wrap items-center gap-2">
              <Each of={SEED_PRESETS}>
                {(preset) => (
                  <Swatch
                    label={preset().label}
                    color={preset().hex}
                    selected={currentSeed() === preset().hex}
                    onPress={() => {
                      setThemeSeed(preset().hex);
                      setFeedback(`颜色主题：${preset().label}`);
                    }}
                  />
                )}
              </Each>
              <If when={currentSeed()}>
                <Button
                  label="清除"
                  size="xs"
                  variant="ghost"
                  onPress={() => {
                    setThemeSeed(null);
                    setFeedback("颜色主题：随皮肤");
                  }}
                />
              </If>
            </View>
            <Text class="text-xs text-muted-foreground">
              主题色会覆盖强调色，决定品牌色与页面渐变；清除后回到皮肤自带的配色。
            </Text>
          </View>
        </SettingLine>
        <SettingLine label="自定义颜色">
          <View class="flex-row items-center gap-2">
            <Input
              value={customSeed()}
              placeholder="#RRGGBB"
              accessibilityLabel="颜色主题自定义色值"
              onChange={(event) => {
                customSeed.set(event.value);
                customSeedHint.set(null);
              }}
              class="w-28"
            />
            <Button label="应用" size="sm" variant="outline" onPress={applyCustomSeed} />
            <If when={customSeedHint()}>
              {(text) => <Text class="text-xs text-warning">{text()}</Text>}
            </If>
          </View>
        </SettingLine>
        <SettingLine label="语言 / Language">
          <Segmented<Language>
            accessibilityLabel="语言"
            value={currentLanguage}
            onChange={(next) => setLanguage(next)}
            options={[
              { value: "zhCn", label: "简体中文" },
              { value: "enUs", label: "English" },
            ]}
          />
        </SettingLine>
      </SettingList>

      <View class="flex-col gap-2">
        <Text class="text-xs font-medium text-foreground">界面皮肤</Text>
        <SkinPicker />
        <Text class="text-xs text-muted-foreground">
          「镜牢主题包」在导航栏单独管理，与界面皮肤无关。
        </Text>
      </View>
    </SettingsCard>
  );
}

/** The custom colour the user is typing, and why it was refused. */
const customSeed = signal("");
const customSeedHint = signal<string | null>(null);

function applyCustomSeed(): void {
  const next = normaliseSeed(customSeed());
  if (!next) {
    customSeedHint.set("颜色值要写成 #RRGGBB");
    return;
  }
  setThemeSeed(next);
  setFeedback(`颜色主题：${next}`);
}

const APPEARANCE_LABELS: Record<Appearance, string> = {
  light: "浅色",
  dark: "深色",
  system: "跟随系统",
};

/** The frame skins were designed around a brass rule; say so instead of letting
 *  a violet accent fight a gold frame. */
function signatureHint(): boolean {
  return currentSkin() === "limbus" && currentAccent() !== SKIN_DEFAULT_ACCENT.limbus;
}
