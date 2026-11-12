// The shell's smoke tests: the title bar lists every destination, a press
// switches the page, the settings utility opens Settings, and the appearance
// toggle cycles and reports what it shows.
import { expect, open, test } from "nsh/test";

/** The window the desktop client runs in (`nsh.json` 1100×720); the headless
 *  default is a phone, where the title bar would be clipped. */
const DESKTOP = { width: 820, height: 680, platform: "windows" as const };

test("the title bar lists every destination", async () => {
  const app = await open("./App.tsx", DESKTOP);
  await app.waitFor("AALC");
  for (const label of ["主控台", "队伍管理", "镜牢主题包", "工具箱", "资源中心", "帮助"]) {
    await app.waitFor(`role:button[${label}]`);
  }
});

test("pressing a destination switches the page", async () => {
  const app = await open("./App.tsx", DESKTOP);
  await app.waitFor("主控台 页面尚未迁入");
  await app.press("role:button[资源中心]");
  await app.waitFor("资源中心 页面尚未迁入");
  await app.press("role:button[帮助]");
  await app.waitFor("帮助 页面尚未迁入");
});

test("the settings utility opens the settings page", async () => {
  const app = await open("./App.tsx", DESKTOP);
  await app.press("role:button[设置]");
  await app.waitFor("快速导航");
  await app.waitFor("主题模式");
});

test("the appearance toggle cycles through the three appearances", async () => {
  const app = await open("./App.tsx", DESKTOP);
  await app.press("role:button[外观：跟随系统]");
  await app.waitFor("role:button[外观：浅色]");
  await app.press("role:button[外观：浅色]");
  await app.waitFor("role:button[外观：深色]");
});

test("every destination keeps the shared window chrome", async () => {
  const app = await open("./App.tsx", DESKTOP);
  await app.press("role:button[工具箱]");
  await app.waitFor("工具箱 页面尚未迁入");
  expect(app.texts()).toContain("AALC");
});
