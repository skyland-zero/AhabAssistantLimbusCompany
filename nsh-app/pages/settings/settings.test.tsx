// The settings page's contract: every section is reachable, the quick
// navigation jumps to one, and a choice that changes the theme takes effect.
import { expect, open, test } from "nsh/test";

/** The window the desktop client runs in (`nsh.json` 820×680). */
const DESKTOP = { width: 820, height: 680, platform: "windows" as const };

/** Opens the app on the settings page. The page swap lands after `press`
 *  returns, so every test waits for the first card before touching it. */
async function openSettings() {
  const app = await open("../../App.tsx", DESKTOP);
  await app.press("role:button[设置]");
  await app.waitFor("外观");
  return app;
}

test("the settings page lists every section", async () => {
  const app = await openSettings();
  for (const title of [
    "外观",
    "全局热键",
    "系统与防护",
    "实验性功能",
    "更新与源配置",
    "资源同步",
    "任务通知",
    "关于",
  ]) {
    await app.waitFor(title);
  }
});

test("the quick navigation reaches a section and says what it edited", async () => {
  const app = await openSettings();
  await app.press("role:button[任务通知]");
  await app.waitFor("WxPusher SPT");
  await app.type("label:WxPusher SPT", "SPT_demo");
  await app.press("role:button[发送测试通知]");
  await app.waitFor("测试通知已发送");
});

test("choosing an interface skin applies it", async () => {
  const app = await openSettings();
  await app.press("role:button[边狱 / limbus]");
  await app.waitFor("界面皮肤：边狱");
});

test("the NSH skin is offered beside the migrated ones", async () => {
  const app = await openSettings();
  await app.press("role:button[NSH / nsh]");
  await app.waitFor("界面皮肤：NSH");
});

test("a colour theme applies from the appearance card", async () => {
  const app = await openSettings();
  await app.press("role:radio[粉]");
  await app.waitFor("颜色主题：粉");
});

test("a custom colour is refused until it is #rrggbb", async () => {
  const app = await openSettings();
  await app.type("label:颜色主题自定义色值", "pink");
  await app.press("role:button[应用]");
  await app.waitFor("颜色值要写成 #RRGGBB");
});

test("a hotkey row captures the next combination", async () => {
  const app = await openSettings();
  await app.press("id:hotkey-capture-startStop");
  await app.keyboard.press("ctrl+f9");
  await app.waitFor("Ctrl+F9");
  await app.waitFor("设置已保存");

  await app.press("id:hotkey-capture-startStop");
  await app.keyboard.press("escape");
  expect(app.texts()).toContain("Ctrl+F9");
});

test("the update mirror reveals its CDK field", async () => {
  const app = await openSettings();
  await app.press("label:更新源选择");
  await app.press("role:option[Mirror 酱]");
  await app.waitFor("Mirror 酱 CDK");
});
