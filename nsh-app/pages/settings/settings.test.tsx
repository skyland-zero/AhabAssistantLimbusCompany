// The settings page's contract: every section is reachable, the quick
// navigation jumps to one, and a choice that changes the theme takes effect.
import { open, test } from "nsh/test";

/** The window the desktop client runs in (`nsh.json` 820×680). */
const DESKTOP = { width: 820, height: 680, platform: "windows" as const };

test("the settings page lists every section", async () => {
  const app = await open("../../App.tsx", DESKTOP);
  await app.press("role:button[设置]");
  for (const title of [
    "外观",
    "全局热键",
    "系统与防护",
    "实验性功能",
    "更新与源配置",
    "任务通知",
    "关于",
  ]) {
    await app.waitFor(title);
  }
});

test("the quick navigation reaches a section and says what it edited", async () => {
  const app = await open("../../App.tsx", DESKTOP);
  await app.press("role:button[设置]");
  await app.press("role:button[任务通知]");
  await app.waitFor("WxPusher SPT");
  await app.press("role:button[发送测试通知]");
  await app.waitFor("测试推送随 P1 的通知服务接入");
});

test("choosing an interface skin applies it", async () => {
  const app = await open("../../App.tsx", DESKTOP);
  await app.press("role:button[设置]");
  await app.press("role:button[边狱 / limbus]");
  await app.waitFor("界面皮肤：边狱");
});

test("the update mirror reveals its CDK field", async () => {
  const app = await open("../../App.tsx", DESKTOP);
  await app.press("role:button[设置]");
  await app.press("label:更新源选择");
  await app.press("role:option[Mirror 酱]");
  await app.waitFor("Mirror 酱 CDK");
});
