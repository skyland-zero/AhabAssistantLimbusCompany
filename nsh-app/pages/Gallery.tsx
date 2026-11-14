// The P2 component gallery: every `components/` control in every variant,
// size and state, so the design system is reviewed here before any feature
// uses it. It is not a user-facing page; `store.ts` names it as `gallery` and
// the development build opens it with `navigate("gallery")`.
import type { JSX } from "nsh";
import { Scroll, Text, View, token } from "nsh";
import { Badge } from "../components/Badge";
import { Button, IconButton } from "../components/Button";
import { Card, CardBody, CardHeader, Divider, Rule } from "../components/Card";
import { EmptyState } from "../components/EmptyState";
import { Icon } from "../components/Icon";
import type { IconName } from "../components/Icon";
import { PageRoot } from "../components/Page";
import {
  ACCENTS,
  ACCENT_LABELS,
  SKINS,
  SKIN_LABELS,
} from "../themes";
import { currentAccent, currentSkin, setAccent, setSkin } from "../store";

/** Every glyph the app ships, so a missing or mis-tinted SVG shows up here. */
const ICONS: IconName[] = [
  "check",
  "chevronDown",
  "close",
  "help",
  "home",
  "loader",
  "maximize",
  "minimize",
  "monitor",
  "moon",
  "package",
  "palette",
  "restore",
  "settings",
  "sun",
  "users",
  "wrench",
];

/** The surface, foreground and status roles, in both appearances. */
const SWATCHES = [
  "background",
  "foreground",
  "card",
  "card-foreground",
  "muted",
  "muted-foreground",
  "primary",
  "primary-foreground",
  "secondary",
  "secondary-foreground",
  "accent",
  "accent-foreground",
  "brand",
  "brand-foreground",
  "brand-light",
  "success",
  "success-light",
  "warning",
  "warning-light",
  "danger",
  "danger-light",
  "border",
  "input",
  "ring",
  "decor-line",
  "hilite",
  "glow",
] as const;

export function Gallery() {
  return (
    <PageRoot>
      <Scroll class="flex-1">
        <View class="flex-col gap-2">
          <ButtonSection />
          <BadgeSection />
          <IconSection />
          <SurfaceSection />
          <ThemeSection />
          <TokenSection />
        </View>
      </Scroll>
    </PageRoot>
  );
}

function Section(props: { title: string; children?: JSX.Element }) {
  return (
    <Card title={props.title}>
      <CardBody>{props.children}</CardBody>
    </Card>
  );
}

function Row(props: { label: string; children?: JSX.Element }) {
  return (
    <View class="flex-row items-center gap-3">
      <Text class="w-16 flex-none text-xs text-muted-foreground">{props.label}</Text>
      <View class="flex-row flex-wrap items-center gap-2">{props.children}</View>
    </View>
  );
}

function ButtonSection() {
  return (
    <Section title="按钮">
      <Row label="变体">
        <Button label="默认" />
        <Button label="主要" variant="primary" />
        <Button label="描边" variant="outline" />
        <Button label="幽灵" variant="ghost" />
        <Button label="危险" variant="danger" />
        <Button label="链接" variant="link" />
      </Row>
      <Row label="尺寸">
        <Button label="xs" size="xs" />
        <Button label="sm" size="sm" />
        <Button label="md" size="md" />
        <Button label="lg" size="lg" />
      </Row>
      <Row label="状态">
        <Button label="禁用" disabled />
        <Button label="选中" variant="ghost" selected />
        <Button label="图标" icon="check" variant="primary" />
        <Button label="带计数" icon="users" variant="outline">
          <Badge label="6" />
        </Button>
      </Row>
      <Row label="图标">
        <IconButton name="wrench" label="工具" />
        <IconButton name="palette" label="主题" variant="outline" />
        <IconButton name="close" label="关闭" variant="danger" />
        <IconButton name="settings" label="设置（禁用）" disabled />
      </Row>
    </Section>
  );
}

function BadgeSection() {
  return (
    <Section title="标记">
      <Row label="语义">
        <Badge label="中性" />
        <Badge label="强调" tone="accent" />
        <Badge label="成功" tone="success" />
        <Badge label="警告" tone="warning" />
        <Badge label="信息" tone="info" />
        <Badge label="危险" tone="danger" />
      </Row>
    </Section>
  );
}

function IconSection() {
  return (
    <Section title="图标">
      <View class="flex-row flex-wrap items-center gap-3">
        <Each of={ICONS}>
          {(name) => (
            <View class="flex-col items-center gap-1">
              <Icon name={name()} size={16} color={token("foreground")} />
              <Text class="text-xs text-muted-foreground">{name()}</Text>
            </View>
          )}
        </Each>
      </View>
      <Row label="尺寸">
        <Icon name="check" size={12} />
        <Icon name="check" size={14} />
        <Icon name="check" size={16} />
        <Icon name="check" size={24} />
      </Row>
      <Row label="颜色">
        <Icon name="check" color={token("foreground")} />
        <Icon name="check" color={token("brand")} />
        <Icon name="check" color={token("success")} />
        <Icon name="check" color={token("danger")} />
      </Row>
    </Section>
  );
}

function SurfaceSection() {
  return (
    <Section title="表面与空状态">
      <Row label="分隔线">
        <View class="w-full">
          <Rule />
        </View>
      </Row>
      <Row label="卡片头">
        <View class="w-full overflow-hidden rounded-(--radius-xl) border border-border">
          <CardHeader label="卡片标题" icon="chart" note="注脚" />
        </View>
      </Row>
      <Row label="空状态">
        <EmptyState class="w-full" title="还没有内容" detail="这是一个空状态的说明文字，最多两行。" />
      </Row>
    </Section>
  );
}

function ThemeSection() {
  return (
    <Section title="皮肤与强调色">
      <Row label="皮肤">
        <Each of={SKINS}>
          {(name) => (
            <Button
              label={SKIN_LABELS[name()]}
              variant={currentSkin() === name() ? "primary" : "outline"}
              selected={currentSkin() === name()}
              onPress={() => setSkin(name())}
            />
          )}
        </Each>
      </Row>
      <Row label="强调色">
        <Each of={ACCENTS}>
          {(name) => (
            <Button
              label={ACCENT_LABELS[name()]}
              variant={currentAccent() === name() ? "primary" : "outline"}
              selected={currentAccent() === name()}
              onPress={() => setAccent(name())}
            />
          )}
        </Each>
      </Row>
      <Row label="分隔符">
        <View class="w-full">
          <Divider />
        </View>
      </Row>
    </Section>
  );
}

function TokenSection() {
  return (
    <Section title="主题 token">
      <View class="flex-row flex-wrap gap-2">
        <Each of={SWATCHES}>
          {(name) => (
            <View class="flex-col gap-1">
              <View
                class="h-10 w-24 rounded-(--radius) border border-border"
                style={{ backgroundColor: token(name()) }}
              />
              <Text class="text-xs text-muted-foreground">{name()}</Text>
            </View>
          )}
        </Each>
      </View>
      <Row label="圆角">
        <View class="h-10 w-16 bg-muted rounded-(--radius-sm)" />
        <View class="h-10 w-16 bg-muted rounded-(--radius)" />
        <View class="h-10 w-16 bg-muted rounded-(--radius-lg)" />
        <View class="h-10 w-16 bg-muted rounded-(--radius-xl)" />
      </Row>
    </Section>
  );
}
